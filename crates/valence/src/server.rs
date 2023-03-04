use std::iter::FusedIterator;
use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::ensure;
use bevy_app::prelude::*;
use bevy_app::{ScheduleRunnerPlugin, ScheduleRunnerSettings};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use flume::{Receiver, Sender};
use rand::rngs::OsRng;
use rsa::{PublicKeyParts, RsaPrivateKey};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::Semaphore;
use uuid::Uuid;
use valence_nbt::{compound, Compound, List};
use valence_protocol::ident;
use valence_protocol::types::Property;

use crate::biome::{validate_biomes, Biome, BiomeId};
use crate::client::event::{register_client_events, run_event_loop};
use crate::client::{update_clients, Client};
use crate::component::Despawned;
use crate::config::{AsyncCallbacks, ConnectionMode, ServerPlugin};
use crate::dimension::{validate_dimensions, Dimension, DimensionId};
use crate::entity::{
    check_mcentity_invariants, deinit_despawned_mcentities, init_mcentities, update_mcentities,
    McEntityManager,
};
use crate::instance::{
    check_instance_invariants, update_instances_post_client, update_instances_pre_client, Instance,
};
use crate::inventory::update_inventories;
use crate::player_list::{update_player_list, PlayerList};
use crate::prelude::{Inventory, InventoryKind};
use crate::server::connect::do_accept_loop;

mod byte_channel;
mod connect;
pub(crate) mod connection;

/// Contains global server state accessible as a [`Resource`].
#[derive(Resource)]
pub struct Server {
    /// Incremented on every tick.
    current_tick: i64,
    shared: SharedServer,
}

impl Deref for Server {
    type Target = SharedServer;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

impl Server {
    /// Provides a reference to the [`SharedServer`].
    pub fn shared(&self) -> &SharedServer {
        &self.shared
    }

    /// Returns the number of ticks that have elapsed since the server began.
    pub fn current_tick(&self) -> i64 {
        self.current_tick
    }
}

/// The subset of global server state which can be shared between threads.
///
/// `SharedServer`s are internally refcounted and are inexpensive to clone.
#[derive(Clone)]
pub struct SharedServer(Arc<SharedServerInner>);

struct SharedServerInner {
    address: SocketAddr,
    tps: i64,
    connection_mode: ConnectionMode,
    compression_threshold: Option<u32>,
    max_connections: usize,
    incoming_capacity: usize,
    outgoing_capacity: usize,
    /// The tokio handle used by the server.
    tokio_handle: Handle,
    /// Holding a runtime handle is not enough to keep tokio working. We need
    /// to store the runtime here so we don't drop it.
    _tokio_runtime: Option<Runtime>,
    dimensions: Arc<[Dimension]>,
    biomes: Arc<[Biome]>,
    /// Contains info about dimensions, biomes, and chats.
    /// Sent to all clients when joining.
    registry_codec: Compound,
    /// Sender for new clients past the login stage.
    new_clients_send: Sender<Client>,
    /// Receiver for new clients past the login stage.
    new_clients_recv: Receiver<Client>,
    /// A semaphore used to limit the number of simultaneous connections to the
    /// server. Closing this semaphore stops new connections.
    connection_sema: Arc<Semaphore>,
    /// The RSA keypair used for encryption with clients.
    rsa_key: RsaPrivateKey,
    /// The public part of `rsa_key` encoded in DER, which is an ASN.1 format.
    /// This is sent to clients during the authentication process.
    public_key_der: Box<[u8]>,
    /// For session server requests.
    http_client: reqwest::Client,
}

impl SharedServer {
    /// Creates a new [`Instance`] component with the given dimension.
    #[must_use]
    pub fn new_instance(&self, dimension: DimensionId) -> Instance {
        Instance::new(dimension, self)
    }

    /// Gets the socket address this server is bound to.
    pub fn address(&self) -> SocketAddr {
        self.0.address
    }

    /// Gets the configured ticks per second of this server.
    pub fn tps(&self) -> i64 {
        self.0.tps
    }

    /// Gets the connection mode of the server.
    pub fn connection_mode(&self) -> &ConnectionMode {
        &self.0.connection_mode
    }

    /// Gets the compression threshold for packets. `None` indicates no
    /// compression.
    pub fn compression_threshold(&self) -> Option<u32> {
        self.0.compression_threshold
    }

