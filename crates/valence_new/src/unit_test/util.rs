use bytes::BytesMut;
use tokio::sync::OwnedSemaphorePermit;
use valence_protocol::{EncodePacket, Username};

use crate::client::Client;
use crate::server::byte_channel::{ByteReceiver, ByteSender};
use crate::server::{NewClientInfo, PlayPacketReceiver, PlayPacketSender};

/// Creates a mock client that can be used for unit testing.
///
/// Returns the client, and a helper to inject packets as if the client sent
/// them and receive packets as if the client received them.
pub fn create_mock_client(
    permit: OwnedSemaphorePermit,
    client_info: NewClientInfo,
) -> (Client, MockClientHelper) {
    let (pkt_send, recv) = PlayPacketSender::new_injectable();
    let (pkt_recv, send) = PlayPacketReceiver::new_injectable();

    let client = Client::new(pkt_send, pkt_recv, permit, client_info);
    (client, MockClientHelper::new(send, recv))
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
    /// Used to pretend the client sent bytes.
    send: ByteSender,
    /// Used to pretend the client received bytes.
    recv: ByteReceiver,
}

/// Has a `ByteSender` to inject packets to be read and acted
/// upon by the server, and a `ByteReceiver` to receive packets and make
/// assertions about what the server sent.
impl MockClientHelper {
    fn new(send: ByteSender, recv: ByteReceiver) -> Self {
        Self { send, recv }
    }

    /// Inject a packet to be parsed by the server. Panics if the packet cannot
    /// be sent.
    pub fn send_packet(&mut self, packet: impl EncodePacket) {
        let mut buffer = Vec::<u8>::new();
        valence_protocol::encode_packet(&mut buffer, &packet).expect("Failed to encode packet");
        self.send
            .try_send(BytesMut::from(buffer.as_slice()))
            .expect("Failed to send packet");
    }
}
