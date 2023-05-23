#[cfg(feature = "encryption")]
use aes::cipher::generic_array::GenericArray;
#[cfg(feature = "encryption")]
use aes::cipher::{AsyncStreamCipher, BlockDecryptMut, KeyIvInit};
use anyhow::{bail, ensure};
use bytes::{Buf, BytesMut};

use crate::packet::var_int::{VarInt, VarIntDecodeError};
use crate::packet::{Packet, MAX_PACKET_SIZE};

/// The AES block cipher with a 128 bit key, using the CFB-8 mode of
/// operation.
#[cfg(feature = "encryption")]
type Cipher = cfb8::Decryptor<aes::Aes128>;

#[derive(Default)]
pub struct PacketDecoder {
    buf: BytesMut,
    #[cfg(feature = "compression")]
    decompress_buf: BytesMut,
    #[cfg(feature = "compression")]
    compression_threshold: Option<u32>,
    #[cfg(feature = "encryption")]
    cipher: Option<Cipher>,
}

impl PacketDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_next_packet(&mut self) -> anyhow::Result<Option<BytesMut>> {
        let mut r = &self.buf[..];

        let packet_len = match VarInt::decode_partial(&mut r) {
            Ok(len) => len,
            Err(VarIntDecodeError::Incomplete) => return Ok(None),
            Err(VarIntDecodeError::TooLarge) => bail!("malformed packet length VarInt"),
        };

        ensure!(
            (0..=MAX_PACKET_SIZE).contains(&packet_len),
            "packet length of {packet_len} is out of bounds"
        );

        if r.len() < packet_len as usize {
            // Not enough data arrived yet.
            return Ok(None);
        }

        let packet_len_len = VarInt(packet_len).written_size();

        #[cfg(feature = "compression")]
        if let Some(threshold) = self.compression_threshold {
            use std::io::Write;

            use bytes::BufMut;
            use flate2::write::ZlibDecoder;

            use crate::packet::Decode;

            r = &r[..packet_len as usize];

            let data_len = VarInt::decode(&mut r)?.0;

            ensure!(
                (0..MAX_PACKET_SIZE).contains(&data_len),
                "decompressed packet length of {data_len} is out of bounds"
            );

            // Is this packet compressed?
            if data_len > 0 {
                ensure!(
                    data_len as u32 > threshold,
                    "decompressed packet length of {data_len} is <= the compression threshold of \
                     {threshold}"
                );

                debug_assert!(self.decompress_buf.is_empty());

                self.decompress_buf.put_bytes(0, data_len as usize);

                // TODO: use libdeflater or zune-inflate?
                let mut z = ZlibDecoder::new(&mut self.decompress_buf[..]);

                z.write_all(r)?;

                ensure!(
                    z.finish()?.is_empty(),
                    "decompressed packet length is shorter than expected"
                );

                let total_packet_len = VarInt(packet_len).written_size() + packet_len as usize;

                self.buf.advance(total_packet_len);

                return Ok(Some(self.decompress_buf.split()));
            } else {
                debug_assert_eq!(data_len, 0);

                ensure!(
                    r.len() <= threshold as usize,
                    "uncompressed packet length of {} exceeds compression threshold of {}",
                    r.len(),
                    threshold
                );

                let remaining_len = r.len();

                self.buf.advance(packet_len_len + 1);
                return Ok(Some(self.buf.split_to(remaining_len)));
            }
        }

        self.buf.advance(packet_len_len);
        Ok(Some(self.buf.split_to(packet_len as usize)))
    }

    #[cfg(feature = "compression")]
    pub fn compression(&self) -> Option<u32> {
        self.compression_threshold
    }

    #[cfg(feature = "compression")]
    pub fn set_compression(&mut self, threshold: Option<u32>) {
        self.compression_threshold = threshold;
    }

    #[cfg(feature = "encryption")]
    pub fn enable_encryption(&mut self, key: &[u8; 16]) -> anyhow::Result<()> {
        assert!(self.cipher.is_none(), "encryption is already enabled");

        let mut cipher = Cipher::new_from_slices(key, key)?;

        // Don't forget to decrypt the data we already have.
        let gen_arr = GenericArray::from_mut_slice(self.buf.as_mut());
        cipher.decrypt_blocks_mut(&mut [*gen_arr]);

        self.cipher = Some(cipher);

        Ok(())
    }

    pub fn queue_bytes(&mut self, mut bytes: BytesMut) {
        #![allow(unused_mut)]

        #[cfg(feature = "encryption")]
        if let Some(cipher) = &mut self.cipher {
            let mut gen_arr = GenericArray::from_mut_slice(bytes.as_mut());
            cipher.decrypt_blocks_mut(&mut [*gen_arr]);
        }

        self.buf.unsplit(bytes);
    }

    pub fn queue_slice(&mut self, bytes: &[u8]) {
        #[cfg(feature = "encryption")]
        let len = self.buf.len();

        self.buf.extend_from_slice(bytes);

        #[cfg(feature = "encryption")]
        if let Some(cipher) = &mut self.cipher {
            let gen_arr = GenericArray::from_mut_slice(self.buf[len..].as_mut());
            cipher.decrypt_blocks_mut(&mut [*gen_arr]);
        }
    }

    pub fn take_capacity(&mut self) -> BytesMut {
        self.buf.split_off(self.buf.len())
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }
}

/// Decodes a (packet ID + data) packet frame. An error is returned if the input
/// is not read to the end.
pub fn decode_packet<'a, P: Packet<'a>>(mut bytes: &'a [u8]) -> anyhow::Result<P> {
    let pkt = P::decode_packet(&mut bytes)?;

    ensure!(
        bytes.is_empty(),
        "missed {} bytes while decoding {}",
        bytes.len(),
        pkt.packet_name()
    );

    Ok(pkt)
}
