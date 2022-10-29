use std::io::Read;

use aes::cipher::{AsyncStreamCipher, NewCipher};
use aes::Aes128;
use anyhow::{bail, ensure, Context};
use bytes::{Buf, BufMut, BytesMut};
use cfb8::Cfb8;
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use log::log_enabled;

use crate::protocol::packets::{DecodePacket, EncodePacket};
use crate::protocol::var_int::VarIntDecodeError;
use crate::protocol::{Decode, Encode, VarInt, MAX_PACKET_SIZE};

/// The AES block cipher with a 128 bit key, using the CFB-8 mode of
/// operation.
type Cipher = Cfb8<Aes128>;

pub struct PacketEncoder {
    buf: BytesMut,
    compress_buf: Vec<u8>,
    compression_threshold: Option<u32>,
    cipher: Option<Cipher>,
}

impl PacketEncoder {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::new(),
            compress_buf: Vec::new(),
            compression_threshold: None,
            cipher: None,
        }
    }

    pub fn append_packet(&mut self, pkt: &(impl EncodePacket + ?Sized)) -> anyhow::Result<()> {
        self.append_or_prepend_packet::<true>(pkt)
    }

    pub fn prepend_packet(&mut self, pkt: &(impl EncodePacket + ?Sized)) -> anyhow::Result<()> {
        self.append_or_prepend_packet::<false>(pkt)
    }

    fn append_or_prepend_packet<const APPEND: bool>(
        &mut self,
        pkt: &(impl EncodePacket + ?Sized),
    ) -> anyhow::Result<()> {
        let data_len = pkt.encoded_packet_len();

        if let Some(threshold) = self.compression_threshold {
            if data_len >= threshold as usize {
                let mut z = ZlibEncoder::new(&mut self.compress_buf, Compression::best());
                pkt.encode_packet(&mut z)?;
                drop(z);

                let packet_len = VarInt(data_len as i32).encoded_len() + self.compress_buf.len();

                ensure!(
                    packet_len <= MAX_PACKET_SIZE as usize,
                    "packet exceeds maximum length"
                );

                // BytesMut doesn't implement io::Write for some reason.
                let mut writer = (&mut self.buf).writer();

                if APPEND {
                    VarInt(packet_len as i32).encode(&mut writer)?;
                    VarInt(data_len as i32).encode(&mut writer)?;
                    self.buf.extend_from_slice(&self.compress_buf);
                } else {
                    let mut slice = move_forward_by(
                        &mut self.buf,
                        VarInt(packet_len as i32).encoded_len() + packet_len,
                    );

                    VarInt(packet_len as i32).encode(&mut slice)?;
                    VarInt(data_len as i32).encode(&mut slice)?;
                    slice.copy_from_slice(&self.compress_buf);
                }

                self.compress_buf.clear();
            } else {
                let packet_len = VarInt(0).encoded_len() + data_len;

                ensure!(
                    packet_len <= MAX_PACKET_SIZE as usize,
                    "packet exceeds maximum length"
                );

                let mut writer = (&mut self.buf).writer();

                if APPEND {
                    VarInt(packet_len as i32).encode(&mut writer)?;
                    VarInt(0).encode(&mut writer)?; // 0 for no compression on this packet.
                    pkt.encode_packet(&mut writer)?;
                } else {
                    let mut slice = move_forward_by(
                        &mut self.buf,
                        VarInt(packet_len as i32).encoded_len() + packet_len,
                    );

                    VarInt(packet_len as i32).encode(&mut slice)?;
                    VarInt(0).encode(&mut slice)?;
                    pkt.encode_packet(&mut slice)?;

                    debug_assert!(
                        slice.is_empty(),
                        "actual size of {} packet differs from reported size (actual = {}, \
                         reported = {})",
                        pkt.packet_name(),
                        data_len - slice.len(),
                        data_len,
                    );
                }
            }
        } else {
            let packet_len = data_len;

            ensure!(
                packet_len <= MAX_PACKET_SIZE as usize,
                "packet exceeds maximum length"
            );

            if APPEND {
                let mut writer = (&mut self.buf).writer();
                VarInt(packet_len as i32).encode(&mut writer)?;
                pkt.encode_packet(&mut writer)?;
            } else {
                let mut slice = move_forward_by(
                    &mut self.buf,
                    VarInt(packet_len as i32).encoded_len() + packet_len,
                );

                VarInt(packet_len as i32).encode(&mut slice)?;
                pkt.encode_packet(&mut slice)?;

                debug_assert!(
                    slice.is_empty(),
                    "actual size of {} packet differs from reported size (actual = {}, reported = \
                     {})",
                    pkt.packet_name(),
                    data_len - slice.len(),
                    data_len,
                );
            }
        }

        Ok(())
    }

    /// Takes all the packets written so far and encrypts them if encryption is
    /// enabled.
    pub fn take(&mut self) -> BytesMut {
        if let Some(cipher) = &mut self.cipher {
            cipher.encrypt(&mut self.buf);
        }

        self.buf.split()
    }

    pub fn set_compression(&mut self, threshold: Option<u32>) {
        self.compression_threshold = threshold;
    }

    /// Enables encryption for all future packets **and any packets that have
    /// not been [taken] yet.**
    ///
    /// [taken]: Self::take
    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        assert!(self.cipher.is_none(), "encryption is already enabled");
        self.cipher = Some(NewCipher::new(key.into(), key.into()));
    }
}

/// Move the bytes in `bytes` forward by `count` bytes and return a
/// mutable reference to the new space at the front.
fn move_forward_by(bytes: &mut BytesMut, count: usize) -> &mut [u8] {
    let len = bytes.len();
    bytes.put_bytes(0, count);
    bytes.copy_within(..len, count);
    &mut bytes[..count]
}

