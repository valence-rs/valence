use std::io::Write;

use valence_protocol::{encode_packet, encode_packet_compressed, Encode, Packet};

pub trait WritePacket {
    fn write_packet<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Encode + Packet + ?Sized;

    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()>;
}

impl<W: WritePacket> WritePacket for &mut W {
    fn write_packet<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Encode + Packet + ?Sized,
    {
        (*self).write_packet(packet)
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        (*self).write_bytes(bytes)
    }
}

pub struct PacketWriter<'a> {
    buf: &'a mut Vec<u8>,
    threshold: Option<u32>,
    scratch: &'a mut Vec<u8>,
}

impl<'a> PacketWriter<'a> {
    pub fn new(buf: &'a mut Vec<u8>, threshold: Option<u32>, scratch: &'a mut Vec<u8>) -> Self {
        Self {
            buf,
            threshold,
            scratch,
        }
    }
}

impl WritePacket for PacketWriter<'_> {
    fn write_packet<P>(&mut self, pkt: &P) -> anyhow::Result<()>
    where
        P: Encode + Packet + ?Sized,
    {
        if let Some(threshold) = self.threshold {
            encode_packet_compressed(self.buf, pkt, threshold, self.scratch)
        } else {
            encode_packet(self.buf, pkt)
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        Ok(self.buf.write_all(bytes)?)
    }
}
