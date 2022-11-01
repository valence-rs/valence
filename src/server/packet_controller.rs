use std::io::ErrorKind;
use std::time::Duration;

use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::protocol::codec::{PacketDecoder, PacketEncoder};
use crate::protocol::packets::{DecodePacket, EncodePacket};
use crate::server::byte_channel::{byte_channel, ByteReceiver, ByteSender, TryRecvError};

pub struct InitialPacketController<R, W> {
    reader: R,
    writer: W,
    enc: PacketEncoder,
    dec: PacketDecoder,
    timeout: Duration,
}

const READ_BUF_SIZE: usize = 4096;

impl<R, W> InitialPacketController<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub fn new(
        reader: R,
        writer: W,
        enc: PacketEncoder,
        dec: PacketDecoder,
        timeout: Duration,
    ) -> Self {
        Self {
            reader,
            writer,
            enc,
            dec,
            timeout,
        }
    }

    pub async fn send_packet<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        self.enc.append_packet(pkt)?;
        let bytes = self.enc.take();
        timeout(self.timeout, self.writer.write_all(&bytes)).await??;
        Ok(())
    }

    pub async fn recv_packet<P>(&mut self) -> anyhow::Result<P>
    where
        P: DecodePacket,
    {
        timeout(self.timeout, async {
            loop {
                if let Some(pkt) = self.dec.try_next_packet()? {
                    return Ok(pkt);
                }

                self.dec.reserve(READ_BUF_SIZE);
                let mut buf = self.dec.take_capacity();

                if self.reader.read_buf(&mut buf).await? == 0 {
                    return Err(io::Error::from(ErrorKind::UnexpectedEof).into());
                }

                // This should always be an O(1) unsplit because we reserved space earlier and
                // the previous call to `read_buf` shouldn't have grown the allocation.
                self.dec.queue_bytes(buf);
            }
        })
        .await?
    }

    #[allow(dead_code)]
    pub fn set_compression(&mut self, threshold: Option<u32>) {
        self.enc.set_compression(threshold);
        self.dec.set_compression(threshold.is_some());
    }

    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        self.enc.enable_encryption(key);
        self.dec.enable_encryption(key);
    }

    pub fn into_play_packet_controller(
        mut self,
        incoming_limit: usize,
        outgoing_limit: usize,
    ) -> PlayPacketController
    where
        R: Send + 'static,
        W: Send + 'static,
    {
        let (mut incoming_sender, incoming_receiver) = byte_channel(incoming_limit);

        let reader_task = tokio::spawn(async move {
            loop {
                let mut buf = incoming_sender.take_capacity(READ_BUF_SIZE);

                match self.reader.read_buf(&mut buf).await {
                    Ok(0) => break,
                    Err(e) => {
                        log::warn!("error reading packet data: {e}");
                        break;
                    }
                    _ => {}
                }

                // This should always be an O(1) unsplit because we reserved space earlier.
                if let Err(e) = incoming_sender.send_async(buf).await {
                    log::warn!("error sending packet data: {e}");
                    break;
                }
            }
        });

        let (outgoing_sender, mut outgoing_receiver) = byte_channel(outgoing_limit);

        let writer_task = tokio::spawn(async move {
            loop {
                let bytes = match outgoing_receiver.recv_async().await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        log::warn!("error receiving packet data: {e}");
                        break;
                    }
                };

                if let Err(e) = self.writer.write_all(&bytes).await {
                    log::warn!("error writing packet data: {e}");
                }
            }
        });

        PlayPacketController {
            enc: self.enc,
            dec: self.dec,
            send: outgoing_sender,
            recv: incoming_receiver,
            reader_task,
            writer_task: Some(writer_task),
        }
    }
}

/// A convenience structure for managing a pair of packet encoder/decoders and
/// the byte channels from which to send and receive the packet data during the
/// play state.
pub struct PlayPacketController {
    enc: PacketEncoder,
    dec: PacketDecoder,
    send: ByteSender,
    recv: ByteReceiver,
    reader_task: JoinHandle<()>,
    writer_task: Option<JoinHandle<()>>,
}

impl PlayPacketController {
    pub fn append_packet<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        self.enc.append_packet(pkt)
    }

    pub fn prepend_packet<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        self.enc.prepend_packet(pkt)
    }

    pub fn try_next_packet<P>(&mut self) -> anyhow::Result<Option<P>>
    where
        P: DecodePacket,
    {
        self.dec.try_next_packet()
    }

    /// Returns true if the client is connected. Returns false otherwise.
    pub fn try_recv(&mut self) -> bool {
        match self.recv.try_recv() {
            Ok(bytes) => {
                self.dec.queue_bytes(bytes);
                true
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => false,
        }
    }

    #[allow(dead_code)]
    pub fn set_compression(&mut self, threshold: Option<u32>) {
        self.enc.set_compression(threshold)
    }

    pub fn flush(&mut self) -> anyhow::Result<()> {
        let bytes = self.enc.take();
        self.send.try_send(bytes)?;
        Ok(())
    }
}

impl Drop for PlayPacketController {
    fn drop(&mut self) {
        self.reader_task.abort();

        let _ = self.flush();

        if let Some(writer_task) = self.writer_task.take() {
            if !writer_task.is_finished() {
                // Give any unsent packets a moment to send before we cut the connection.
                tokio::spawn(timeout(Duration::from_secs(1), writer_task));
            }
        }
    }
}
