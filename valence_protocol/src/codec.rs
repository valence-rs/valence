use std::io::Write;

#[cfg(feature = "encryption")]
use aes::cipher::{AsyncStreamCipher, NewCipher};
use anyhow::{bail, ensure};
use bytes::{Buf, BufMut, BytesMut};

use crate::var_int::{VarInt, VarIntDecodeError};
use crate::{Decode, Encode, Packet, Result, MAX_PACKET_SIZE};

/// The AES block cipher with a 128 bit key, using the CFB-8 mode of
/// operation.
#[cfg(feature = "encryption")]
type Cipher = cfb8::Cfb8<aes::Aes128>;

#[derive(Default)]
pub struct PacketEncoder {
    buf: BytesMut,
    #[cfg(feature = "compression")]
    compress_buf: Vec<u8>,
    #[cfg(feature = "compression")]
    compression_threshold: Option<u32>,
    #[cfg(feature = "encryption")]
    cipher: Option<Cipher>,
}

impl PacketEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_packet<P>(&mut self, pkt: &P) -> Result<()>
    where
        P: Encode + Packet + ?Sized,
    {
        self.append_or_prepend_packet::<true>(pkt)
    }

    pub fn append_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes)
    }

    pub fn prepend_packet<P>(&mut self, pkt: &P) -> Result<()>
    where
        P: Encode + Packet + ?Sized,
    {
        self.append_or_prepend_packet::<false>(pkt)
    }

    fn append_or_prepend_packet<const APPEND: bool>(
        &mut self,
        pkt: &(impl Encode + Packet + ?Sized),
    ) -> Result<()> {
        let data_len = pkt.encoded_len();

        #[cfg(debug_assertions)]
        {
            use crate::byte_counter::ByteCounter;

            let mut counter = ByteCounter::new();
            pkt.encode(&mut counter)?;

            let actual = counter.0;

            assert_eq!(
                actual,
                data_len,
                "actual encoded size of {} packet differs from reported size (actual = {actual}, \
                 reported = {data_len})",
                pkt.packet_name()
            );
        }

        #[cfg(feature = "compression")]
        if let Some(threshold) = self.compression_threshold {
            use flate2::write::ZlibEncoder;
            use flate2::Compression;

            if data_len >= threshold as usize {
                let mut z = ZlibEncoder::new(&mut self.compress_buf, Compression::new(4));
                pkt.encode(&mut z)?;
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
                    pkt.encode(&mut writer)?;
                } else {
                    let mut slice = move_forward_by(
                        &mut self.buf,
                        VarInt(packet_len as i32).encoded_len() + packet_len,
                    );

                    VarInt(packet_len as i32).encode(&mut slice)?;
                    VarInt(0).encode(&mut slice)?;
                    pkt.encode(&mut slice)?;
                }
            }

            return Ok(());
        }

        let packet_len = data_len;

        ensure!(
            packet_len <= MAX_PACKET_SIZE as usize,
            "packet exceeds maximum length"
        );

        if APPEND {
            let mut writer = (&mut self.buf).writer();
            VarInt(packet_len as i32).encode(&mut writer)?;
            pkt.encode(&mut writer)?;
        } else {
            let mut slice = move_forward_by(
                &mut self.buf,
                VarInt(packet_len as i32).encoded_len() + packet_len,
            );

            VarInt(packet_len as i32).encode(&mut slice)?;
            pkt.encode(&mut slice)?;

            debug_assert!(
                slice.is_empty(),
                "actual size of {} packet differs from reported size (actual = {}, reported = {})",
                pkt.packet_name(),
                data_len - slice.len(),
                data_len,
            );
        }

        Ok(())
    }

    /// Takes all the packets written so far and encrypts them if encryption is
    /// enabled.
    pub fn take(&mut self) -> BytesMut {
        #[cfg(feature = "encryption")]
        if let Some(cipher) = &mut self.cipher {
            cipher.encrypt(&mut self.buf);
        }

        self.buf.split()
    }

    #[cfg(feature = "compression")]
    pub fn set_compression(&mut self, threshold: Option<u32>) {
        self.compression_threshold = threshold;
    }

    /// Encrypts all future packets **and any packets that have
    /// not been [taken] yet.**
    ///
    /// [taken]: Self::take
    #[cfg(feature = "encryption")]
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

