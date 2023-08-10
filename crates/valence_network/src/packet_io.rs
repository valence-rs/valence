use std::io::ErrorKind;
use std::sync::Arc;
use std::time::Instant;
use std::{io, mem};

use anyhow::bail;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tracing::{debug, warn};
use valence_protocol::CompressionThreshold;
use valence_server::client::{ClientBundleArgs, ClientConnection, ReceivedPacket};
use valence_server::protocol::decode::PacketFrame;
use valence_server::protocol::{Decode, Encode, Packet, PacketDecoder, PacketEncoder};

use crate::byte_channel::{byte_channel, ByteSender, TrySendError};
use crate::{CleanupOnDrop, NewClientInfo};

pub(crate) struct PacketIo {
    stream: TcpStream,
    enc: PacketEncoder,
    dec: PacketDecoder,
    frame: PacketFrame,
}

const READ_BUF_SIZE: usize = 4096;

impl PacketIo {
    pub(crate) fn new(stream: TcpStream, enc: PacketEncoder, dec: PacketDecoder) -> Self {
        Self {
            stream,
            enc,
            dec,
            frame: PacketFrame {
                id: -1,
                body: BytesMut::new(),
            },
        }
    }

    pub(crate) async fn send_packet<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.enc.append_packet(pkt)?;
        let bytes = self.enc.take();
        self.stream.write_all(&bytes).await?;
        Ok(())
    }

    pub(crate) async fn recv_packet<'a, P>(&'a mut self) -> anyhow::Result<P>
    where
        P: Packet + Decode<'a>,
    {
        loop {
            if let Some(frame) = self.dec.try_next_packet()? {
                self.frame = frame;

                return self.frame.decode();
            }

            self.dec.reserve(READ_BUF_SIZE);
            let mut buf = self.dec.take_capacity();

            if self.stream.read_buf(&mut buf).await? == 0 {
                return Err(io::Error::from(ErrorKind::UnexpectedEof).into());
            }

            // This should always be an O(1) unsplit because we reserved space earlier and
            // the call to `read_buf` shouldn't have grown the allocation.
            self.dec.queue_bytes(buf);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_compression(&mut self, threshold: CompressionThreshold) {
        self.enc.set_compression(threshold);
        self.dec.set_compression(threshold);
    }

    pub(crate) fn enable_encryption(&mut self, key: &[u8; 16]) {
        self.enc.enable_encryption(key);
        self.dec.enable_encryption(key);
    }

    pub(crate) fn into_client_args(
        mut self,
        info: NewClientInfo,
        incoming_byte_limit: usize,
        outgoing_byte_limit: usize,
        cleanup: CleanupOnDrop,
    ) -> ClientBundleArgs {
        let (incoming_sender, incoming_receiver) = flume::unbounded();

        let incoming_byte_limit = incoming_byte_limit.min(Semaphore::MAX_PERMITS);

        let recv_sem = Arc::new(Semaphore::new(incoming_byte_limit));
        let recv_sem_clone = recv_sem.clone();

        let (mut reader, mut writer) = self.stream.into_split();

        let reader_task = tokio::spawn(async move {
            let mut buf = BytesMut::new();

            loop {
                let frame = match self.dec.try_next_packet() {
                    Ok(Some(frame)) => frame,
                    Ok(None) => {
                        // Incomplete packet. Need more data.

                        buf.reserve(READ_BUF_SIZE);
                        match reader.read_buf(&mut buf).await {
                            Ok(0) => break, // Reader is at EOF.
                            Ok(_) => {}
                            Err(e) => {
                                debug!("error reading data from stream: {e}");
                                break;
                            }
                        }

                        self.dec.queue_bytes(buf.split());

                        continue;
                    }
                    Err(e) => {
                        warn!("error decoding packet frame: {e:#}");
                        break;
                    }
                };

                let timestamp = Instant::now();

                // Estimate memory usage of this packet.
                let cost = mem::size_of::<ReceivedPacket>() + frame.body.len();

                if cost > incoming_byte_limit {
                    debug!(
                        cost,
                        incoming_byte_limit,
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
                    timestamp,
                    id: frame.id,
                    body: frame.body.freeze(),
                };

                if incoming_sender.try_send(packet).is_err() {
                    // Channel closed.
                    break;
                }
            }
        });

        let (outgoing_sender, mut outgoing_receiver) = byte_channel(outgoing_byte_limit);

        let writer_task = tokio::spawn(async move {
            loop {
                let bytes = match outgoing_receiver.recv_async().await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        debug!("error receiving packet data: {e}");
                        break;
                    }
                };

                if let Err(e) = writer.write_all(&bytes).await {
                    debug!("error writing data to stream: {e}");
                }
            }
        });

        ClientBundleArgs {
            username: info.username,
            uuid: info.uuid,
            ip: info.ip,
            properties: info.properties.0,
            conn: Box::new(RealClientConnection {
                send: outgoing_sender,
                recv: incoming_receiver,
                recv_sem: recv_sem_clone,
                reader_task,
                writer_task,
                _cleanup: cleanup,
            }),
            enc: self.enc,
        }
    }
}

struct RealClientConnection {
    send: ByteSender,
    recv: flume::Receiver<ReceivedPacket>,
    /// Limits the amount of data queued in the `recv` channel. Each permit
    /// represents one byte.
    recv_sem: Arc<Semaphore>,
    _cleanup: CleanupOnDrop,
    reader_task: JoinHandle<()>,
    writer_task: JoinHandle<()>,
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
                let cost = mem::size_of::<ReceivedPacket>() + packet.body.len();

                // Add the permits back that we removed eariler.
                self.recv_sem.add_permits(cost);

                Ok(Some(packet))
            }
            Err(flume::TryRecvError::Empty) => Ok(None),
            Err(flume::TryRecvError::Disconnected) => bail!("client disconnected"),
        }
    }

    fn len(&self) -> usize {
        self.recv.len()
    }
}

impl Drop for RealClientConnection {
    fn drop(&mut self) {
        self.writer_task.abort();
        self.reader_task.abort();
    }
}
