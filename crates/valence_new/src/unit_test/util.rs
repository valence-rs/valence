use tokio::sync::OwnedSemaphorePermit;
use valence_protocol::Username;

use crate::client::Client;
use crate::server::byte_channel::{ByteReceiver, ByteSender};
use crate::server::{NewClientInfo, PlayPacketReceiver, PlayPacketSender};

/// Creates a mock client that can be used for unit testing.
///
/// Returns the client, a `ByteSender` to inject packets to be read and acted
/// upon by the server, and a `ByteReceiver` to receive packets and make
/// assertions about what the server sent.
pub fn create_mock_client(
    permit: OwnedSemaphorePermit,
    client_info: NewClientInfo,
) -> (Client, ByteSender, ByteReceiver) {
    let (pkt_send, recv) = PlayPacketSender::new_injectable();
    let (pkt_recv, send) = PlayPacketReceiver::new_injectable();

    let client = Client::new(pkt_send, pkt_recv, permit, client_info);
    (client, send, recv)
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