pub struct PacketDecoder {
    buf: BytesMut,
    decompress_buf: Vec<u8>,
    compression: bool,
    cipher: Option<Cipher>,
}

impl PacketDecoder {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::new(),
            decompress_buf: Vec::new(),
            compression: false,
            cipher: None,
        }
    }

    pub fn try_next_packet<P>(&mut self) -> anyhow::Result<Option<P>>
    where
        P: DecodePacket,
    {
        let mut r = &self.buf[..];

        let packet_len = match VarInt::decode_partial(&mut r) {
            Ok(len) => len,
            Err(VarIntDecodeError::Incomplete) => return Ok(None),
            Err(VarIntDecodeError::TooLarge) => bail!("malformed packet length VarInt"),
        };

        ensure!(
            packet_len <= MAX_PACKET_SIZE,
            "packet length of {packet_len} is out of bounds"
        );

        if r.len() < packet_len as usize {
            return Ok(None);
        }

        r = &r[..packet_len as usize];

        let packet = if self.compression {
            let data_len = VarInt::decode(&mut r)?.0;

            ensure!(
                (0..MAX_PACKET_SIZE).contains(&data_len),
                "decompressed packet length of {data_len} is out of bounds"
            );

            if data_len != 0 {
                self.decompress_buf.clear();
                self.decompress_buf.reserve_exact(data_len as usize);
                let mut z = ZlibDecoder::new(r).take(data_len as u64);

                z.read_to_end(&mut self.decompress_buf)
                    .context("decompressing packet")?;

                r = &self.decompress_buf;
                P::decode_packet(&mut r)?
            } else {
                P::decode_packet(&mut r)?
            }
        } else {
            P::decode_packet(&mut r)?
        };

        if !r.is_empty() {
            if log_enabled!(log::Level::Debug) {
                log::debug!("packet after partial decode: {packet:?}");
            }

            bail!(
                "packet contents were not read completely ({} bytes remain)",
                r.len()
            );
        }

        let total_packet_len = VarInt(packet_len).encoded_len() + packet_len as usize;

        self.buf.advance(total_packet_len);

        Ok(Some(packet))
    }

    pub fn set_compression(&mut self, compression: bool) {
        self.compression = compression;
    }

    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        assert!(self.cipher.is_none(), "encryption is already enabled");

        let mut cipher = Cipher::new(key.into(), key.into());
        // Don't forget to decrypt the data we already have.
        cipher.decrypt(&mut self.buf);
        self.cipher = Some(cipher);
    }

    pub fn queue_bytes(&mut self, mut bytes: BytesMut) {
        if let Some(cipher) = &mut self.cipher {
            cipher.decrypt(&mut bytes);
        }

        self.buf.unsplit(bytes);
    }

    pub fn take_capacity(&mut self) -> BytesMut {
        self.buf.split_to(self.buf.len())
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use anyhow::Context;

    use super::*;
    use crate::protocol::packets::{DecodePacket, EncodePacket, PacketName};
    use crate::protocol::{Decode, Encode};

    const CRYPT_KEY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TestPacket {
        string: String,
        vec_of_u16: Vec<u16>,
        u64: u64,
    }

    impl PacketName for TestPacket {
        fn packet_name(&self) -> &'static str {
            "TestPacket"
        }
    }

    impl EncodePacket for TestPacket {
        fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()> {
            self.string.encode(w)?;
            self.vec_of_u16.encode(w)?;
            self.u64.encode(w)
        }

        fn encoded_packet_len(&self) -> usize {
            self.string.encoded_len() + self.vec_of_u16.encoded_len() + self.u64.encoded_len()
        }
    }

    impl DecodePacket for TestPacket {
        fn decode_packet(r: &mut &[u8]) -> anyhow::Result<Self> {
            Ok(TestPacket {
                string: String::decode(r).context("decoding string field")?,
                vec_of_u16: Vec::decode(r).context("decoding vec of u16 field")?,
                u64: u64::decode(r).context("decoding u64 field")?,
            })
        }
    }

    impl TestPacket {
        fn new(s: impl Into<String>) -> Self {
            Self {
                string: s.into(),
                vec_of_u16: vec![0x1234, 0xabcd],
                u64: 0x1122334455667788,
            }
        }

        fn check(&self, s: impl AsRef<str>) {
            assert_eq!(&self.string, s.as_ref());
            assert_eq!(&self.vec_of_u16, &[0x1234, 0xabcd]);
            assert_eq!(self.u64, 0x1122334455667788);
        }
    }

    #[test]
    fn packets_round_trip() {
        let mut buf = BytesMut::new();

        let mut enc = PacketEncoder::new();

        enc.append_packet(&TestPacket::new("first")).unwrap();
        enc.set_compression(Some(0));
        enc.append_packet(&TestPacket::new("second")).unwrap();
        buf.unsplit(enc.take());
        enc.enable_encryption(&CRYPT_KEY);
        enc.append_packet(&TestPacket::new("third")).unwrap();
        enc.prepend_packet(&TestPacket::new("fourth")).unwrap();
        buf.unsplit(enc.take());

        let mut dec = PacketDecoder::new();

        dec.queue_bytes(buf);
        dec.try_next_packet::<TestPacket>()
            .unwrap()
            .unwrap()
            .check("first");
        dec.set_compression(true);
        dec.try_next_packet::<TestPacket>()
            .unwrap()
            .unwrap()
            .check("second");
        dec.enable_encryption(&CRYPT_KEY);
        dec.try_next_packet::<TestPacket>()
            .unwrap()
            .unwrap()
            .check("fourth");
        dec.try_next_packet::<TestPacket>()
            .unwrap()
            .unwrap()
            .check("third");
    }
}
