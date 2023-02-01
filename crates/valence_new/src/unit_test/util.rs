use std::sync::{Arc, Mutex};

use tokio::sync::OwnedSemaphorePermit;
use valence_protocol::{EncodePacket, PacketDecoder, PacketEncoder, Username};

use crate::client::Client;
use crate::packet_stream::{MockPacketStream, PacketStream, PacketStreamer};
use crate::server::NewClientInfo;

/// Creates a mock client that can be used for unit testing.
///
/// Returns the client, and a helper to inject packets as if the client sent
/// them and receive packets as if the client received them.
pub fn create_mock_client(
    permit: OwnedSemaphorePermit,
    client_info: NewClientInfo,
) -> (Client, MockClientHelper) {
    let mock_stream = Arc::new(Mutex::new(MockPacketStream::new()));
    let streamer = PacketStreamer::new(
        mock_stream.clone(),
        PacketEncoder::new(),
        PacketDecoder::new(),
    );
    let client = Client::new(streamer, permit, client_info);
    (client, MockClientHelper::new(mock_stream))
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
    stream: Arc<Mutex<MockPacketStream>>,
}

/// Contains the mocked packet stream and helper methods to inject packets and
/// read packets from the send stream.
impl MockClientHelper {
    fn new(stream: Arc<Mutex<MockPacketStream>>) -> Self {
        Self { stream }
    }

    /// Inject a packet to be parsed by the server. Panics if the packet cannot
    /// be sent.
    pub fn send_packet(&mut self, packet: impl EncodePacket) {
        self.stream.lock().unwrap().inject_recv(packet);
    }

    pub(crate) fn inner_stream(&self) -> Arc<Mutex<MockPacketStream>> {
        self.stream.clone()
    }
}
