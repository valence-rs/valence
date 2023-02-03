use std::ops::{Deref, DerefMut};

#[cfg(feature = "encryption")]
use aes::cipher::{AsyncStreamCipher, NewCipher};
use anyhow::{bail, ensure};
use bytes::{Buf, BufMut, BytesMut};
use tracing::debug;

use crate::var_int::{VarInt, VarIntDecodeError};
use crate::{DecodePacket, Encode, EncodePacket, Result, MAX_PACKET_SIZE};

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

    #[inline]
    pub fn append_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes)
    }

    pub fn prepend_packet<P>(&mut self, pkt: &P) -> Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        let start_len = self.buf.len();
        self.append_packet(pkt)?;

        let end_len = self.buf.len();
        let total_packet_len = end_len - start_len;

        // 1) Move everything back by the length of the packet.
        // 2) Move the packet to the new space at the front.
        // 3) Truncate the old packet away.
        self.buf.put_bytes(0, total_packet_len);
        self.buf.copy_within(..end_len, total_packet_len);
        self.buf.copy_within(total_packet_len + start_len.., 0);
        self.buf.truncate(end_len);

        Ok(())
    }

    pub fn append_packet<P>(&mut self, pkt: &P) -> Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        let start_len = self.buf.len();

        pkt.encode_packet((&mut self.buf).writer())?;

        let data_len = self.buf.len() - start_len;

        #[cfg(feature = "compression")]
        if let Some(threshold) = self.compression_threshold {
            use std::io::Read;

            use flate2::bufread::ZlibEncoder;
            use flate2::Compression;

            if data_len > threshold as usize {
                let mut z = ZlibEncoder::new(&self.buf[start_len..], Compression::new(4));

                self.compress_buf.clear();

                let data_len_size = VarInt(data_len as i32).written_size();

                let packet_len = data_len_size + z.read_to_end(&mut self.compress_buf)?;

                ensure!(
                    packet_len <= MAX_PACKET_SIZE as usize,
                    "packet exceeds maximum length"
                );

                drop(z);

                self.buf.truncate(start_len);

                let mut writer = (&mut self.buf).writer();

                VarInt(packet_len as i32).encode(&mut writer)?;
                VarInt(data_len as i32).encode(&mut writer)?;
                self.buf.extend_from_slice(&self.compress_buf);
            } else {
                let data_len_size = 1;
                let packet_len = data_len_size + data_len;

                ensure!(
                    packet_len <= MAX_PACKET_SIZE as usize,
                    "packet exceeds maximum length"
                );

                let packet_len_size = VarInt(packet_len as i32).written_size();

                let data_prefix_len = packet_len_size + data_len_size;

                self.buf.put_bytes(0, data_prefix_len);
                self.buf
                    .copy_within(start_len..start_len + data_len, start_len + data_prefix_len);

                let mut front = &mut self.buf[start_len..];

                VarInt(packet_len as i32).encode(&mut front)?;
                // Zero for no compression on this packet.
                VarInt(0).encode(front)?;
            }

            return Ok(());
        }

        let packet_len = data_len;

        ensure!(
            packet_len <= MAX_PACKET_SIZE as usize,
            "packet exceeds maximum length"
        );

        let packet_len_size = VarInt(packet_len as i32).written_size();

        self.buf.put_bytes(0, packet_len_size);
        self.buf
            .copy_within(start_len..start_len + data_len, start_len + packet_len_size);

        let front = &mut self.buf[start_len..];
        VarInt(packet_len as i32).encode(front)?;

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

    pub fn clear(&mut self) {
        self.buf.clear();
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

pub fn encode_packet<P>(buf: &mut Vec<u8>, pkt: &P) -> Result<()>
where
    P: EncodePacket + ?Sized,
{
    let start_len = buf.len();

    pkt.encode_packet(&mut *buf)?;

    let packet_len = buf.len() - start_len;

    ensure!(
        packet_len <= MAX_PACKET_SIZE as usize,
        "packet exceeds maximum length"
    );

    let packet_len_size = VarInt(packet_len as i32).written_size();

    buf.put_bytes(0, packet_len_size);
    buf.copy_within(
        start_len..start_len + packet_len,
        start_len + packet_len_size,
    );

    let front = &mut buf[start_len..];
    VarInt(packet_len as i32).encode(front)?;

    Ok(())
}

#[cfg(feature = "compression")]
pub fn encode_packet_compressed<P>(
    buf: &mut Vec<u8>,
    pkt: &P,
    threshold: u32,
    scratch: &mut Vec<u8>,
) -> Result<()>
where
    P: EncodePacket + ?Sized,
{
    use std::io::Read;

    use flate2::bufread::ZlibEncoder;
    use flate2::Compression;

    let start_len = buf.len();

    pkt.encode_packet(&mut *buf)?;

    let data_len = buf.len() - start_len;

    if data_len > threshold as usize {
        let mut z = ZlibEncoder::new(&buf[start_len..], Compression::new(4));

        scratch.clear();

        let data_len_size = VarInt(data_len as i32).written_size();

        let packet_len = data_len_size + z.read_to_end(scratch)?;

        ensure!(
            packet_len <= MAX_PACKET_SIZE as usize,
            "packet exceeds maximum length"
        );

        drop(z);

        buf.truncate(start_len);

        VarInt(packet_len as i32).encode(&mut *buf)?;
        VarInt(data_len as i32).encode(&mut *buf)?;
        buf.extend_from_slice(scratch);
    } else {
        let data_len_size = 1;
        let packet_len = data_len_size + data_len;

        ensure!(
            packet_len <= MAX_PACKET_SIZE as usize,
            "packet exceeds maximum length"
        );

        let packet_len_size = VarInt(packet_len as i32).written_size();

        let data_prefix_len = packet_len_size + data_len_size;

        buf.put_bytes(0, data_prefix_len);
        buf.copy_within(start_len..start_len + data_len, start_len + data_prefix_len);

        let mut front = &mut buf[start_len..];

        VarInt(packet_len as i32).encode(&mut front)?;
        // Zero for no compression on this packet.
        VarInt(0).encode(front)?;
    }

    Ok(())
}

#[derive(Default)]
struct DecodeBuffer(BytesMut);

impl DecodeBuffer {
    /// Return the data of a minecraft packet and its total size in bytes (including its length field).
    /// When the packet is incomplete, return `Ok(None)`
    ///
    /// `offset` is the number of bytes to skip from the beginning of the buffer to get to the packet first byte
    #[inline]
    pub fn get_packet_data(&self, offset: usize) -> Result<Option<(&[u8], usize)>> {
        let mut r = &self[offset..];

        let packet_len = match VarInt::decode_partial(&mut r) {
            Ok(len) => len,
            Err(VarIntDecodeError::Incomplete) => return Ok(None),
            Err(VarIntDecodeError::TooLarge) => bail!("malformed packet length VarInt"),
        };

        // the packet data is not yet entirely inside the buffer
        if packet_len as usize > r.len() {
            return Ok(None);
        }

        ensure!(
            (0..=MAX_PACKET_SIZE).contains(&packet_len),
            "packet length of {packet_len} is out of bounds"
        );

        let total_packet_len = VarInt(packet_len).written_size() + packet_len as usize;
        Ok(Some((&r[..packet_len as usize], total_packet_len)))
    }
}

impl Deref for DecodeBuffer {
    type Target = BytesMut;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DecodeBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "compression")]
