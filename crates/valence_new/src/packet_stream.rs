use std::sync::{Arc, Mutex};

use bytes::BytesMut;
use valence_protocol::packets::S2cPlayPacket;
use valence_protocol::{DecodePacket, EncodePacket, PacketDecoder, PacketEncoder};

use crate::packet::WritePacket;
use crate::server::byte_channel::{ByteReceiver, ByteSender, TryRecvError, TrySendError};

pub struct PacketStreamer {
    pub stream: Arc<Mutex<dyn PacketStream + Send + Sync>>,
    enc: PacketEncoder,
    dec: PacketDecoder,
}

impl PacketStreamer {
    pub fn new(
        stream: Arc<Mutex<impl PacketStream + Send + Sync + 'static>>,
        enc: PacketEncoder,
        dec: PacketDecoder,
    ) -> Self {
        Self { stream, enc, dec }
    }

    /// Returns true if the client is still connected.
    pub fn probe_recv(&mut self) -> bool {
        match self.stream.lock().unwrap().try_recv() {
            Ok(bytes) => {
                self.dec.queue_bytes(bytes);
                true
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => false,
        }
    }

    /// Parses and returns the next packet in the receive stream.
    pub fn try_recv<'a, P>(&'a mut self) -> anyhow::Result<Option<P>>
    where
        P: DecodePacket<'a> + std::fmt::Debug,
    {
        loop {
            match self.stream.lock().unwrap().try_recv() {
                Ok(bytes) => self.dec.queue_bytes(bytes),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => anyhow::bail!("Disconnected"),
            }
        }

        self.dec.try_next_packet()
    }

    /// Encodes and writes the given packet to the send queue.
    pub fn try_send<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        self.enc.append_packet(packet)?;
        Ok(())
    }

    /// Encodes and writes the given packet to the beginning of send queue.
    pub fn try_send_prepend<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        self.enc.prepend_packet(packet)?;
        Ok(())
    }

    /// Add a packet to the send queue that has already been encoded.
    pub fn send_raw(&mut self, bytes: &[u8]) {
        self.enc.append_bytes(bytes);
    }

    /// Flushes the send queue to the stream;
    pub fn send_flush(&mut self) -> anyhow::Result<()> {
        self.stream.lock().unwrap().try_send(self.enc.take())?;
        Ok(())
    }

    /// Clears the send queue without flushing.
    pub fn send_drop(&mut self) {
        self.enc.clear();
    }
}

impl WritePacket for PacketStreamer {
    fn write_packet<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        self.try_send(packet)
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.send_raw(bytes);
        Ok(())
    }
}

/// Represents a byte stream that packets can be read from and written to.
pub trait PacketStream {
    /// Grabs all the bytes buffered for the stream and returns them.
    fn try_recv(&mut self) -> Result<BytesMut, TryRecvError>;

    /// Writes the given bytes to the stream.
    fn try_send(&mut self, bytes: BytesMut) -> Result<(), TrySendError>;
}

pub(crate) struct RealPacketStream {
    recv: ByteReceiver,
    send: ByteSender,
}

impl RealPacketStream {
    pub(crate) fn new(recv: ByteReceiver, send: ByteSender) -> Self {
        Self { recv, send }
    }
}

impl PacketStream for RealPacketStream {
    fn try_recv(&mut self) -> Result<BytesMut, TryRecvError> {
        let bytes = self.recv.try_recv()?;
        Ok(bytes)
    }

    fn try_send(&mut self, bytes: BytesMut) -> Result<(), TrySendError> {
        self.send.try_send(bytes)
    }
}

/// A `PacketStream` that reads and writes from an in memory buffer of packets
/// used for testing.
pub(crate) struct MockPacketStream {
    recv_enc: PacketEncoder,
    pending_recv: Vec<u8>,
    send_dec: PacketDecoder,
    flushed_sent: Vec<u8>,
}

impl<'a> MockPacketStream {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            recv_enc: PacketEncoder::new(),
            pending_recv: Vec::new(),
            send_dec: PacketDecoder::new(),
            flushed_sent: Vec::new(),
        }
    }

    /// Clears the sent packets.
    fn clear_sent(&mut self) {
        self.flushed_sent.clear();
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
    #[allow(dead_code)]
    pub(crate) fn inject_recv<P>(&mut self, packet: P)
    where
        P: EncodePacket,
    {
        self.recv_enc
            .append_packet(&packet)
            .expect("failed to encode injected packet");
        let bytes = self.recv_enc.take();
        self.pending_recv.extend_from_slice(bytes.as_ref());
    }

    /// Collects all the packets that have been sent so assertions can be made
    /// on what the server sent in unit tests.
    #[allow(dead_code)]
    pub fn collect_sent(&'a mut self) -> anyhow::Result<Vec<S2cPlayPacket<'a>>> {
        let bytes = BytesMut::from(self.flushed_sent.as_slice());
        self.send_dec.queue_bytes(bytes);
        let mut packets = Vec::new();
        while let Ok(packet) = self.send_dec.try_next_packet::<S2cPlayPacket<'a>>() {
            if let Some(packet) = packet {
                packets.push(packet);
            } else {
                break;
            }
        }
        Ok(packets)
    }
}

impl PacketStream for MockPacketStream {
    fn try_recv(&mut self) -> Result<BytesMut, TryRecvError> {
        let bytes = BytesMut::from(self.pending_recv.as_slice());
        self.pending_recv.clear();
        Ok(bytes)
    }

    fn try_send(&mut self, bytes: BytesMut) -> Result<(), TrySendError> {
        self.flushed_sent.extend_from_slice(bytes.as_ref());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use valence_protocol::packets::c2s::play::KeepAliveC2s;
    use valence_protocol::packets::s2c::play::KeepAliveS2c;

    use super::*;

    #[test]
    fn test_mock_stream_read() {
        let stream = Arc::new(Mutex::new(MockPacketStream::new()));
        let mut streamer = PacketStreamer::new(stream, PacketEncoder::new(), PacketDecoder::new());
        let packet = KeepAliveC2s { id: 0xdeadbeef };
        streamer.try_send(&packet).unwrap();
        let packet_out = streamer.try_recv::<KeepAliveC2s>().unwrap().unwrap();
        assert_eq!(packet.id, packet_out.id);
    }

    #[test]
    fn test_mock_stream_assert_sent() {
        let stream = Arc::new(Mutex::new(MockPacketStream::new()));
        let mut streamer =
            PacketStreamer::new(stream.clone(), PacketEncoder::new(), PacketDecoder::new());
        let packet = KeepAliveS2c { id: 0xdeadbeef };
        streamer.try_send(&packet).unwrap();
        let mut s = stream.lock().unwrap();
        let packets_out = s.collect_sent().unwrap();
        let S2cPlayPacket::KeepAliveS2c(packet_out) = packets_out[0] else {
            assert!(false);
            return;
        };
        assert_eq!(packet.id, packet_out.id);
    }
}
