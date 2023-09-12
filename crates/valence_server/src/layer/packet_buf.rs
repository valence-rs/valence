use bevy_ecs::prelude::*;
use derive_more::{Deref, DerefMut};
use valence_protocol::encode::PacketWriter;
use valence_protocol::{CompressionThreshold, Encode, Packet, WritePacket};
use valence_server_common::Server;

use crate::Client;

#[derive(Component, Clone, Deref, DerefMut, Eq, Debug)]
pub struct PacketBuf {
    #[deref]
    #[deref_mut]
    buf: Vec<u8>,
    threshold: CompressionThreshold,
}

impl PartialEq for PacketBuf {
    fn eq(&self, other: &Self) -> bool {
        self.buf.eq(&other.buf)
    }
}

impl WritePacket for PacketBuf {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        PacketWriter::new(&mut self.buf, self.threshold).write_packet_fallible(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes)
    }
}

impl PacketBuf {
    /// Creates a new, empty packet buffer.
    pub fn new(server: &Server) {
        Self {
            buf: vec![],
            threshold: server.compression_threshold(),
        }
    }

    /// Sends all packet data in this buffer to all clients given by the
    /// iterator. The buffer is then cleared.
    pub fn broadcast<I, C>(&mut self, clients: I)
    where
        I: IntoIterator<Item = C>,
        C: DerefMut<Target = Client>,
    {
        if !self.is_empty() {
            for mut client in clients {
                client.write_packet_bytes(&self);
            }

            self.clear();
        }
    }
}