#[derive(Default)]
struct DecompressionBuffer(Vec<u8>);

#[cfg(feature = "compression")]
impl DecompressionBuffer {
    /// Return the decompressed data for a minecraft packet.
    /// `packet_data` should be formated as:
    /// `uncompressed_length`(`VarInt`) followed by `compressed_data`
    ///
    /// Detailed info: https://wiki.vg/Protocol#With_compression
    #[inline]
    fn decompress<'a>(&'a mut self, packet_data: &'a [u8]) -> Result<&'a [u8]> {
        use crate::Decode;
        use anyhow::Context;
        use flate2::bufread::ZlibDecoder;
        use std::io::Read;

        let packet_data = &mut &packet_data[..];
        let buf = &mut self.0;

        let uncompress_len = VarInt::decode(packet_data)?.0;

        ensure!(
            (0..MAX_PACKET_SIZE).contains(&uncompress_len),
            "decompressed packet length of {uncompress_len} is out of bounds"
        );

        if uncompress_len != 0 {
            buf.clear();
            buf.reserve_exact(uncompress_len as usize);
            let mut z = ZlibDecoder::new(*packet_data).take(uncompress_len as u64);

            z.read_to_end(buf).context("decompressing packet")?;

            return Ok(&buf[..]);
        }

        Ok(packet_data)
    }
}

