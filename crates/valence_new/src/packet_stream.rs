use std::collections::VecDeque;

use bytes::{BufMut, BytesMut};
use valence_protocol::packets::S2cPlayPacket;
use valence_protocol::{DecodePacket, EncodePacket, PacketDecoder, PacketEncoder};

/// Represents a byte stream that packets can be read from and written to.
trait PacketStream {
    /// Parses and returns the next packet in the stream.
    fn try_recv<'a, P>(&'a mut self) -> anyhow::Result<Option<P>>
    where
        P: DecodePacket<'a> + std::fmt::Debug;

    /// Encodes and writes the given packet to the stream.
    fn try_send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: EncodePacket;
}

/// A `PacketStream` that reads and writes from an in memory buffer of packets
/// used for testing.
pub(crate) struct MockPacketStream {
    recv_enc: PacketEncoder,
    recv_dec: PacketDecoder,

    send_queue: VecDeque<Vec<u8>>,
}

impl<'a> MockPacketStream {
    pub(crate) fn new() -> Self {
        Self {
            recv_enc: PacketEncoder::new(),
            recv_dec: PacketDecoder::new(),
            send_queue: VecDeque::new(),
        }
    }

    /// Injects a packet into the receive stream as if it were received from a
    /// client.
    ///
    /// ```rust
    /// use valence_new::packet_stream::MockPacketStream;
    /// use valence_protocol::packets::c2s::play::KeepAliveC2s;
    ///
    /// let mut stream = MockPacketStream::new();
    /// let packet = KeepAliveC2s { id: 0xdeadbeef };
    /// stream.inject_recv(packet);
    /// ```
    pub(crate) fn inject_recv<P>(&mut self, packet: P)
    where
        P: EncodePacket,
    {
        self.recv_enc
            .append_packet(&packet)
            .expect("failed to encode injected packet");
        let bytes = self.recv_enc.take();
        self.recv_dec.queue_bytes(bytes);
    }

    fn queue_send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: EncodePacket,
    {
        let bytes = BytesMut::new();
        let mut w = bytes.writer();
        P::encode_packet(&packet, &mut w);
        let bytes = w.into_inner();
        self.send_queue.push_back(bytes.to_vec());
        Ok(())
    }

    pub(crate) fn flush_sent(&'a mut self) -> anyhow::Result<Vec<S2cPlayPacket<'a>>> {
        let mut packets = Vec::new();
        for pkt in self.send_queue.drain(..) {
            let packet = S2cPlayPacket::<'a>::decode_packet(&mut pkt.as_slice())?;
            packets.push(packet);
        }
        Ok(packets)
    }
}

impl PacketStream for MockPacketStream {
    fn try_recv<'a, P>(&'a mut self) -> anyhow::Result<Option<P>>
    where
        P: DecodePacket<'a> + std::fmt::Debug,
    {
        self.recv_dec.try_next_packet()
    }

    fn try_send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: EncodePacket,
    {
        self.queue_send(packet)
    }
}

#[cfg(test)]
mod tests {
    use valence_protocol::packets::c2s::play::KeepAliveC2s;
    use valence_protocol::packets::s2c::play::KeepAliveS2c;

    use super::*;

    #[test]
    fn test_mock_stream_read() {
        let mut stream = MockPacketStream::new();
        let packet = KeepAliveC2s { id: 0xdeadbeef };
        stream.inject_recv(packet.clone());
        let packet_out = stream.try_recv::<KeepAliveC2s>().unwrap().unwrap();
        assert_eq!(packet.id, packet_out.id);
    }

    #[test]
    fn test_mock_stream_assert_sent() {
        let mut stream = MockPacketStream::new();
        let packet = KeepAliveS2c { id: 0xdeadbeef };
        stream.try_send(packet.clone()).unwrap();
        let packets_out = stream.flush_sent().unwrap();
        let S2cPlayPacket::KeepAliveS2c(packet_out) = packets_out[0] else {
            assert!(false);
            return;
        };
        assert_eq!(packet.id, packet_out.id);
    }
}
