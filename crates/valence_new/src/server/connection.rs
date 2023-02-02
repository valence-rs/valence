use std::io;
use std::io::ErrorKind;
use std::time::Duration;

use anyhow::bail;
use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::OwnedSemaphorePermit;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::debug;
use valence_protocol::{DecodePacket, EncodePacket, PacketDecoder, PacketEncoder};

use crate::client::{Client, ClientConnection};
use crate::server::byte_channel::{
    byte_channel, ByteReceiver, ByteSender, TryRecvError, TrySendError,
};
use crate::server::NewClientInfo;

pub(super) struct InitialConnection<R, W> {
    reader: R,
    writer: W,
    enc: PacketEncoder,
    dec: PacketDecoder,
    timeout: Duration,
    permit: OwnedSemaphorePermit,
}

const READ_BUF_SIZE: usize = 4096;

impl<R, W> InitialConnection<R, W>
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
        permit: OwnedSemaphorePermit,
    ) -> Self {
        Self {
            reader,
            writer,
            enc,
            dec,
            timeout,
            permit,
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

    pub async fn recv_packet<'a, P>(&'a mut self) -> anyhow::Result<P>
    where
        P: DecodePacket<'a>,
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

    pub fn into_client(
        mut self,
        info: NewClientInfo,
        incoming_limit: usize,
        outgoing_limit: usize,
    ) -> Client
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
                        debug!("error reading packet data: {e}");
                        break;
                    }
                    _ => {}
                }

                // This should always be an O(1) unsplit because we reserved space earlier.
                if let Err(e) = incoming_sender.send_async(buf).await {
                    debug!("error sending packet data: {e}");
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
                        debug!("error receiving packet data: {e}");
                        break;
                    }
                };

                if let Err(e) = self.writer.write_all(&bytes).await {
                    debug!("error writing packet data: {e}");
                }
            }
        });

        Client::new(
            info,
            Box::new(RealClientConnection {
                send: outgoing_sender,
                recv: incoming_receiver,
                _permit: self.permit,
                reader_task,
                writer_task,
            }),
            self.enc,
            self.dec,
        )
    }
}

struct RealClientConnection {
    send: ByteSender,
    recv: ByteReceiver,
    /// Ensures that we don't allow more connections to the server until the
    /// client is dropped.
    _permit: OwnedSemaphorePermit,
    reader_task: JoinHandle<()>,
    writer_task: JoinHandle<()>,
}

impl Drop for RealClientConnection {
    fn drop(&mut self) {
        self.writer_task.abort();
        self.reader_task.abort();
    }
}

impl ClientConnection for RealClientConnection {
    fn try_send(&mut self, bytes: BytesMut) -> anyhow::Result<()> {
        match self.send.try_send(bytes) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => bail!(
                "reached configured outgoing limit of {} bytes",
                self.send.limit()
            ),
            Err(TrySendError::Disconnected(_)) => bail!("client disconnected"),
        }
    }

    fn try_recv(&mut self) -> anyhow::Result<BytesMut> {
        match self.recv.try_recv() {
            Ok(bytes) => Ok(bytes),
            Err(TryRecvError::Empty) => Ok(BytesMut::new()),
            Err(TryRecvError::Disconnected) => bail!("client disconnected"),
        }
    }
}