#[derive(Default)]
pub struct PacketDecoder {
    buf: DecodeBuffer,
    cursor: usize,
    #[cfg(feature = "compression")]
    decompress_buf: DecompressionBuffer,
    #[cfg(feature = "compression")]
    compression_enabled: bool,
    #[cfg(feature = "encryption")]
    cipher: Option<Cipher>,
}

impl PacketDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to decode the next packet available.
    /// If a packet is succesfully decoded, update the cursor to point to the next packet position.
    /// If the next packet in the buffer is incomplete, return `Ok(None)`.
    pub fn try_next_packet<'a, P>(&'a mut self) -> Result<Option<P>>
    where
        P: DecodePacket<'a>,
    {
        self.buf.advance(self.cursor);
        self.cursor = 0;

        let Some(( mut packet_data, total_packet_len )) = self.buf.get_packet_data(self.cursor)? else {
            return Ok(None);
        };

        #[cfg(feature = "compression")]
        if self.compression_enabled {
            packet_data = self.decompress_buf.decompress(packet_data)?;
        }

        let packet = decode_packet(packet_data)?;
        self.cursor = total_packet_len;

        Ok(Some(packet))
    }

    /// Repeatedly decodes a packet type until all packets in the decoder are
    /// consumed or an error occurs. The decoded packets are returned in a vec.
    ///
    /// Intended for testing purposes with encryption and compression disabled.
    #[track_caller]
    pub fn collect_into_vec<'a, P>(&'a mut self) -> Result<Vec<P>>
    where
        P: DecodePacket<'a>,
    {
        #[cfg(feature = "encryption")]
        assert!(
            self.cipher.is_none(),
            "encryption must be disabled to use this method"
        );

        #[cfg(feature = "compression")]
        assert!(
            !self.compression_enabled,
            "compression must be disabled to use this method"
        );

        self.buf.advance(self.cursor);
        self.cursor = 0;

        let mut res = vec![];

        loop {
            let Some(( packet_data, total_packet_len )) = self.buf.get_packet_data(self.cursor)? else {
                return Ok(res);
            };
            let packet: P = decode_packet(packet_data)?;

            self.cursor += total_packet_len;
            res.push(packet);
        }
    }

    /// Check if there a complete packet available inside the buffer
    #[inline]
    pub fn has_next_packet(&self) -> Result<bool> {
        Ok(self.buf.get_packet_data(self.cursor)?.is_some())
    }

    pub fn queued_bytes(&self) -> &[u8] {
        self.buf.as_ref()
    }

    pub fn take_capacity(&mut self) -> BytesMut {
        let len = self.buf.len();
        self.buf.split_off(len)
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
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
}

#[inline]
fn decode_packet<'a, P>(packet_data: &'a [u8]) -> Result<P>
where
    P: DecodePacket<'a>,
{
    let packet_data = &mut &packet_data[..];
    let packet = P::decode_packet(packet_data)?;

    if !packet_data.is_empty() {
        let remaining = packet_data.len();

        debug!("packet after partial decode ({remaining} bytes remain): {packet:?}");

        bail!("packet contents were not read completely ({remaining} bytes remain)");
    }

    Ok(packet)
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
    use crate::Decode;

    #[cfg(feature = "encryption")]
    const CRYPT_KEY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    #[derive(PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
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

    #[test]
    fn collect_packets_into_vec() {
        let packets = vec![
            TestPacket::new("foo"),
            TestPacket::new("bar"),
            TestPacket::new("baz"),
        ];

        let mut enc = PacketEncoder::new();
        let mut dec = PacketDecoder::new();

        for pkt in &packets {
            enc.append_packet(pkt).unwrap();
        }

        dec.queue_bytes(enc.take());
        let res = dec.collect_into_vec::<TestPacket>().unwrap();

        assert_eq!(packets, res);
    }
}