    /// Gets the maximum number of connections allowed to the server at once.
    pub fn max_connections(&self) -> usize {
        self.0.max_connections
    }

    /// Gets the configured incoming capacity.
    pub fn incoming_capacity(&self) -> usize {
        self.0.incoming_capacity
    }

    /// Gets the configured outgoing incoming capacity.
    pub fn outgoing_capacity(&self) -> usize {
        self.0.outgoing_capacity
    }

    /// Gets a handle to the tokio instance this server is using.
    pub fn tokio_handle(&self) -> &Handle {
        &self.0.tokio_handle
    }

    /// Obtains a [`Dimension`] by using its corresponding [`DimensionId`].
    #[track_caller]
    pub fn dimension(&self, id: DimensionId) -> &Dimension {
        self.0
            .dimensions
            .get(id.0 as usize)
            .expect("invalid dimension ID")
    }

    /// Returns an iterator over all added dimensions and their associated
    /// [`DimensionId`].
    pub fn dimensions(&self) -> impl FusedIterator<Item = (DimensionId, &Dimension)> + Clone {
        self.0
            .dimensions
            .iter()
            .enumerate()
            .map(|(i, d)| (DimensionId(i as u16), d))
    }

    /// Obtains a [`Biome`] by using its corresponding [`BiomeId`].
    #[track_caller]
    pub fn biome(&self, id: BiomeId) -> &Biome {
        self.0.biomes.get(id.0 as usize).expect("invalid biome ID")
    }

    /// Returns an iterator over all added biomes and their associated
    /// [`BiomeId`] in ascending order.
    pub fn biomes(
        &self,
    ) -> impl ExactSizeIterator<Item = (BiomeId, &Biome)> + DoubleEndedIterator + FusedIterator + Clone
    {
        self.0
            .biomes
            .iter()
            .enumerate()
            .map(|(i, b)| (BiomeId(i as u16), b))
    }

