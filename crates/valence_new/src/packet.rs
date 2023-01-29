use std::io::Write;

use tracing::warn;
use valence_protocol::{encode_packet, encode_packet_compressed, EncodePacket};

pub(crate) trait WritePacket {
    fn write_packet<P>(&mut self, packet: &P)
    where
        P: EncodePacket + ?Sized;

    fn write_packet_bytes(&mut self, bytes: &[u8]);
}

impl<W: WritePacket> WritePacket for &mut W {
    fn write_packet<P>(&mut self, packet: &P)
    where
        P: EncodePacket + ?Sized,
    {
        (*self).write_packet(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        (*self).write_packet_bytes(bytes)
    }
}

pub(crate) struct PacketWriter<'a> {
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
    fn write_packet<P>(&mut self, pkt: &P)
    where
        P: EncodePacket + ?Sized,
    {
        let res = if let Some(threshold) = self.threshold {
            encode_packet_compressed(self.buf, pkt, threshold, self.scratch)
        } else {
            encode_packet(self.buf, pkt)
        };

        if let Err(e) = res {
            warn!("failed to write packet: {e:#}");
        }
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        if let Err(e) = self.buf.write_all(bytes) {
            warn!("failed to write packet bytes: {e:#}");
        }
    }
}
