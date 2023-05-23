use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;
use valence_client::ClientBundleArgs;
use valence_core::packet::decode::{decode_packet, PacketDecoder};
use valence_core::packet::encode::PacketEncoder;
use valence_core::packet::s2c::play::S2cPlayPacket;
use valence_core::packet::var_int::VarInt;
use valence_core::packet::Packet;
use valence_core::protocol::decode::decode_packet;
use valence_core::{ident, CoreSettings, Server};
use valence_entity::Location;
use valence_network::{ConnectionMode, NetworkSettings};

use crate::client::{ClientBundle, ClientConnection, ReceivedPacket};
use crate::instance::Instance;
use crate::DefaultPlugins;

/// Sets up valence with a single mock client. Returns the Entity of the client
/// and the corresponding MockClientHelper.
///
/// Reduces boilerplate in unit tests.
fn scenario_single_client(app: &mut App) -> (Entity, MockClientHelper) {
    app.insert_resource(CoreSettings {
        compression_threshold: None,
        ..Default::default()
    });

    app.insert_resource(NetworkSettings {
        connection_mode: ConnectionMode::Offline,
        ..Default::default()
    });

    app.add_plugins(DefaultPlugins);

    let server = app.world.resource::<Server>();
    let instance = Instance::new_unit_testing(ident!("overworld"), server);
    let instance_ent = app.world.spawn(instance).id();
    let (client, client_helper) = create_mock_client();

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

/// Creates a mock client bundle that can be used for unit testing.
///
/// Returns the client, and a helper to inject packets as if the client sent
/// them and receive packets as if the client received them.
fn create_mock_client() -> (ClientBundle, MockClientHelper) {
    let conn = MockClientConnection::new();

    let bundle = ClientBundle::new(ClientBundleArgs {
        username: "test".into(),
        uuid: Uuid::from_bytes(rand::random()),
        ip: "127.0.0.1".parse().unwrap(),
        properties: vec![],
        conn: Box::new(conn.clone()),
        enc: PacketEncoder::new(),
    });

    let helper = MockClientHelper::new(conn);

    (bundle, helper)
}

/// A mock client connection that can be used for testing.
///
/// Safe to clone, but note that the clone will share the same buffers.
#[derive(Clone)]
struct MockClientConnection {
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
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockClientConnectionInner {
                recv_buf: VecDeque::new(),
                send_buf: BytesMut::new(),
            })),
        }
    }

    /// Injects a (Packet ID + data) frame to be received by the server.
    fn inject_recv(&mut self, mut bytes: BytesMut) {
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

    fn take_sent(&mut self) -> BytesMut {
        self.inner.lock().unwrap().send_buf.split()
    }

    fn clear_sent(&mut self) {
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
struct MockClientHelper {
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
    fn send<'a>(&mut self, packet: &impl Packet<'a>) {
        packet
            .encode_packet((&mut self.scratch).writer())
            .expect("failed to encode packet");

        self.conn.inject_recv(self.scratch.split());
    }

    /// Collect all packets that have been sent to the client.
    fn collect_sent(&mut self) -> Vec<S2cPlayPacket> {
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

    fn clear_sent(&mut self) {
        self.conn.clear_sent();
    }
}

macro_rules! assert_packet_order {
    ($sent_packets:ident, $($packets:pat),+) => {{
        let sent_packets: &Vec<valence_core::packet::s2c::play::S2cPlayPacket> = &$sent_packets;
        let positions = [
            $((sent_packets.iter().position(|p| matches!(p, $packets))),)*
        ];
        assert!(positions.windows(2).all(|w: &[Option<usize>]| w[0] < w[1]));
    }};
}

macro_rules! assert_packet_count {
    ($sent_packets:ident, $count:tt, $packet:pat) => {{
        let sent_packets: &Vec<valence_core::packet::s2c::play::S2cPlayPacket> = &$sent_packets;
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

mod client;
mod example;
mod inventory;
mod weather;
