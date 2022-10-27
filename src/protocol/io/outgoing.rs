use std::io::ErrorKind;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use aes::cipher::{AsyncStreamCipher, NewCipher};
use anyhow::{bail, ensure};
use bytes::{BufMut, BytesMut};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use flume::{Receiver, Sender, TryRecvError};
use tokio::io;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::protocol::io::{Cipher, PendingBytes};
use crate::protocol::packets::EncodePacket;
use crate::protocol::{Encode, VarInt, MAX_PACKET_SIZE};

pub fn outgoing(limit: usize) -> (OutgoingPacketSender, OutgoingPacketReceiver) {
    let pending_bytes = Arc::new(PendingBytes::new(limit));

    let (send, recv) = flume::unbounded();

    let sender = OutgoingPacketSender {
        send,
        buf: BytesMut::new(),
        compress_buf: vec![],
        pending_bytes: pending_bytes.clone(),
        compression_threshold: None,
        cipher: None,
    };

    let receiver = OutgoingPacketReceiver {
        recv,
        pending_bytes,
    };

    (sender, receiver)
}

pub struct OutgoingPacketSender {
    send: Sender<BytesMut>,
    /// The buffer where packets are queued.
    buf: BytesMut,
    /// Scratch space for compression.
    compress_buf: Vec<u8>,
    pending_bytes: Arc<PendingBytes>,
    /// Compression is disabled when `None`.
    compression_threshold: Option<u32>,
    /// Cipher for encryption.
    cipher: Option<Cipher>,
}

impl OutgoingPacketSender {
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

    pub fn flush(&mut self) -> anyhow::Result<()> {
        let len = self.buf.len();

        if len == 0 {
            return Ok(());
        }

        if self
            .pending_bytes
            .current
            .fetch_add(len, Ordering::SeqCst)
            .saturating_add(len)
            >= self.pending_bytes.limit
        {
            bail!("reached pending byte limit of {}", self.pending_bytes.limit);
        }

        if let Some(cipher) = &mut self.cipher {
            cipher.encrypt(&mut self.buf);
        }

        self.send.try_send(self.buf.split())?;

        Ok(())
    }

    pub fn set_compression(&mut self, threshold: Option<u32>) {
        self.compression_threshold = threshold;
    }

    /// Enables encryption for all future packets **and any unflushed packets.**
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

pub struct OutgoingPacketReceiver {
    recv: Receiver<BytesMut>,
    pending_bytes: Arc<PendingBytes>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum WriteLoop {
    /// Reached EOF on the writer.
    Eof,
    /// The [`OutgoingPacketReceiver`] was disconnected.
    Disconnected,
    /// There is no more data in the [`OutgoingPacketReceiver`] to write.
    Empty,
}

impl OutgoingPacketReceiver {
    pub async fn write_loop<W>(&mut self, mut writer: W, await_more: bool) -> io::Result<WriteLoop>
    where
        W: AsyncWrite + Unpin,
    {
        loop {
            let bytes = if await_more {
                match self.recv.recv_async().await {
                    Ok(bytes) => bytes,
                    Err(_) => return Ok(WriteLoop::Disconnected),
                }
            } else {
                match self.recv.try_recv() {
                    Ok(bytes) => bytes,
                    Err(TryRecvError::Disconnected) => return Ok(WriteLoop::Disconnected),
                    Err(TryRecvError::Empty) => return Ok(WriteLoop::Empty),
                }
            };

            let byte_count = bytes.len();
            if let Err(e) = writer.write_all(&bytes).await {
                if e.kind() == ErrorKind::WriteZero {
                    return Ok(WriteLoop::Eof);
                }
            }
            self.pending_bytes
                .current
                .fetch_sub(byte_count, Ordering::SeqCst);
        }
    }
}
