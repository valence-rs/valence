use std::fmt::Write;
use std::io::ErrorKind;
use std::sync::Arc;

use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use valence_protocol::codec::{PacketDecoder, PacketEncoder};
use valence_protocol::Packet as ValencePacket;

use crate::context::{Context, Packet, Stage};
use crate::packet_widget::PacketDirection;

pub struct State {
    pub direction: PacketDirection,
    pub context: Arc<Context>,
    pub enc: PacketEncoder,
    pub dec: PacketDecoder,
    pub read: OwnedReadHalf,
    pub write: OwnedWriteHalf,
    pub buf: String,
}

impl State {
    pub async fn rw_packet<'a, P>(&'a mut self, stage: Stage) -> anyhow::Result<P>
    where
        P: ValencePacket<'a>,
    {
        while !self.dec.has_next_packet()? {
            self.dec.reserve(4096);
            let mut buf = self.dec.take_capacity();

            if self.read.read_buf(&mut buf).await? == 0 {
                return Err(std::io::Error::from(ErrorKind::UnexpectedEof).into());
            }

            self.dec.queue_bytes(buf);
        }

        let has_compression = self.dec.compression();
        let pkt: P = self.dec.try_next_packet()?.unwrap();

        self.buf.clear();
        write!(&mut self.buf, "{pkt:?}")?;

        let packet_name = self
            .buf
            .split_once(|ch: char| !ch.is_ascii_alphanumeric())
            .map(|(fst, _)| fst)
            .unwrap_or(&self.buf);

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
            use_compression: has_compression,
            packet_data: bytes.to_vec(),
            stage,
            created_at: time,
            selected: false,
            packet_type: pkt.packet_id(),
            packet_name: packet_name.to_string(),
        });

        Ok(pkt)
    }
}
