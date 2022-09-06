//! Reading and writing packets.

use std::io::Read;
use std::time::Duration;

use aes::Aes128;
use anyhow::{bail, ensure, Context};
use cfb8::cipher::{AsyncStreamCipher, NewCipher};
use cfb8::Cfb8;
use flate2::bufread::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use log::{log_enabled, Level};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::time::timeout;

use super::packets::{DecodePacket, EncodePacket};
use crate::protocol::{Decode, Encode, VarInt, MAX_PACKET_SIZE};

pub struct Encoder<W> {
    write: W,
    buf: Vec<u8>,
    compress_buf: Vec<u8>,
    compression_threshold: Option<u32>,
    cipher: Option<Cipher>,
    timeout: Duration,
}

impl<W: AsyncWrite + Unpin> Encoder<W> {
    pub fn new(write: W, timeout: Duration) -> Self {
        Self {
            write,
            buf: Vec::new(),
            compress_buf: Vec::new(),
            compression_threshold: None,
            cipher: None,
            timeout,
        }
    }

    /// Queues a packet to be written to the writer.
    ///
    /// To write all queued packets, call [`Self::flush`].
    pub fn queue_packet(&mut self, packet: &(impl EncodePacket + ?Sized)) -> anyhow::Result<()> {
        let start_len = self.buf.len();

        packet.encode_packet(&mut self.buf)?;

        let data_len = self.buf.len() - start_len;

        ensure!(data_len <= i32::MAX as usize, "bad packet data length");

        if let Some(threshold) = self.compression_threshold {
            if data_len >= threshold as usize {
                let mut z = ZlibEncoder::new(&self.buf[start_len..], Compression::best());

                z.read_to_end(&mut self.compress_buf)?;

                let data_len_len = VarInt(data_len as i32).written_size();
                let packet_len = data_len_len + self.compress_buf.len();

                ensure!(packet_len <= MAX_PACKET_SIZE as usize, "bad packet length");

                self.buf.truncate(start_len);

                VarInt(packet_len as i32).encode(&mut self.buf)?;
                VarInt(data_len as i32).encode(&mut self.buf)?;
                self.buf.extend_from_slice(&self.compress_buf);
                self.compress_buf.clear();
            } else {
                let packet_len = VarInt(0).written_size() + data_len;

                ensure!(packet_len <= MAX_PACKET_SIZE as usize, "bad packet length");

                self.buf.truncate(start_len);

                VarInt(packet_len as i32).encode(&mut self.buf)?;
                VarInt(0).encode(&mut self.buf)?; // 0 for no compression.
                packet.encode_packet(&mut self.buf)?;
            }
        } else {
            let packet_len = data_len;

            ensure!(packet_len <= MAX_PACKET_SIZE as usize, "bad packet length");

            self.buf.truncate(start_len);

            VarInt(packet_len as i32).encode(&mut self.buf)?;
            packet.encode_packet(&mut self.buf)?;
        }

        Ok(())
    }

    /// Writes all queued packets to the writer.
    pub async fn flush(&mut self) -> anyhow::Result<()> {
        if !self.buf.is_empty() {
            if let Some(cipher) = &mut self.cipher {
                cipher.encrypt(&mut self.buf);
            }

            timeout(self.timeout, self.write.write_all(&self.buf)).await??;
            self.buf.clear();
        }

        Ok(())
    }

    /// Queue one packet and then flush the buffer.
    pub async fn write_packet(
        &mut self,
        packet: &(impl EncodePacket + ?Sized),
    ) -> anyhow::Result<()> {
        self.queue_packet(packet)?;
        self.flush().await
    }

    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        self.cipher = Some(NewCipher::new(key.into(), key.into()));
    }

    pub fn enable_compression(&mut self, threshold: u32) {
        self.compression_threshold = Some(threshold);
    }

    pub fn into_inner(self) -> W {
        self.write
    }
}

pub struct Decoder<R> {
    read: BufReader<R>,
    buf: Vec<u8>,
    decompress_buf: Vec<u8>,
    compression_threshold: Option<u32>,
    cipher: Option<Cipher>,
    timeout: Duration,
}

impl<R: AsyncRead + Unpin> Decoder<R> {
    pub fn new(read: R, timeout: Duration) -> Self {
        Self {
            read: BufReader::new(read),
            buf: Vec::new(),
            decompress_buf: Vec::new(),
            compression_threshold: None,
            cipher: None,
            timeout,
        }
    }

    pub async fn read_packet<P: DecodePacket>(&mut self) -> anyhow::Result<P> {
        timeout(self.timeout, self.read_packet_impl()).await?
    }

