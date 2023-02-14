use std::fmt::Write;
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::SystemTime;

use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use valence_protocol::{DecodePacket, EncodePacket, PacketDecoder, PacketEncoder};

use crate::context::{Context, Packet};
use crate::packet_widget::PacketDirection;

pub struct State {
    pub direction: PacketDirection,
    pub context: Arc<Context>,
    // cli: Arc<Cli>,
    pub enc: PacketEncoder,
    pub dec: PacketDecoder,
    pub read: OwnedReadHalf,
    pub write: OwnedWriteHalf,
    pub buf: String,
}

impl State {
    pub async fn rw_packet<'a, P>(&'a mut self) -> anyhow::Result<P>
    where
        P: DecodePacket<'a> + EncodePacket,
    {
        while !self.dec.has_next_packet()? {
            self.dec.reserve(4096);
            let mut buf = self.dec.take_capacity();

            if self.read.read_buf(&mut buf).await? == 0 {
                return Err(std::io::Error::from(ErrorKind::UnexpectedEof).into());
            }

            self.dec.queue_bytes(buf);
        }

        let pkt: P = self.dec.try_next_packet()?.unwrap();

        self.enc.append_packet(&pkt)?;

        let bytes = self.enc.take();
        self.write.write_all(&bytes).await?;

        self.buf.clear();
        write!(&mut self.buf, "{pkt:#?}")?;

        let packet_name = self
            .buf
            .split_once(|ch: char| !ch.is_ascii_alphabetic())
            .map(|(fst, _)| fst)
            .unwrap_or(&self.buf);

        self.context.add(Packet {
            id: 0, // updated when added to context
            direction: self.direction.clone(),
            selected: false,
            packet_type: bytes[0],
            packet_name: packet_name.to_owned(),
            packet: self.buf.clone(),
            created_at: OffsetDateTime::now_local().unwrap(),
        });

        // println!("{}", self.buf);

        Ok(pkt)
    }
}
