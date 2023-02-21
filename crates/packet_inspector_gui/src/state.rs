use std::io::ErrorKind;
use std::sync::{Arc, Mutex};

use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use valence_protocol::{DecodePacket, EncodePacket, PacketDecoder, PacketEncoder};

use crate::context::{Context, Packet, Stage};
use crate::packet_widget::PacketDirection;

pub struct State {
    pub direction: PacketDirection,
    pub context: Arc<Context>,
    // cli: Arc<Cli>,
    pub enc: PacketEncoder,
    pub dec: PacketDecoder,
    pub read: OwnedReadHalf,
    pub write: OwnedWriteHalf,
}

impl State {
    pub async fn rw_packet<'a, P>(&'a mut self, stage: Stage) -> anyhow::Result<P>
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

        let has_compression = self.dec.compression().clone();
        let pkt: P = self.dec.try_next_packet()?.unwrap();

        self.enc.append_packet(&pkt)?;

        let bytes = self.enc.take();
        self.write.write_all(&bytes).await?;

        let time = match OffsetDateTime::now_local() {
            Ok(time) => time,
            Err(_) => {
                eprintln!("Unable to get local time, using UTC"); // this might get a bit spammy..
                OffsetDateTime::now_utc()
            }
        };

        self.context.add(Packet {
            id: 0, // updated when added to context
            direction: self.direction.clone(),
            selected: false,
            packet_type: bytes[0],
            use_compression: has_compression,
            packet_name: Arc::new(Mutex::new(None)),
            packet_str: Arc::new(Mutex::new(None)),
            packet_data: bytes.to_vec(),
            stage,
            created_at: time,
        });

        // println!("{}", self.buf);

        Ok(pkt)
    }
}
