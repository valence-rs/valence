use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;
use valence_ident::ident;
use valence_network::NetworkPlugin;
use valence_registry::{BiomeRegistry, DimensionTypeRegistry};
use valence_server::client::ClientBundleArgs;
use valence_server::keepalive::KeepaliveSettings;
use valence_server::layer::CombinedLayerBundle;
use valence_server::protocol::decode::PacketFrame;
use valence_server::protocol::packets::play::{PlayerPositionLookS2c, TeleportConfirmC2s};
use valence_server::protocol::{Decode, Encode, Packet, PacketDecoder, PacketEncoder, VarInt};
use valence_server::{Server, ServerSettings};

use crate::client::{ClientBundle, ClientConnection, ReceivedPacket};
use crate::DefaultPlugins;

pub struct ScenarioSingleClient {
    /// The new bevy application.
    pub app: App,
    /// Entity handle for the single client.
    pub client: Entity,
    /// Helper for sending and receiving packets from the mock client.
    pub helper: MockClientHelper,
    /// Entity with [`CombinedLayerBundle`] components.
    pub layer: Entity,
}

impl ScenarioSingleClient {
    /// Sets up Valence with a single mock client and dimension+entity layer.
    /// The client is configured to be placed within the layer.
    ///
    /// Reduces boilerplate in unit tests.
    pub fn new() -> Self {
        let mut app = App::new();

        app.insert_resource(KeepaliveSettings {
            period: Duration::MAX,
        })
        .insert_resource(ServerSettings {
            compression_threshold: Default::default(),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins.build().disable::<NetworkPlugin>());

        app.update(); // Initialize plugins.

        let layer = app
            .world
            .spawn(CombinedLayerBundle::new(
                Default::default(),
                app.world.resource::<DimensionTypeRegistry>(),
                app.world.resource::<BiomeRegistry>(),
                app.world.resource::<Server>(),
            ))
            .id();

        let (mut client, helper) = create_mock_client("test");
        client.player.layer.0 = layer;
        client.visible_layers.insert(layer);
        let client = app.world.spawn(client).id();

        ScenarioSingleClient {
            app,
            client,
            helper,
            layer,
        }
    }
}

impl Default for ScenarioSingleClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a mock client bundle that can be used for unit testing.
///
/// Returns the client, and a helper to inject packets as if the client sent
/// them and receive packets as if the client received them.
pub fn create_mock_client(name: impl Into<String>) -> (ClientBundle, MockClientHelper) {
    let conn = MockClientConnection::new();

    let bundle = ClientBundle::new(ClientBundleArgs {
        username: name.into(),
        uuid: Uuid::from_bytes(rand::random()),
        ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
        properties: Default::default(),
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
pub struct MockClientConnection {
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
    fn inject_send(&self, mut bytes: BytesMut) {
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

    fn take_received(&self) -> BytesMut {
        self.inner.lock().unwrap().send_buf.split()
    }

    fn clear_received(&self) {
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

impl Default for MockClientConnection {
    fn default() -> Self {
        Self::new()
    }
}

/// Contains the mocked client connection and helper methods to inject packets
/// and read packets from the send stream.
pub struct MockClientHelper {
    conn: MockClientConnection,
    dec: PacketDecoder,
    scratch: BytesMut,
}

impl MockClientHelper {
    pub fn new(conn: MockClientConnection) -> Self {
        Self {
            conn,
            dec: PacketDecoder::new(),
            scratch: BytesMut::new(),
        }
    }

    /// Inject a packet to be treated as a packet inbound to the server. Panics
    /// if the packet cannot be sent.
    #[track_caller]
    pub fn send<P>(&mut self, packet: &P)
    where
        P: Packet + Encode,
    {
        packet
            .encode_with_id((&mut self.scratch).writer())
            .expect("failed to encode packet");

        self.conn.inject_send(self.scratch.split());
    }

    /// Collect all packets that have been received by the client.
    #[track_caller]
    pub fn collect_received(&mut self) -> PacketFrames {
        self.dec.queue_bytes(self.conn.take_received());

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

    pub fn clear_received(&mut self) {
        self.conn.clear_received();
    }

    pub fn confirm_initial_pending_teleports(&mut self) {
        let mut counter = 0;

        for pkt in self.collect_received().0 {
            if pkt.id == PlayerPositionLookS2c::ID {
                pkt.decode::<PlayerPositionLookS2c>().unwrap();

                self.send(&TeleportConfirmC2s {
                    teleport_id: counter.into(),
                });

                counter += 1;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PacketFrames(pub Vec<PacketFrame>);

impl PacketFrames {
    #[track_caller]
    pub fn assert_count<P: Packet>(&self, expected_count: usize) {
        let actual_count = self.0.iter().filter(|f| f.id == P::ID).count();

        if expected_count != actual_count {
            panic!(
                "unexpected packet count for {} (expected {expected_count}, got {actual_count})",
                P::NAME,
            );
        }
    }

    #[track_caller]
    pub fn assert_order<L: PacketList>(&self) {
        let positions: Vec<_> = self
            .0
            .iter()
            .filter_map(|f| L::packets().iter().position(|(id, _)| f.id == *id))
            .collect();

        // TODO: replace with slice::is_sorted when stabilized.
        let is_sorted = positions.windows(2).all(|w| w[0] <= w[1]);

        if !is_sorted {
            panic!(
                "packets out of order (expected {:?}, got {:?})",
                L::packets(),
                self.debug_order::<L>()
            );
        }
    }

    /// Finds the first occurrence of `P` in the packet list and decodes it.
    ///
    /// # Panics
    ///
    /// Panics if the packet was not found or a decoding error occurs.
    #[track_caller]
    pub fn first<'a, P>(&'a self) -> P
    where
        P: Packet + Decode<'a>,
    {
        if let Some(frame) = self.0.iter().find(|p| p.id == P::ID) {
            frame.decode::<P>().unwrap()
        } else {
            panic!("failed to find packet {}", P::NAME)
        }
    }

    pub fn debug_order<L: PacketList>(&self) -> impl std::fmt::Debug {
        self.0
            .iter()
            .filter_map(|f| L::packets().iter().find(|(id, _)| f.id == *id).cloned())
            .collect::<Vec<_>>()
    }
}

pub trait PacketList {
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