pub fn write_packet<W, P>(mut writer: W, packet: &P) -> Result<()>
where
    W: Write,
    P: Encode + Packet + ?Sized,
{
    let packet_len = packet.encoded_len();

    ensure!(
        packet_len <= MAX_PACKET_SIZE as usize,
        "packet exceeds maximum length"
    );

    VarInt(packet_len as i32).encode(&mut writer)?;
    packet.encode(&mut writer)
}

#[cfg(feature = "compression")]
pub fn write_packet_compressed<W, P>(
    mut writer: W,
    threshold: u32,
    scratch: &mut Vec<u8>,
    packet: &P,
) -> Result<()>
where
    W: Write,
    P: Encode + Packet + ?Sized,
{
    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    let data_len = packet.encoded_len();

    if data_len > threshold as usize {
        scratch.clear();

        let mut z = ZlibEncoder::new(&mut *scratch, Compression::new(4));
        packet.encode(&mut z)?;
        drop(z);

        let packet_len = VarInt(data_len as i32).encoded_len() + scratch.len();

        ensure!(
            packet_len <= MAX_PACKET_SIZE as usize,
            "packet exceeds maximum length"
        );

        VarInt(packet_len as i32).encode(&mut writer)?;
        VarInt(data_len as i32).encode(&mut writer)?;
        writer.write_all(scratch)?;
    } else {
        let packet_len = VarInt(0).encoded_len() + data_len;

        ensure!(
            packet_len <= MAX_PACKET_SIZE as usize,
            "packet exceeds maximum length"
        );

        VarInt(packet_len as i32).encode(&mut writer)?;
        VarInt(0).encode(&mut writer)?; // 0 for no compression on this packet.
        packet.encode(&mut writer)?;
    }

    Ok(())
}

#[derive(Default)]
pub struct PacketDecoder {
    buf: BytesMut,
    cursor: usize,
    #[cfg(feature = "compression")]
    decompress_buf: Vec<u8>,
    #[cfg(feature = "compression")]
    compression_enabled: bool,
    #[cfg(feature = "encryption")]
    cipher: Option<Cipher>,
}

