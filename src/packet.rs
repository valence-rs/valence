use std::io::Write;

use valence_protocol::{write_packet, write_packet_compressed, Encode, Packet};

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

pub struct PacketWriter<'a, W> {
    pub writer: W,
    pub threshold: Option<u32>,
    pub scratch: &'a mut Vec<u8>,
}

impl<'a, W: Write> PacketWriter<'a, W> {
    pub fn new(writer: W, threshold: Option<u32>, scratch: &'a mut Vec<u8>) -> PacketWriter<W> {
        Self {
            writer,
            threshold,
            scratch,
        }
    }
}

impl<W: Write> WritePacket for PacketWriter<'_, W> {
    fn write_packet<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Encode + Packet + ?Sized,
    {
        if let Some(threshold) = self.threshold {
            write_packet_compressed(&mut self.writer, threshold, self.scratch, packet)
        } else {
            write_packet(&mut self.writer, packet)
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        Ok(self.writer.write_all(bytes)?)
    }
}
