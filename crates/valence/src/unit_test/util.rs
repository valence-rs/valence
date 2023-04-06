use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bytes::{Buf, BufMut, BytesMut};
use valence_protocol::decoder::{decode_packet, PacketDecoder};
use valence_protocol::encoder::PacketEncoder;
use valence_protocol::packet::S2cPlayPacket;
use valence_protocol::var_int::VarInt;
use valence_protocol::{ident, Packet};

use crate::client::{ClientBundle, ClientConnection, ReceivedPacket};
use crate::component::Location;
use crate::config::{ConnectionMode, ServerPlugin};
use crate::instance::Instance;
use crate::server::{NewClientInfo, Server};

/// Creates a mock client bundle that can be used for unit testing.
///
/// Returns the client, and a helper to inject packets as if the client sent
/// them and receive packets as if the client received them.
pub(crate) fn create_mock_client(client_info: NewClientInfo) -> (ClientBundle, MockClientHelper) {
    let mock_connection = MockClientConnection::new();
    let enc = PacketEncoder::new();
    let bundle = ClientBundle::new(client_info, Box::new(mock_connection.clone()), enc);

    (bundle, MockClientHelper::new(mock_connection))
}

/// Creates a `NewClientInfo` with the given username and a random UUID.
pub fn gen_client_info(username: impl Into<String>) -> NewClientInfo {
    NewClientInfo {
        username: username.into(),
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
    inner: Arc<Mutex<MockClientConnectionInner>>,
}

struct MockClientConnectionInner {
    /// The queue of packets to receive from the client to be processed by the
    /// server.
    recv_buf: VecDeque<ReceivedPacket>,
    /// The queue of packets to send from the server to the client.
    send_buf: BytesMut,
}

impl MockClientConnection {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockClientConnectionInner {
                recv_buf: VecDeque::new(),
                send_buf: BytesMut::new(),
            })),
        }
    }

    /// Injects a (Packet ID + data) frame to be received by the server.
    pub fn inject_recv(&mut self, mut bytes: BytesMut) {
        let id = VarInt::decode_partial((&mut bytes).reader()).expect("failed to decode packet ID");

        self.inner
            .lock()
            .unwrap()
            .recv_buf
            .push_back(ReceivedPacket {
                timestamp: Instant::now(),
                id,
                data: bytes.freeze(),
            });
    }

    pub fn take_sent(&mut self) -> BytesMut {
        self.inner.lock().unwrap().send_buf.split()
    }

    pub fn clear_sent(&mut self) {
        self.inner.lock().unwrap().send_buf.clear();
    }
}

impl ClientConnection for MockClientConnection {
    fn try_send(&mut self, bytes: BytesMut) -> anyhow::Result<()> {
        self.inner.lock().unwrap().send_buf.unsplit(bytes);
        Ok(())
    }

    fn try_recv(&mut self) -> anyhow::Result<Option<ReceivedPacket>> {
        Ok(self.inner.lock().unwrap().recv_buf.pop_front())
    }

    fn len(&self) -> usize {
        self.inner.lock().unwrap().recv_buf.len()
    }
}

/// Contains the mocked client connection and helper methods to inject packets
/// and read packets from the send stream.
pub struct MockClientHelper {
    conn: MockClientConnection,
    dec: PacketDecoder,
    scratch: BytesMut,
    collected_frames: Vec<BytesMut>,
}

impl MockClientHelper {
    fn new(conn: MockClientConnection) -> Self {
        Self {
            conn,
            dec: PacketDecoder::new(),
            scratch: BytesMut::new(),
            collected_frames: vec![],
        }
    }

    /// Inject a packet to be treated as a packet inbound to the server. Panics
    /// if the packet cannot be sent.
    pub fn send<'a>(&mut self, packet: &impl Packet<'a>) {
        packet
            .encode_packet((&mut self.scratch).writer())
            .expect("failed to encode packet");

        self.conn.inject_recv(self.scratch.split());
    }

    /// Collect all packets that have been sent to the client.
    pub fn collect_sent<'a>(&'a mut self) -> Vec<S2cPlayPacket<'a>> {
        self.dec.queue_bytes(self.conn.take_sent());

        self.collected_frames.clear();

        while let Some(frame) = self
            .dec
            .try_next_packet()
            .expect("failed to decode packet frame")
        {
            self.collected_frames.push(frame);
        }

        self.collected_frames
            .iter()
            .map(|frame| decode_packet(frame).expect("failed to decode packet"))
            .collect()
    }

    pub fn clear_sent(&mut self) {
        self.conn.clear_sent();
    }
}

/// Sets up valence with a single mock client. Returns the Entity of the client
/// and the corresponding MockClientHelper.
///
/// Reduces boilerplate in unit tests.
pub fn scenario_single_client(app: &mut App) -> (Entity, MockClientHelper) {
    app.add_plugin(
        ServerPlugin::new(())
            .with_compression_threshold(None)
            .with_connection_mode(ConnectionMode::Offline),
    );

    let server = app.world.resource::<Server>();
    let instance = Instance::new_unit_testing(ident!("overworld"), server);
    let instance_ent = app.world.spawn(instance).id();
    let (client, client_helper) = create_mock_client(gen_client_info("test"));

    let client_ent = app.world.spawn(client).id();

    // Set initial location.
    app.world.get_mut::<Location>(client_ent).unwrap().0 = instance_ent;

    // Print warnings if there are ambiguities in the schedule.
    app.edit_schedule(CoreSchedule::Main, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            ..Default::default()
        });
    });

    (client_ent, client_helper)
}

#[macro_export]
macro_rules! assert_packet_order {
    ($sent_packets:ident, $($packets:pat),+) => {{
        let sent_packets: &Vec<valence_protocol::packet::S2cPlayPacket> = &$sent_packets;
        let positions = [
            $((sent_packets.iter().position(|p| matches!(p, $packets))),)*
        ];
        assert!(positions.windows(2).all(|w: &[Option<usize>]| w[0] < w[1]));
    }};
}

#[macro_export]
macro_rules! assert_packet_count {
    ($sent_packets:ident, $count:tt, $packet:pat) => {{
        let sent_packets: &Vec<valence_protocol::packet::S2cPlayPacket> = &$sent_packets;
        let count = sent_packets.iter().filter(|p| matches!(p, $packet)).count();
        assert_eq!(
            count,
            $count,
            "expected {} {} packets, got {}\nPackets actually found:\n[\n\t{}\n]\n",
            $count,
            stringify!($packet),
            count,
            sent_packets
                .iter()
                .map(|p| format!("{:?}", p))
                .collect::<Vec<_>>()
                .join(",\n\t")
        );
    }};
}
