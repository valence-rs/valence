#[cfg(feature = "encryption")]
use aes::cipher::{generic_array::GenericArray, BlockDecryptMut, BlockSizeUser, KeyIvInit};
use anyhow::{bail, ensure, Context};
use bytes::{Buf, BytesMut};

use crate::var_int::{VarInt, VarIntDecodeError};
#[cfg(feature = "compression")]
use crate::CompressionThreshold;
use crate::{Decode, Packet, MAX_PACKET_SIZE};

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
    threshold: CompressionThreshold,
    #[cfg(feature = "encryption")]
    cipher: Option<Cipher>,
}

impl PacketDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_next_packet(&mut self) -> anyhow::Result<Option<PacketFrame>> {
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

        let mut data;

        #[cfg(feature = "compression")]
        if self.threshold.0 >= 0 {
            use std::io::Write;

            use bytes::BufMut;
            use flate2::write::ZlibDecoder;

            r = &r[..packet_len as usize];

            let data_len = VarInt::decode(&mut r)?.0;

            ensure!(
                (0..MAX_PACKET_SIZE).contains(&data_len),
                "decompressed packet length of {data_len} is out of bounds"
            );

            // Is this packet compressed?
            if data_len > 0 {
                ensure!(
                    data_len > self.threshold.0,
                    "decompressed packet length of {data_len} is <= the compression threshold of \
                     {}",
                    self.threshold.0
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

                data = self.decompress_buf.split();
            } else {
                debug_assert_eq!(data_len, 0);

                ensure!(
                    r.len() <= self.threshold.0 as usize,
                    "uncompressed packet length of {} exceeds compression threshold of {}",
                    r.len(),
                    self.threshold.0
                );

                let remaining_len = r.len();

                self.buf.advance(packet_len_len + 1);

                data = self.buf.split_to(remaining_len);
            }
        } else {
            self.buf.advance(packet_len_len);
            data = self.buf.split_to(packet_len as usize);
        }

        #[cfg(not(feature = "compression"))]
        {
            self.buf.advance(packet_len_len);
            data = self.buf.split_to(packet_len as usize);
        }

        // Decode the leading packet ID.
        r = &data[..];
        let packet_id = VarInt::decode(&mut r)
            .context("failed to decode packet ID")?
            .0;

        data.advance(data.len() - r.len());

        Ok(Some(PacketFrame {
            id: packet_id,
            body: data,
        }))
    }

    #[cfg(feature = "compression")]
    pub fn compression(&self) -> CompressionThreshold {
        self.threshold
    }

    #[cfg(feature = "compression")]
    pub fn set_compression(&mut self, threshold: CompressionThreshold) {
        self.threshold = threshold;
    }

    #[cfg(feature = "encryption")]
    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        assert!(self.cipher.is_none(), "encryption is already enabled");

        let mut cipher = Cipher::new_from_slices(key, key).expect("invalid key");

        // Don't forget to decrypt the data we already have.
        Self::decrypt_bytes(&mut cipher, &mut self.buf);

        self.cipher = Some(cipher);
    }

    /// Decrypts the provided byte slice in place using the cipher, without
    /// consuming the cipher.
    #[cfg(feature = "encryption")]
    fn decrypt_bytes(cipher: &mut Cipher, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(Cipher::block_size()) {
            let gen_arr = GenericArray::from_mut_slice(chunk);
            cipher.decrypt_block_mut(gen_arr);
        }
    }

    pub fn queue_bytes(&mut self, mut bytes: BytesMut) {
        #![allow(unused_mut)]

        #[cfg(feature = "encryption")]
        if let Some(cipher) = &mut self.cipher {
            Self::decrypt_bytes(cipher, &mut bytes);
        }

        self.buf.unsplit(bytes);
    }

    pub fn queue_slice(&mut self, bytes: &[u8]) {
        #[cfg(feature = "encryption")]
        let len = self.buf.len();

        self.buf.extend_from_slice(bytes);

        #[cfg(feature = "encryption")]
        if let Some(cipher) = &mut self.cipher {
            let slice = &mut self.buf[len..];
            Self::decrypt_bytes(cipher, slice);
        }
    }

    pub fn take_capacity(&mut self) -> BytesMut {
        self.buf.split_off(self.buf.len())
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }
}

#[derive(Clone, Debug)]
pub struct PacketFrame {
    /// The ID of the decoded packet.
    pub id: i32,
    /// The contents of the packet after the leading VarInt ID.
    pub body: BytesMut,
}

impl PacketFrame {
    /// Attempts to decode this packet as type `P`. An error is returned if the
    /// packet ID does not match, the body of the packet failed to decode, or
    /// some input was missed.
    pub fn decode<'a, P>(&'a self) -> anyhow::Result<P>
    where
        P: Packet + Decode<'a>,
    {
        ensure!(
            P::ID == self.id,
            "packet ID mismatch while decoding '{}': expected {}, got {}",
            P::NAME,
            P::ID,
            self.id
        );

        let mut r = &self.body[..];

        let pkt = P::decode(&mut r)?;

        ensure!(
            r.is_empty(),
            "missed {} bytes while decoding '{}'",
            r.len(),
            P::NAME
        );

        Ok(pkt)
    }
}
