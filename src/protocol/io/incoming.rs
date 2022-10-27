use std::io::Read;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use aes::cipher::{AsyncStreamCipher, NewCipher};
use anyhow::{bail, ensure, Context};
use bytes::{Buf, BytesMut};
use flate2::bufread::ZlibDecoder;
use flume::{Receiver, Sender, TryRecvError};
use log::log_enabled;
use thiserror::Error;
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::protocol::io::{Cipher, PendingBytes};
use crate::protocol::packets::DecodePacket;
use crate::protocol::var_int::VarIntDecodeError;
use crate::protocol::{Decode, Encode, VarInt, MAX_PACKET_SIZE};

pub fn incoming(limit: usize) -> (IncomingPacketSender, IncomingPacketReceiver) {
    let pending_bytes = Arc::new(PendingBytes::new(limit));

    let (send, recv) = flume::unbounded();

    let sender = IncomingPacketSender {
        send,
        buf: BytesMut::new(),
        pending_bytes: pending_bytes.clone(),
    };

    let receiver = IncomingPacketReceiver {
        recv,
        buf: BytesMut::new(),
        pending_bytes,
        decompress_buf: vec![],
        compression: false,
        cipher: None,
    };

    (sender, receiver)
}

pub struct IncomingPacketSender {
    send: Sender<BytesMut>,
    buf: BytesMut,
    pending_bytes: Arc<PendingBytes>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ReadLoop {
    Eof,
    Disconnected,
    ReachedLimit,
}

impl IncomingPacketSender {
    pub async fn read_loop<R>(&mut self, mut reader: R) -> io::Result<ReadLoop>
    where
        R: AsyncRead + Unpin,
    {
        loop {
            // Make sure we have plenty of space to read to.
            self.buf.reserve(4096);

            let bytes_read = reader.read_buf(&mut self.buf).await?;

            if bytes_read == 0 {
                return Ok(ReadLoop::Eof);
            }

            if self
                .pending_bytes
                .current
                .fetch_add(bytes_read, Ordering::SeqCst)
                + bytes_read
                > self.pending_bytes.limit
            {
                return Ok(ReadLoop::ReachedLimit);
            }

            if let Err(e) = self.send.send_async(self.buf.split()).await {
                self.buf.unsplit(e.into_inner());
                return Ok(ReadLoop::Disconnected);
            }
        }
    }
}

pub struct IncomingPacketReceiver {
    recv: Receiver<BytesMut>,
    /// Contains decrypted packet data from the receiver. May contain partially
    /// read packets.
    ///
    /// The beginning of the buffer is always at the beginning of the next
    /// packet.
    buf: BytesMut,
    pending_bytes: Arc<PendingBytes>,
    /// Scratch space for decompression.
    decompress_buf: Vec<u8>,
    /// If compression is enabled or not.
    compression: bool,
    /// Cipher for decryption.
    cipher: Option<Cipher>,
}

#[derive(Copy, Clone, Debug, Error)]
pub enum RecvError {
    #[error("packet receiver disconnected")]
    Disconnected,
}

impl IncomingPacketReceiver {
    /// Reads any pending data from the [`IncomingPacketSender`] and puts
    /// it in the queue. If there is no pending data or the receiver is
    /// disconnected, the function returns without any effect.
    ///
    /// This function will never block.
    pub fn recv_nonblocking(&mut self) -> Result<(), RecvError> {
        let start_len = self.buf.len();

        loop {
            match self.recv.try_recv() {
                Ok(bytes) => {
                    self.buf.unsplit(bytes);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Err(RecvError::Disconnected),
            }
        }

        if let Some(cipher) = &mut self.cipher {
            if self.buf.len() > start_len {
                cipher.decrypt(&mut self.buf[start_len..]);
            }
        }

        Ok(())
    }

    /// Reads any pending data from the [`IncomingPacketSender`] and puts it in
    /// the queue. If there is no pending data, the future will yield to the
    /// async runtime until some data is available or the receiver is
    /// disconnected.
    pub async fn recv_async(&mut self) -> Result<(), RecvError> {
        let start_len = self.buf.len();

        if self.recv.is_empty() {
            self.buf.unsplit(
                self.recv
                    .recv_async()
                    .await
                    .map_err(|_| RecvError::Disconnected)?,
            );
        } else {
            loop {
                match self.recv.try_recv() {
                    Ok(bytes) => {
                        self.buf.unsplit(bytes);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return Err(RecvError::Disconnected),
                }
            }
        }

        if let Some(cipher) = &mut self.cipher {
            if self.buf.len() > start_len {
                cipher.decrypt(&mut self.buf[start_len..]);
            }
        }

        Ok(())
    }

    pub async fn next_packet<P>(&mut self) -> anyhow::Result<P>
    where
        P: DecodePacket,
    {
        loop {
            // Check if there's a complete packet available to read.
            if let Some(pkt) = self.try_next_packet()? {
                return Ok(pkt);
            }
            // If not, get some more data.
            self.recv_async().await?;
        }
    }

    /// If the entire packet is not available, `Ok(None)` is returned.
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

        self.pending_bytes
            .current
            .fetch_sub(total_packet_len, Ordering::SeqCst);

        Ok(Some(packet))
    }

    pub fn set_compression(&mut self, compression: bool) {
        self.compression = compression;
    }

    /// Enables encryption for all future packets.
    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        assert!(self.cipher.is_none(), "encryption is already enabled");
        let mut cipher = Cipher::new(key.into(), key.into());
        // Don't forget to decrypt the data we already have.
        cipher.decrypt(&mut self.buf);
        self.cipher = Some(cipher);
    }
}
