use std::sync::{Arc, Mutex};

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
    let client = Client::new(client_info, Box::new(mock_connection), enc, dec);
    (client, MockClientHelper {})
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

pub struct MockClientHelper {
    // stream: Arc<Mutex<MockPacketStream>>,
}
