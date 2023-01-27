use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::ensure;
use bevy_app::prelude::*;
use bevy_app::AppExit;
use bevy_ecs::event::ManualEventReader;
use bevy_ecs::prelude::*;
use flume::{Receiver, Sender};
pub use packet_manager::{PlayPacketReceiver, PlayPacketSender};
use rand::rngs::OsRng;
use rsa::{PublicKeyParts, RsaPrivateKey};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;
use valence_protocol::types::Property;
use valence_protocol::Username;

use crate::config::{AsyncCallbacks, ConnectionMode, ServerPlugin};
use crate::server::connect::do_accept_loop;

pub(crate) mod byte_channel;
mod connect;
mod packet_manager;

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
    /// The instant the server was started.
    start_instant: Instant,
    /// Sender for new clients past the login stage.
    new_clients_send: Sender<NewClientMessage>,
    /// Receiver for new clients past the login stage.
    new_clients_recv: Receiver<NewClientMessage>,
    /// A semaphore used to limit the number of simultaneous connections to the
    /// server. Closing this semaphore stops new connections.
    connection_sema: Arc<Semaphore>,
    /// The result that will be returned when the server is shut down.
    shutdown_result: Mutex<Option<anyhow::Result<()>>>,
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

    /// Returns the instant the server was started.
    pub fn start_instant(&self) -> Instant {
        self.0.start_instant
    }

    /// Immediately stops new connections to the server and initiates server
    /// shutdown. The given result is returned through [`start_server`].
    ///
    /// You may want to disconnect all players with a message prior to calling
    /// this function.
    pub fn shutdown<E>(&self, res: Result<(), E>)
    where
        E: Into<anyhow::Error>,
    {
        self.0.connection_sema.close();
        *self.0.shutdown_result.lock().unwrap() = Some(res.map_err(|e| e.into()));
    }
}

/// Contains information about a new client joining the server.
#[non_exhaustive]
pub struct NewClientInfo {
    /// The username of the new client.
    pub username: Username<String>,
    /// The UUID of the new client.
    pub uuid: Uuid,
    /// The remote address of the new client.
    pub ip: IpAddr,
    /// The client's properties from the game profile. Typically contains a
    /// `textures` property with the skin and cape of the player.
    pub properties: Vec<Property>,
}

struct NewClientMessage {
    info: NewClientInfo,
    send: PlayPacketSender,
    recv: PlayPacketReceiver,
    permit: OwnedSemaphorePermit,
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
        start_instant: Instant::now(),
        new_clients_send,
        new_clients_recv,
        connection_sema: Arc::new(Semaphore::new(plugin.max_connections)),
        shutdown_result: Mutex::new(None),
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

    // Exclusive system to spawn new clients. Should run before everything else.
    let spawn_new_clients = move |world: &mut World| {
        for _ in 0..shared.0.new_clients_recv.len() {
            let Ok(msg) = shared.0.new_clients_recv.try_recv() else {
                break
            };

            world.spawn((
                Client::new(msg.send, msg.recv, msg.permit, msg.info),
                Inventory::new(InventoryKind::Player),
            ));
        }
    };

    let shared = server.shared.clone();

    // Start accepting connections in PostStartup to allow user startup code to run
    // first.
    app.add_startup_system_to_stage(StartupStage::PostStartup, start_accept_loop);

    // Insert resources.
    app.insert_resource(server);

    // Add core systems. User code is expected to run in `CoreStage::Update`, so
    // we'll add our systems before and after that.

    app.add_system_to_stage(CoreStage::PreUpdate, spawn_new_clients);

    let tick_duration = Duration::from_secs_f64((shared.tps() as f64).recip());

    // Overwrite the app's runner.
    app.set_runner(move |mut app: App| {
        let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();

        loop {
            let tick_start = Instant::now();

            // Stop the server if there was an AppExit event.
            if let Some(app_exit_events) = app.world.get_resource_mut::<Events<AppExit>>() {
                if app_exit_event_reader
                    .iter(&app_exit_events)
                    .last()
                    .is_some()
                {
                    return;
                }
            }

            // Run the scheduled stages.
            app.update();

            // Clear tracker state so that change detection works correctly.
            // TODO: is this needed?
            app.world.clear_trackers();

            // Sleep until the next tick.
            thread::sleep(tick_duration.saturating_sub(tick_start.elapsed()));
        }
    });

    Ok(())
}
