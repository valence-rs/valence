use std::io::ErrorKind;
use std::time::Duration;

use anyhow::Result;
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use valence_protocol::{Decode, Encode, Packet, PacketDecoder, PacketEncoder};

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

    pub async fn send_packet<P>(&mut self, pkt: &P) -> Result<()>
    where
        P: Encode + Packet + ?Sized,
    {
        self.enc.append_packet(pkt)?;
        let bytes = self.enc.take();
        timeout(self.timeout, self.writer.write_all(&bytes)).await??;
        Ok(())
    }

    pub async fn recv_packet<'a, P>(&'a mut self) -> Result<P>
    where
        P: Decode<'a> + Packet,
    {
        timeout(self.timeout, async {
            while !self.dec.has_next_packet()? {
                self.dec.reserve(READ_BUF_SIZE);
                let mut buf = self.dec.take_capacity();

                if self.reader.read_buf(&mut buf).await? == 0 {
                    return Err(io::Error::from(ErrorKind::UnexpectedEof).into());
                }

                // This should always be an O(1) unsplit because we reserved space earlier and
                // the call to `read_buf` shouldn't have grown the allocation.
                self.dec.queue_bytes(buf);
            }

            Ok(self
                .dec
                .try_next_packet()?
                .expect("decoder said it had another packet"))

            // The following is what I want to write but can't due to borrow
            // checker errors I don't understand.
            /*
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
                // the call to `read_buf` shouldn't have grown the allocation.
                self.dec.queue_bytes(buf);
            }
            */
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
        handle: Handle,
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
            handle,
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
    handle: Handle,
}

impl PlayPacketController {
    pub fn append_packet<P>(&mut self, pkt: &P) -> Result<()>
    where
        P: Encode + Packet + ?Sized,
    {
        self.enc.append_packet(pkt)
    }

    pub fn prepend_packet<P>(&mut self, pkt: &P) -> Result<()>
    where
        P: Encode + Packet + ?Sized,
    {
        self.enc.prepend_packet(pkt)
    }

    pub fn try_next_packet<'a, P>(&'a mut self) -> Result<Option<P>>
    where
        P: Decode<'a> + Packet,
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

    pub fn flush(&mut self) -> Result<()> {
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
                let _guard = self.handle.enter();

                // Give any unsent packets a moment to send before we cut the connection.
                self.handle
                    .spawn(timeout(Duration::from_secs(1), writer_task));
            }
        }
    }
}
