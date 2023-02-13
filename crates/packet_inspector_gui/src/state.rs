use std::{io::ErrorKind, sync::Arc};

use std::fmt::Write;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};
use valence_protocol::{DecodePacket, EncodePacket, PacketDecoder, PacketEncoder};

use crate::context::Context;
use crate::packet_widget::PacketDirection;

pub struct State {
    direction: PacketDirection,
    context: Arc<Context>,
    // cli: Arc<Cli>,
    enc: PacketEncoder,
    dec: PacketDecoder,
    read: OwnedReadHalf,
    write: OwnedWriteHalf,
    buf: String,
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
        write!(&mut self.buf, "{pkt:?}")?;

        let packet_name = self
            .buf
            .split_once(|ch: char| !ch.is_ascii_alphabetic())
            .map(|(fst, _)| fst)
            .unwrap_or(&self.buf);

        // if let Some(r) = &self.cli.include_regex {
        //     if !r.is_match(packet_name) {
        //         return Ok(pkt);
        //     }
        // }

        // if let Some(r) = &self.cli.exclude_regex {
        //     if r.is_match(packet_name) {
        //         return Ok(pkt);
        //     }
        // }

        println!("{}", self.buf);

        Ok(pkt)
    }
}
