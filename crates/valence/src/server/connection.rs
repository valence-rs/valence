use std::io::ErrorKind;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{io, mem};

use anyhow::bail;
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{debug, warn};
use valence_protocol::decoder::{decode_packet, PacketDecoder};
use valence_protocol::encoder::PacketEncoder;
use valence_protocol::var_int::VarInt;
use valence_protocol::{Decode, Packet};

use crate::client::{ClientConnection, ReceivedPacket};
use crate::server::byte_channel::{byte_channel, ByteSender, TrySendError};
use crate::server::NewClientInfo;

pub(super) struct InitialConnection<R, W> {
    reader: R,
    writer: W,
    enc: PacketEncoder,
    dec: PacketDecoder,
    frame: BytesMut,
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
            frame: BytesMut::new(),
            timeout,
            permit,
        }
    }

    pub async fn send_packet<'a, P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: Packet<'a>,
    {
        self.enc.append_packet(pkt)?;
        let bytes = self.enc.take();
        timeout(self.timeout, self.writer.write_all(&bytes)).await??;
        Ok(())
    }

    pub async fn recv_packet<'a, P>(&'a mut self) -> anyhow::Result<P>
    where
        P: Packet<'a>,
    {
        timeout(self.timeout, async {
            loop {
                if let Some(frame) = self.dec.try_next_packet()? {
                    self.frame = frame;

                    return decode_packet(&self.frame);
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
        })
        .await?
    }

    #[allow(dead_code)]
    pub fn set_compression(&mut self, threshold: Option<u32>) {
        self.enc.set_compression(threshold);
        self.dec.set_compression(threshold);
    }

    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        self.enc.enable_encryption(key);
        self.dec.enable_encryption(key);
    }

    pub fn into_client_args(
        mut self,
        info: NewClientInfo,
        incoming_limit: usize,
        outgoing_limit: usize,
    ) -> NewClientArgs
    where
        R: Send + 'static,
        W: Send + 'static,
    {
        let (mut incoming_sender, incoming_receiver) = flume::unbounded();

        let recv_sem = Arc::new(Semaphore::new(incoming_limit));
        let recv_sem_clone = recv_sem.clone();

        let reader_task = tokio::spawn(async move {
            let mut buf = BytesMut::new();

            loop {
                buf.reserve(READ_BUF_SIZE);

                match self.reader.read_buf(&mut buf).await {
                    Ok(0) => break, // Reader is at EOF.
                    Ok(_) => {}
                    Err(e) => {
                        debug!("error reading data from stream: {e}");
                        break;
                    }
                }

                self.dec.queue_bytes(buf.split());

                let mut data = match self.dec.try_next_packet() {
                    Ok(Some(data)) => data,
                    Ok(None) => continue, // Incomplete packet. Need more data.
                    Err(e) => {
                        warn!("error decoding packet frame: {e:#}");
                        break;
                    }
                };

                let timestamp = Instant::now();

                // Remove the packet ID from the front of the data.
                let packet_id = {
                    let mut r = &data[..];

                    match VarInt::decode(&mut r) {
                        Ok(id) => {
                            data.advance(data.len() - r.len());
                            id.0
                        }
                        Err(e) => {
                            warn!("failed to decode packet ID: {e:#}");
                            break;
                        }
                    }
                };

                // Estimate memory usage of this packet.
                let cost = mem::size_of::<ReceivedPacket>() + data.len();

                if cost > incoming_limit {
                    debug!(
                        cost,
                        incoming_limit,
                        "cost of received packet is greater than the incoming memory limit"
                    );
                    // We would never acquire enough permits, so we should exit instead of getting
                    // stuck.
                    break;
                }

                // Wait until there's enough space for this packet.
                let Ok(permits) = recv_sem.acquire_many(cost as u32).await else {
                    // Semaphore closed.
                    break;
                };

                // The permits will be added back on the other side of the channel.
                permits.forget();

                let packet = ReceivedPacket {
                    timestamp: Instant::now(),
                    id: packet_id,
                    data: data.freeze(),
                };

                if incoming_sender.try_send(packet).is_err() {
                    // Channel closed.
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
                    debug!("error writing data to stream: {e}");
                }
            }
        });

        NewClientArgs {
            info,
            conn: Box::new(RealClientConnection {
                send: outgoing_sender,
                recv: incoming_receiver,
                recv_sem: recv_sem_clone,
                _client_permit: self.permit,
                reader_task,
                writer_task,
            }),
            enc: self.enc,
        }
    }
}

pub struct NewClientArgs {
    pub info: NewClientInfo,
    pub conn: Box<dyn ClientConnection>,
    pub enc: PacketEncoder,
}

struct RealClientConnection {
    send: ByteSender,
    recv: flume::Receiver<ReceivedPacket>,
    /// Limits the amount of data queued in the `recv` channel. Each permit
    /// represents one byte.
    recv_sem: Arc<Semaphore>,
    /// Limits the number of new clients that can connect to the server. Permit
    /// is released when the connection is dropped.
    _client_permit: OwnedSemaphorePermit,
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

    fn try_recv(&mut self) -> anyhow::Result<Option<ReceivedPacket>> {
        match self.recv.try_recv() {
            Ok(packet) => {
                let cost = mem::size_of::<ReceivedPacket>() + packet.data.len();

                // Add the permits back that we removed eariler.
                self.recv_sem.add_permits(cost);

                Ok(Some(packet))
            }
            Err(flume::TryRecvError::Empty) => Ok(None),
            Err(flume::TryRecvError::Disconnected) => bail!("disconnected"),
        }
    }

    fn len(&self) -> usize {
        self.recv.len()
    }
}
