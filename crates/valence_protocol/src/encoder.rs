use anyhow::ensure;
use bytes::{BufMut, BytesMut};

use crate::var_int::VarInt;
use crate::{Encode, Packet, MAX_PACKET_SIZE};

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

    pub fn prepend_packet<'a, P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: Packet<'a>,
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

    pub fn append_packet<'a, P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: Packet<'a>,
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
            use aes::cipher::AsyncStreamCipher;

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
        use aes::cipher::NewCipher;

        assert!(self.cipher.is_none(), "encryption is already enabled");
        self.cipher = Some(NewCipher::new(key.into(), key.into()));
    }
}

pub fn encode_packet<'a, P>(buf: &mut Vec<u8>, pkt: &P) -> anyhow::Result<()>
where
    P: Packet<'a>,
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
pub fn encode_packet_compressed<'a, P>(
    buf: &mut Vec<u8>,
    pkt: &P,
    threshold: u32,
    scratch: &mut Vec<u8>,
) -> anyhow::Result<()>
where
    P: Packet<'a>,
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
