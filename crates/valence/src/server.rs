use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::ensure;
use bevy_app::prelude::*;
use bevy_app::{ScheduleRunnerPlugin, ScheduleRunnerSettings};
use bevy_ecs::prelude::*;
use flume::{Receiver, Sender};
use rand::rngs::OsRng;
use rsa::{PublicKeyParts, RsaPrivateKey};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::Semaphore;
use uuid::Uuid;
use valence_protocol::types::Property;

use crate::biome::BiomePlugin;
use crate::client::{ClientBundle, ClientPlugin};
use crate::config::{AsyncCallbacks, ConnectionMode, ServerPlugin};
use crate::dimension::DimensionPlugin;
use crate::entity::EntityPlugin;
use crate::event_loop::{EventLoopPlugin, RunEventLoopSet};
use crate::instance::InstancePlugin;
use crate::inventory::InventoryPlugin;
use crate::player_list::PlayerListPlugin;
use crate::prelude::ComponentPlugin;
use crate::registry_codec::RegistryCodecPlugin;
use crate::server::connect::do_accept_loop;
use crate::weather::WeatherPlugin;

mod byte_channel;
mod connect;
pub(crate) mod connection;

use connection::NewClientArgs;

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
    /// Sender for new clients past the login stage.
    new_clients_send: Sender<NewClientArgs>,
    /// Receiver for new clients past the login stage.
    new_clients_recv: Receiver<NewClientArgs>,
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
            let Ok(args) = shared.0.new_clients_recv.try_recv() else {
                break
            };

            world.spawn(ClientBundle::new(args.info, args.conn, args.enc));
        }
    };

    let shared = server.shared.clone();

    // Insert resources.
    app.insert_resource(server);

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

    // Spawn new clients before the event loop starts.
    app.add_system(
        spawn_new_clients
            .in_base_set(CoreSet::PreUpdate)
            .before(RunEventLoopSet),
    );

    app.add_system(increment_tick_counter.in_base_set(CoreSet::Last));

    // Add internal plugins.
    app.add_plugin(EventLoopPlugin)
        .add_plugin(RegistryCodecPlugin)
        .add_plugin(BiomePlugin)
        .add_plugin(DimensionPlugin)
        .add_plugin(ComponentPlugin)
        .add_plugin(ClientPlugin)
        .add_plugin(EntityPlugin)
        .add_plugin(InstancePlugin)
        .add_plugin(InventoryPlugin)
        .add_plugin(PlayerListPlugin)
        .add_plugin(WeatherPlugin);

    Ok(())
}

fn increment_tick_counter(mut server: ResMut<Server>) {
    server.current_tick += 1;
}
