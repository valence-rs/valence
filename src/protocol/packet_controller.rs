use std::time::Duration;

use crate::protocol::byte_channel::{ByteReceiver, ByteSender};
use crate::protocol::codec_new::{PacketDecoder, PacketEncoder};
use crate::protocol::packets::{DecodePacket, EncodePacket};

/// A convenience structure for managing a pair of packet encoder/decoders and
/// the byte channels from which to send and receive the packet data.
///
/// This is especially useful in the initial packet exchange between client and
/// server.
pub struct PacketController {
    pub enc: PacketEncoder,
    pub dec: PacketDecoder,
    pub send: ByteSender,
    pub recv: ByteReceiver,

    pub timeout: Duration,
}

impl PacketController {
    pub fn send_packet<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        self.enc.append_packet(pkt)?;
        self.flush()?;
        Ok(())
    }

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

    pub async fn recv_packet_async<P>(&mut self) -> anyhow::Result<P>
    where
        P: DecodePacket,
    {
        loop {
            if let Some(pkt) = self.dec.try_next_packet()? {
                return Ok(pkt);
            }

            self.dec.queue_bytes(self.recv.try_recv()?);
        }
    }

    pub fn flush(&mut self) -> anyhow::Result<()> {
        Ok(self.send.try_send(self.enc.take())?)
    }

    pub fn set_compression(&mut self, threshold: Option<u32>) {
        self.enc.set_compression(threshold);
        self.dec.set_compression(threshold.is_some());
    }

    pub fn enable_encryption(&mut self, key: &[u8; 16]) -> anyhow::Result<()> {
        self.flush()?;
        self.enc.enable_encryption(key);
        self.dec.enable_encryption(key);
        Ok(())
    }
}
