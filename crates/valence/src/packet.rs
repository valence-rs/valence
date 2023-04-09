use std::io::Write;

use tracing::warn;
use valence_protocol::encoder::{encode_packet, encode_packet_compressed, PacketEncoder};
use valence_protocol::Packet;

/// Types that can have packets written to them.
pub trait WritePacket {
    /// Writes a packet to this object. Encoding errors are typically logged and
    /// discarded.
    fn write_packet<'a>(&mut self, packet: &impl Packet<'a>);
    /// Copies raw packet data directly into this object. Don't use this unless
    /// you know what you're doing.
    fn write_packet_bytes(&mut self, bytes: &[u8]);
}

impl<W: WritePacket> WritePacket for &mut W {
    fn write_packet<'a>(&mut self, packet: &impl Packet<'a>) {
        (*self).write_packet(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        (*self).write_packet_bytes(bytes)
    }
}

/// An implementor of [`WritePacket`] backed by a `Vec` reference.
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
    fn write_packet<'a>(&mut self, pkt: &impl Packet<'a>) {
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

impl WritePacket for PacketEncoder {
    fn write_packet<'a>(&mut self, packet: &impl Packet<'a>) {
        if let Err(e) = self.append_packet(packet) {
            warn!("failed to write packet: {e:#}");
        }
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.append_bytes(bytes)
    }
}
