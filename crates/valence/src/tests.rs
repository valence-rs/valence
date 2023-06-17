use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;
use valence_biome::BiomeRegistry;
use valence_client::ClientBundleArgs;
use valence_core::protocol::decode::{PacketDecoder, PacketFrame};
use valence_core::protocol::encode::PacketEncoder;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{Encode, Packet};
use valence_core::{ident, CoreSettings, Server};
use valence_dimension::DimensionTypeRegistry;
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

    app.update(); // Initialize plugins.

    let instance = Instance::new(
        ident!("overworld"),
        app.world.resource::<DimensionTypeRegistry>(),
        app.world.resource::<BiomeRegistry>(),
        app.world.resource::<Server>(),
    );

    let instance_ent = app.world.spawn(instance).id();

    let (client, client_helper) = create_mock_client();

    let client_ent = app.world.spawn(client).id();

    // Set initial location.
    app.world.get_mut::<Location>(client_ent).unwrap().0 = instance_ent;

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
                body: bytes.freeze(),
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
}

impl MockClientHelper {
    fn new(conn: MockClientConnection) -> Self {
        Self {
            conn,
            dec: PacketDecoder::new(),
            scratch: BytesMut::new(),
        }
    }

    /// Inject a packet to be treated as a packet inbound to the server. Panics
    /// if the packet cannot be sent.
    #[track_caller]
    fn send<P>(&mut self, packet: &P)
    where
        P: Packet + Encode,
    {
        packet
            .encode_with_id((&mut self.scratch).writer())
            .expect("failed to encode packet");

        self.conn.inject_recv(self.scratch.split());
    }

    /// Collect all packets that have been sent to the client.
    #[track_caller]
    fn collect_sent(&mut self) -> PacketFrames {
        self.dec.queue_bytes(self.conn.take_sent());

        let mut res = vec![];

        while let Some(frame) = self
            .dec
            .try_next_packet()
            .expect("failed to decode packet frame")
        {
            res.push(frame);
        }

        PacketFrames(res)
    }

    fn clear_sent(&mut self) {
        self.conn.clear_sent();
    }
}

struct PacketFrames(Vec<PacketFrame>);

impl PacketFrames {
    #[track_caller]
    fn assert_count<P: Packet>(&self, expected_count: usize) {
        let actual_count = self.0.iter().filter(|f| f.id == P::ID).count();

        assert_eq!(
            expected_count,
            actual_count,
            "unexpected packet count for {} (expected {expected_count}, got {actual_count})",
            P::NAME
        );
    }

    #[track_caller]
    fn assert_order<L: PacketList>(&self) {
        let positions: Vec<_> = self
            .0
            .iter()
            .filter_map(|f| L::packets().iter().position(|(id, _)| f.id == *id))
            .collect();

        // TODO: replace with slice::is_sorted.
        let is_sorted = positions.windows(2).all(|w| w[0] <= w[1]);

        assert!(
            is_sorted,
            "packets out of order (expected {:?}, got {:?})",
            L::packets(),
            self.debug::<L>()
        );
    }

    fn debug<L: PacketList>(&self) -> impl std::fmt::Debug {
        self.0
            .iter()
            .map(|f| {
                L::packets()
                    .iter()
                    .find(|(id, _)| f.id == *id)
                    .cloned()
                    .unwrap_or((f.id, "<ignored>"))
            })
            .collect::<Vec<_>>()
    }
}

trait PacketList {
    fn packets() -> &'static [(i32, &'static str)];
}

macro_rules! impl_packet_list {
    ($($ty:ident),*) => {
        impl<$($ty: Packet,)*> PacketList for ($($ty,)*) {
            fn packets() -> &'static [(i32, &'static str)] {
                &[
                    $(
                        (
                            $ty::ID,
                            $ty::NAME
                        ),
                    )*
                ]
            }
        }
    }
}

impl_packet_list!(A);
impl_packet_list!(A, B);
impl_packet_list!(A, B, C);
impl_packet_list!(A, B, C, D);
impl_packet_list!(A, B, C, D, E);
impl_packet_list!(A, B, C, D, E, F);
impl_packet_list!(A, B, C, D, E, F, G);
impl_packet_list!(A, B, C, D, E, F, G, H);
impl_packet_list!(A, B, C, D, E, F, G, H, I);
impl_packet_list!(A, B, C, D, E, F, G, H, I, J);
impl_packet_list!(A, B, C, D, E, F, G, H, I, J, K);

mod client;
mod example;
mod inventory;
mod weather;
mod world_border;
mod world_time;