impl PacketDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_next_packet<'a, P>(&'a mut self) -> Result<Option<P>>
    where
        P: Decode<'a> + Packet,
    {
        self.buf.advance(self.cursor);
        self.cursor = 0;

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
            return Ok(None);
        }

        r = &r[..packet_len as usize];

        #[cfg(feature = "compression")]
        let packet = if self.compression_enabled {
            let data_len = VarInt::decode(&mut r)?.0;

            ensure!(
                (0..MAX_PACKET_SIZE).contains(&data_len),
                "decompressed packet length of {data_len} is out of bounds"
            );

            if data_len != 0 {
                use std::io::Read;

                use anyhow::Context;
                use flate2::bufread::ZlibDecoder;

                self.decompress_buf.clear();
                self.decompress_buf.reserve_exact(data_len as usize);
                let mut z = ZlibDecoder::new(r).take(data_len as u64);

                z.read_to_end(&mut self.decompress_buf)
                    .context("decompressing packet")?;

                r = &self.decompress_buf;
                P::decode(&mut r)?
            } else {
                P::decode(&mut r)?
            }
        } else {
            P::decode(&mut r)?
        };

        #[cfg(not(feature = "compression"))]
        let packet = P::decode(&mut r)?;

        ensure!(
            r.is_empty(),
            "packet contents were not read completely ({} bytes remain)",
            r.len()
        );

        let total_packet_len = VarInt(packet_len).encoded_len() + packet_len as usize;
        self.cursor = total_packet_len;

        Ok(Some(packet))
    }

    pub fn has_next_packet(&self) -> Result<bool> {
        let mut r = &self.buf[self.cursor..];

        match VarInt::decode_partial(&mut r) {
            Ok(packet_len) => {
                ensure!(
                    (0..=MAX_PACKET_SIZE).contains(&packet_len),
                    "packet length of {packet_len} is out of bounds"
                );

                Ok(r.len() >= packet_len as usize)
            }
            Err(VarIntDecodeError::Incomplete) => Ok(false),
            Err(VarIntDecodeError::TooLarge) => bail!("malformed packet length VarInt"),
        }
    }

    #[cfg(feature = "compression")]
    pub fn set_compression(&mut self, enabled: bool) {
        self.compression_enabled = enabled;
    }

    #[cfg(feature = "encryption")]
    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        assert!(self.cipher.is_none(), "encryption is already enabled");

        let mut cipher = Cipher::new(key.into(), key.into());
        // Don't forget to decrypt the data we already have.
        cipher.decrypt(&mut self.buf[self.cursor..]);
        self.cipher = Some(cipher);
    }

    pub fn queue_bytes(&mut self, mut bytes: BytesMut) {
        #![allow(unused_mut)]

        #[cfg(feature = "encryption")]
        if let Some(cipher) = &mut self.cipher {
            cipher.decrypt(&mut bytes);
        }

        self.buf.unsplit(bytes);
    }

    pub fn queue_slice(&mut self, bytes: &[u8]) {
        #[cfg(feature = "encryption")]
        let len = self.buf.len();

        self.buf.extend_from_slice(bytes);

        #[cfg(feature = "encryption")]
        if let Some(cipher) = &mut self.cipher {
            cipher.decrypt(&mut self.buf[len..]);
        }
    }

    pub fn queued_bytes(&self) -> &[u8] {
        self.buf.as_ref()
    }

    pub fn take_capacity(&mut self) -> BytesMut {
        self.buf.split_off(self.buf.len())
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_pos::BlockPos;
    use crate::entity_meta::PaintingKind;
    use crate::ident::Ident;
    use crate::item::{ItemKind, ItemStack};
    use crate::text::{Text, TextFormat};
    use crate::username::Username;
    use crate::var_long::VarLong;

    #[cfg(feature = "encryption")]
    const CRYPT_KEY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    #[derive(PartialEq, Debug, Encode, Decode, Packet)]
    #[packet_id = 42]
    struct TestPacket<'a> {
        a: bool,
        b: u8,
        c: i32,
        d: f32,
        e: f64,
        f: BlockPos,
        g: PaintingKind,
        h: Ident<&'a str>,
        i: Option<ItemStack>,
        j: Text,
        k: Username<&'a str>,
        l: VarInt,
        m: VarLong,
        n: &'a str,
        o: &'a [u8; 10],
        p: [u128; 3],
    }

    impl<'a> TestPacket<'a> {
        fn new(n: &'a str) -> Self {
            Self {
                a: true,
                b: 12,
                c: -999,
                d: 5.001,
                e: 1e10,
                f: BlockPos::new(1, 2, 3),
                g: PaintingKind::DonkeyKong,
                h: Ident::new("minecraft:whatever").unwrap(),
                i: Some(ItemStack::new(ItemKind::WoodenSword, 12, None)),
                j: "my ".into_text() + "fancy".italic() + " text",
                k: Username::new("00a").unwrap(),
                l: VarInt(123),
                m: VarLong(456),
                n,
                o: &[7; 10],
                p: [123456789; 3],
            }
        }

        fn check(&self, n: &'a str) {
            assert_eq!(self, &Self::new(n));
        }
    }

    #[test]
    fn packets_round_trip() {
        let mut buf = BytesMut::new();

        let mut enc = PacketEncoder::new();

        enc.append_packet(&TestPacket::new("first")).unwrap();
        #[cfg(feature = "compression")]
        enc.set_compression(Some(0));
        enc.append_packet(&TestPacket::new("second")).unwrap();
        buf.unsplit(enc.take());
        #[cfg(feature = "encryption")]
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
        #[cfg(feature = "compression")]
        dec.set_compression(true);
        dec.try_next_packet::<TestPacket>()
            .unwrap()
            .unwrap()
            .check("second");
        #[cfg(feature = "encryption")]
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
