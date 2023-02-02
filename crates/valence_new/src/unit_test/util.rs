use valence_protocol::packets::S2cPlayPacket;
use valence_protocol::{EncodePacket, PacketDecoder, PacketEncoder, Username};

use crate::client::Client;
use crate::server::connection::MockClientConnection;
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