    async fn read_packet_impl<P: DecodePacket>(&mut self) -> anyhow::Result<P> {
        let packet_len = self
            .read_var_int_async()
            .await
            .context("reading packet length")?;

        ensure!(
            (0..=MAX_PACKET_SIZE).contains(&packet_len),
            "invalid packet length of {packet_len}."
        );

        self.buf.resize(packet_len as usize, 0);

        self.read
            .read_exact(&mut self.buf)
            .await
            .context("reading packet body")?;

        if let Some(cipher) = &mut self.cipher {
            cipher.decrypt(&mut self.buf);
        }

        let mut packet_contents = self.buf.as_slice();

        // Compression enabled?
        let packet = if self.compression_threshold.is_some() {
            // The length of the packet data once uncompressed (zero indicates no
            // compression).
            let data_len = VarInt::decode(&mut packet_contents)
                .context("reading data length (once decompressed)")?
                .0;

            ensure!(
                (0..=MAX_PACKET_SIZE).contains(&data_len),
                "invalid packet data length of {data_len}."
            );

            if data_len != 0 {
                let mut z = ZlibDecoder::new(&mut packet_contents);
                self.decompress_buf.resize(data_len as usize, 0);
                z.read_exact(&mut self.decompress_buf)
                    .context("decompressing packet body")?;

                let mut decompressed = self.decompress_buf.as_slice();
                ensure!(
                    decompressed.is_empty(),
                    "packet contents were not read completely"
                );
                let packet = P::decode_packet(&mut decompressed)
                    .context("decoding packet after decompressing")?;
                packet
            } else {
                P::decode_packet(&mut packet_contents).context("decoding packet")?
            }
        } else {
            P::decode_packet(&mut packet_contents).context("decoding packet")?
        };

        if !packet_contents.is_empty() {
            if log_enabled!(Level::Debug) {
                log::debug!("complete packet after partial decode: {packet:?}");
            }

            bail!(
                "packet contents were not decoded completely ({} bytes remaining)",
                packet_contents.len()
            );
        }

        Ok(packet)
    }

    async fn read_var_int_async(&mut self) -> anyhow::Result<i32> {
        let mut val = 0;
        for i in 0..VarInt::MAX_SIZE {
            let array = &mut [self.read.read_u8().await?];
            if let Some(cipher) = &mut self.cipher {
                cipher.decrypt(array);
            }
            let [byte] = *array;

            val |= (byte as i32 & 0b01111111) << (i * 7);
            if byte & 0b10000000 == 0 {
                return Ok(val);
            }
        }
        bail!("var int is too large")
    }

    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        self.cipher = Some(NewCipher::new(key.into(), key.into()));
    }

    pub fn enable_compression(&mut self, threshold: u32) {
        self.compression_threshold = Some(threshold);
    }

    pub fn packet_buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_inner(self) -> R {
        self.read.into_inner()
    }
}

/// The AES block cipher with a 128 bit key, using the CFB-8 mode of
/// operation.
type Cipher = Cfb8<Aes128>;

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::oneshot;

    use super::*;
    use crate::protocol::packets::test::TestPacket;

    #[tokio::test]
    async fn encode_decode() {
        encode_decode_impl().await
    }

    const CRYPT_KEY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    const TIMEOUT: Duration = Duration::from_secs(3);

    async fn encode_decode_impl() {
        let (tx, rx) = oneshot::channel();
        let t = tokio::spawn(listen(tx));

        let stream = TcpStream::connect(rx.await.unwrap()).await.unwrap();
        let mut encoder = Encoder::new(stream, TIMEOUT);

        send_test_packet(&mut encoder).await;
        encoder.enable_compression(10);
        send_test_packet(&mut encoder).await;
        encoder.enable_encryption(&CRYPT_KEY);
        send_test_packet(&mut encoder).await;
        send_test_packet(&mut encoder).await;
        send_test_packet(&mut encoder).await;

        t.await.unwrap()
    }

    async fn listen(local_addr: oneshot::Sender<SocketAddr>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

        local_addr.send(listener.local_addr().unwrap()).unwrap();

        let stream = listener.accept().await.unwrap().0;
        let mut decoder = Decoder::new(stream, TIMEOUT);

        recv_test_packet(&mut decoder).await;
        decoder.enable_compression(10);
        recv_test_packet(&mut decoder).await;
        decoder.enable_encryption(&CRYPT_KEY);
        recv_test_packet(&mut decoder).await;
        recv_test_packet(&mut decoder).await;
        recv_test_packet(&mut decoder).await;
    }

    async fn send_test_packet(w: &mut Encoder<TcpStream>) {
        w.write_packet(&TestPacket {
            first: "abcdefghijklmnopqrstuvwxyz".into(),
            second: vec![0x1234, 0xabcd],
            third: 0x1122334455667788,
        })
        .await
        .unwrap();
    }

    async fn recv_test_packet(r: &mut Decoder<TcpStream>) {
        let TestPacket {
            first,
            second,
            third,
        } = r.read_packet().await.unwrap();

        assert_eq!(&first, "abcdefghijklmnopqrstuvwxyz");
        assert_eq!(&second, &[0x1234, 0xabcd]);
        assert_eq!(third, 0x1122334455667788);
    }
}
