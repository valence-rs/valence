use std::io::ErrorKind;
use std::sync::Arc;

use bytes::BytesMut;
use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use valence::protocol::decode::{decode_packet, PacketDecoder};
use valence::protocol::encode::PacketEncoder;
use valence::protocol::Packet as ValencePacket;

use crate::context::{Context, Packet, Stage};
use crate::packet_widget::PacketDirection;

pub struct State {
    pub direction: PacketDirection,
    pub context: Arc<Context>,
    pub enc: PacketEncoder,
    pub dec: PacketDecoder,
    pub frame: BytesMut,
    pub read: OwnedReadHalf,
    pub write: OwnedWriteHalf,
}

impl State {
    pub async fn rw_packet<'a, P>(&'a mut self, stage: Stage) -> anyhow::Result<P>
    where
        P: ValencePacket<'a>,
    {
        loop {
            if let Some(frame) = self.dec.try_next_packet()? {
                self.frame = frame;

                let pkt: P = decode_packet(&self.frame)?;

                self.enc.append_packet(&pkt)?;

                let bytes = self.enc.take();
                self.write.write_all(&bytes).await?;

                let time = match OffsetDateTime::now_local() {
                    Ok(time) => time,
                    Err(_) => OffsetDateTime::now_utc(),
                };

                self.context.add(Packet {
                    id: 0, // updated when added to context
                    direction: self.direction.clone(),
                    compression_threshold: self.dec.compression(),
                    packet_data: bytes.to_vec(),
                    stage,
                    created_at: time,
                    selected: false,
                    packet_type: pkt.packet_id(),
                    packet_name: pkt.packet_name().to_string(),
                });

                return Ok(pkt);
            }

            self.dec.reserve(4096);
            let mut buf = self.dec.take_capacity();

            if self.read.read_buf(&mut buf).await? == 0 {
                return Err(std::io::Error::from(ErrorKind::UnexpectedEof).into());
            }

            self.dec.queue_bytes(buf);
        }
    }
}