    pub(crate) fn registry_codec(&self) -> &Compound {
        &self.0.registry_codec
    }
}

/// Contains information about a new client joining the server.
#[non_exhaustive]
pub struct NewClientInfo {
    /// The username of the new client.
    pub username: String,
    /// The UUID of the new client.
    pub uuid: Uuid,
    /// The remote address of the new client.
    pub ip: IpAddr,
    /// The client's properties from the game profile. Typically contains a
    /// `textures` property with the skin and cape of the player.
    pub properties: Vec<Property>,
}

pub fn build_plugin(
    plugin: &ServerPlugin<impl AsyncCallbacks>,
    app: &mut App,
) -> anyhow::Result<()> {
    ensure!(
        plugin.tps > 0,
        "configured tick rate must be greater than zero"
    );
    ensure!(
        plugin.incoming_capacity > 0,
        "configured incoming packet capacity must be nonzero"
    );
    ensure!(
        plugin.outgoing_capacity > 0,
        "configured outgoing packet capacity must be nonzero"
    );

    let rsa_key = RsaPrivateKey::new(&mut OsRng, 1024)?;

    let public_key_der =
        rsa_der::public_key_to_der(&rsa_key.n().to_bytes_be(), &rsa_key.e().to_bytes_be())
            .into_boxed_slice();

    let runtime = if plugin.tokio_handle.is_none() {
        Some(Runtime::new()?)
    } else {
        None
    };

    let tokio_handle = match &runtime {
        Some(rt) => rt.handle().clone(),
        None => plugin.tokio_handle.clone().unwrap(),
    };

    validate_dimensions(&plugin.dimensions)?;
    validate_biomes(&plugin.biomes)?;

    let registry_codec = make_registry_codec(&plugin.dimensions, &plugin.biomes);

    let (new_clients_send, new_clients_recv) = flume::bounded(64);

    let shared = SharedServer(Arc::new(SharedServerInner {
        address: plugin.address,
        tps: plugin.tps,
        connection_mode: plugin.connection_mode.clone(),
        compression_threshold: plugin.compression_threshold,
        max_connections: plugin.max_connections,
        incoming_capacity: plugin.incoming_capacity,
        outgoing_capacity: plugin.outgoing_capacity,
        tokio_handle,
        _tokio_runtime: runtime,
        dimensions: plugin.dimensions.clone(),
        biomes: plugin.biomes.clone(),
        registry_codec,
        new_clients_send,
        new_clients_recv,
        connection_sema: Arc::new(Semaphore::new(plugin.max_connections)),
        rsa_key,
        public_key_der,
        http_client: Default::default(),
    }));

    let server = Server {
        current_tick: 0,
        shared,
    };

    let shared = server.shared.clone();
    let callbacks = plugin.callbacks.clone();

    let start_accept_loop = move || {
        let _guard = shared.tokio_handle().enter();

        // Start accepting new connections.
        tokio::spawn(do_accept_loop(shared.clone(), callbacks.clone()));
    };

    let shared = server.shared.clone();

    // System to spawn new clients.
    let spawn_new_clients = move |world: &mut World| {
        for _ in 0..shared.0.new_clients_recv.len() {
            let Ok(client) = shared.0.new_clients_recv.try_recv() else {
                break
            };

            world.spawn((client, Inventory::new(InventoryKind::Player)));
        }
    };

    let shared = server.shared.clone();

    // Insert resources.
    app.insert_resource(server)
        .insert_resource(McEntityManager::new())
        .insert_resource(PlayerList::new());
    register_client_events(&mut app.world);

    // Add the event loop schedule.
    let mut event_loop = Schedule::new();
    event_loop.configure_set(EventLoopSet);
    event_loop.set_default_base_set(EventLoopSet);

    app.add_schedule(EventLoopSchedule, event_loop);

    // Make the app loop forever at the configured TPS.
    {
        let tick_period = Duration::from_secs_f64((shared.tps() as f64).recip());

        app.insert_resource(ScheduleRunnerSettings::run_loop(tick_period))
            .add_plugin(ScheduleRunnerPlugin);
    }

    // Start accepting connections in `PostStartup` to allow user startup code to
    // run first.
    app.add_system(
        start_accept_loop
            .in_schedule(CoreSchedule::Startup)
            .in_base_set(StartupSet::PostStartup),
    );

    // Add `CoreSet:::PreUpdate` systems.
    app.add_systems(
        (spawn_new_clients.before(run_event_loop), run_event_loop).in_base_set(CoreSet::PreUpdate),
    );

    // Add internal valence systems that run after `CoreSet::Update`.
    app.add_systems(
        (
            init_entities,
            check_instance_invariants,
            update_player_list.before(update_instances_pre_client),
            update_instances_pre_client.after(init_entities),
            update_clients.after(update_instances_pre_client),
            update_instances_post_client.after(update_clients),
            deinit_despawned_entities.after(update_instances_post_client),
            despawn_marked_entities.after(deinit_despawned_entities),
            update_entities.after(despawn_marked_entities),
        )
            .in_base_set(CoreSet::PostUpdate),
    )
    .add_systems(
        update_inventories()
            .in_base_set(CoreSet::PostUpdate)
            .before(init_entities),
    )
    .add_system(increment_tick_counter.in_base_set(CoreSet::Last));

    Ok(())
}

/// The [`ScheduleLabel`] for the event loop [`Schedule`].
#[derive(ScheduleLabel, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct EventLoopSchedule;

/// The default base set for the event loop [`Schedule`].
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct EventLoopSet;

/// Despawns all the entities marked as despawned with the [`Despawned`]
/// component.
fn despawn_marked_entities(mut commands: Commands, entities: Query<Entity, With<Despawned>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

fn increment_tick_counter(mut server: ResMut<Server>) {
    server.current_tick += 1;
}

fn make_registry_codec(dimensions: &[Dimension], biomes: &[Biome]) -> Compound {
    let dimensions = dimensions
        .iter()
        .enumerate()
        .map(|(id, dim)| dim.to_dimension_registry_item(id as i32))
        .collect();

    let biomes = biomes
        .iter()
        .enumerate()
        .map(|(id, biome)| biome.to_biome_registry_item(id as i32))
        .collect();

    compound! {
        ident!("dimension_type") => compound! {
            "type" => ident!("dimension_type"),
            "value" => List::Compound(dimensions),
        },
        ident!("worldgen/biome") => compound! {
            "type" => ident!("worldgen/biome"),
            "value" => List::Compound(biomes),
        },
        ident!("chat_type") => compound! {
            "type" => ident!("chat_type"),
            "value" => List::Compound(vec![]),
        },
    }
}
