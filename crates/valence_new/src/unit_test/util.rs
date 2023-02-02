use std::sync::{Arc, Mutex};

use bytes::BytesMut;
use valence_protocol::packets::S2cPlayPacket;
use valence_protocol::{EncodePacket, PacketDecoder, PacketEncoder, Username};

use crate::client::{Client, ClientConnection};
use crate::server::NewClientInfo;

/// Creates a mock client that can be used for unit testing.
///
/// Returns the client, and a helper to inject packets as if the client sent
/// them and receive packets as if the client received them.
pub fn create_mock_client(client_info: NewClientInfo) -> (Client, MockClientHelper) {
    let mock_connection = MockClientConnection::new();
    let enc = PacketEncoder::new();
    let dec = PacketDecoder::new();
    let client = Client::new(client_info, Box::new(mock_connection.clone()), enc, dec);
    (client, MockClientHelper::new(mock_connection))
}

/// Creates a `NewClientInfo` with the given username and a random UUID.
/// Panics if the username is invalid.
pub fn gen_client_info(username: &str) -> NewClientInfo {
    NewClientInfo {
        username: Username::new(username.to_owned()).unwrap(),
        uuid: uuid::Uuid::new_v4(),
        ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        properties: vec![],
    }
}

/// A mock client connection that can be used for testing.
///
/// Safe to clone, but note that the clone will share the same buffers.
#[derive(Clone)]
pub(crate) struct MockClientConnection {
    buffers: Arc<Mutex<MockClientBuffers>>,
}

struct MockClientBuffers {
    /// The queue of packets to receive from the client to be processed by the
    /// server.
    recv_buf: BytesMut,
    /// The queue of packets to send from the server to the client.
    send_buf: BytesMut,
}

impl MockClientConnection {
    pub fn new() -> Self {
        Self {
            buffers: Arc::new(Mutex::new(MockClientBuffers {
                recv_buf: BytesMut::new(),
                send_buf: BytesMut::new(),
            })),
        }
    }

    pub fn inject_recv(&mut self, bytes: BytesMut) {
        self.buffers.lock().unwrap().recv_buf.extend(bytes);
    }

    pub fn take_sent(&mut self) -> BytesMut {
        self.buffers.lock().unwrap().send_buf.split()
    }

    pub fn clear_sent(&mut self) {
        self.buffers.lock().unwrap().send_buf.clear();
    }
}

impl ClientConnection for MockClientConnection {
    fn try_send(&mut self, bytes: BytesMut) -> anyhow::Result<()> {
        self.buffers.lock().unwrap().send_buf.extend(bytes);
        Ok(())
    }

    fn try_recv(&mut self) -> anyhow::Result<BytesMut> {
        Ok(self.buffers.lock().unwrap().recv_buf.split())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_client_recv() -> anyhow::Result<()> {
        let msg = 0xdeadbeefu32.to_be_bytes();
        let b = BytesMut::from(&msg[..]);
        let mut client = MockClientConnection::new();
        client.inject_recv(b);
        let b = client.try_recv()?;
        assert_eq!(b, BytesMut::from(&msg[..]));

        Ok(())
    }

    #[test]
    fn test_mock_client_send() -> anyhow::Result<()> {
        let msg = 0xdeadbeefu32.to_be_bytes();
        let b = BytesMut::from(&msg[..]);
        let mut client = MockClientConnection::new();
        client.try_send(b)?;
        let b = client.take_sent();
        assert_eq!(b, BytesMut::from(&msg[..]));

        Ok(())
    }
}

/// Contains the mocked client connection and helper methods to inject packets
/// and read packets from the send stream.
pub struct MockClientHelper {
    conn: MockClientConnection,
    enc: PacketEncoder,
    dec: PacketDecoder,
}

impl MockClientHelper {
    fn new(conn: MockClientConnection) -> Self {
        Self {
            conn,
            enc: PacketEncoder::new(),
            dec: PacketDecoder::new(),
        }
    }

    /// Inject a packet to be treated as a packet inbound to the server. Panics
    /// if the packet cannot be sent.
    pub fn send(&mut self, packet: &impl EncodePacket) {
        self.enc
            .append_packet(packet)
            .expect("failed to encode packet");
        self.conn.inject_recv(self.enc.take());
    }

    /// Collect all packets that have been sent to the client.
    pub fn collect_sent<'a>(&'a mut self) -> anyhow::Result<Vec<S2cPlayPacket<'a>>> {
        self.dec.queue_bytes(self.conn.take_sent());

        self.dec.collect_into_vec::<S2cPlayPacket<'a>>()
    }

    pub fn clear_sent(&mut self) {
        self.conn.clear_sent();
    }
}
