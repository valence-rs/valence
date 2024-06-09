#![doc = include_str!("../README.md")]

mod byte_channel;
mod connect;
mod legacy_ping;
mod packet_io;

use std::borrow::Cow;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
pub use async_trait::async_trait;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use connect::do_accept_loop;
pub use connect::HandshakeData;
use flume::{Receiver, Sender};
pub use legacy_ping::{ServerListLegacyPingPayload, ServerListLegacyPingResponse};
use rand::rngs::OsRng;
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde::Serialize;
use tokio::net::UdpSocket;
use tokio::runtime::{Handle, Runtime};
use tokio::sync::Semaphore;
use tokio::time;
use tracing::error;
use uuid::Uuid;
use valence_protocol::text::IntoText;
use valence_server::client::{ClientBundle, ClientBundleArgs, Properties, SpawnClientsSet};
use valence_server::{CompressionThreshold, Server, Text, MINECRAFT_VERSION, PROTOCOL_VERSION};

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        if let Err(e) = build_plugin(app) {
            error!("failed to build network plugin: {e:#}");
        }
    }
}

fn build_plugin(app: &mut App) -> anyhow::Result<()> {
    let threshold = app
        .world
        .get_resource::<Server>()
        .context("missing server resource")?
        .compression_threshold();

    let settings = app
        .world
        .get_resource_or_insert_with(NetworkSettings::default);

    let (new_clients_send, new_clients_recv) = flume::bounded(64);

    let rsa_key = RsaPrivateKey::new(&mut OsRng, 1024)?;

    let public_key_der =
        rsa_der::public_key_to_der(&rsa_key.n().to_bytes_be(), &rsa_key.e().to_bytes_be())
            .into_boxed_slice();

    #[allow(clippy::if_then_some_else_none)]
    let runtime = if settings.tokio_handle.is_none() {
        Some(Runtime::new()?)
    } else {
        None
    };

    let tokio_handle = match &runtime {
        Some(rt) => rt.handle().clone(),
        None => settings.tokio_handle.clone().unwrap(),
    };

    let shared = SharedNetworkState(Arc::new(SharedNetworkStateInner {
        callbacks: settings.callbacks.clone(),
        address: settings.address,
        incoming_byte_limit: settings.incoming_byte_limit,
        outgoing_byte_limit: settings.outgoing_byte_limit,
        connection_sema: Arc::new(Semaphore::new(
            settings.max_connections.min(Semaphore::MAX_PERMITS),
        )),
        player_count: AtomicUsize::new(0),
        max_players: settings.max_players,
        connection_mode: settings.connection_mode.clone(),
        threshold,
        tokio_handle,
        _tokio_runtime: runtime,
        new_clients_send,
        new_clients_recv,
        rsa_key,
        public_key_der,
        http_client: reqwest::Client::new(),
    }));

    app.insert_resource(shared.clone());

    // System for starting the accept loop.
    let start_accept_loop = move |shared: Res<SharedNetworkState>| {
        let _guard = shared.0.tokio_handle.enter();

        // Start accepting new connections.
        tokio::spawn(do_accept_loop(shared.clone()));
    };

    let start_broadcast_to_lan_loop = move |shared: Res<SharedNetworkState>| {
        let _guard = shared.0.tokio_handle.enter();

        tokio::spawn(do_broadcast_to_lan_loop(shared.clone()));
    };

    // System for spawning new clients.
    let spawn_new_clients = move |world: &mut World| {
        for _ in 0..shared.0.new_clients_recv.len() {
            match shared.0.new_clients_recv.try_recv() {
                Ok(args) => world.spawn(ClientBundle::new(args)),
                Err(_) => break,
            };
        }
    };

    // Start accepting connections in `PostStartup` to allow user startup code to
    // run first.
    app.add_systems(PostStartup, start_accept_loop);

    // Start the loop that will broadcast messages for the LAN discovery list.
    app.add_systems(PostStartup, start_broadcast_to_lan_loop);

    // Spawn new clients before the event loop starts.
    app.add_systems(PreUpdate, spawn_new_clients.in_set(SpawnClientsSet));

    Ok(())
}

#[derive(Resource, Clone)]
pub struct SharedNetworkState(Arc<SharedNetworkStateInner>);

impl SharedNetworkState {
    pub fn connection_mode(&self) -> &ConnectionMode {
        &self.0.connection_mode
    }

    pub fn player_count(&self) -> &AtomicUsize {
        &self.0.player_count
    }

    pub fn max_players(&self) -> usize {
        self.0.max_players
    }
}
struct SharedNetworkStateInner {
    callbacks: ErasedNetworkCallbacks,
    address: SocketAddr,
    incoming_byte_limit: usize,
    outgoing_byte_limit: usize,
    /// Limits the number of simultaneous connections to the server before the
    /// play state.
    connection_sema: Arc<Semaphore>,
    //// The number of clients in the play state, past the login state.
    player_count: AtomicUsize,
    max_players: usize,
    connection_mode: ConnectionMode,
    threshold: CompressionThreshold,
    tokio_handle: Handle,
    // Holding a runtime handle is not enough to keep tokio working. We need
    // to store the runtime here so we don't drop it.
    _tokio_runtime: Option<Runtime>,
    /// Sender for new clients past the login stage.
    new_clients_send: Sender<ClientBundleArgs>,
    /// Receiver for new clients past the login stage.
    new_clients_recv: Receiver<ClientBundleArgs>,
    /// The RSA keypair used for encryption with clients.
    rsa_key: RsaPrivateKey,
    /// The public part of `rsa_key` encoded in DER, which is an ASN.1 format.
    /// This is sent to clients during the authentication process.
    public_key_der: Box<[u8]>,
    /// For session server requests.
    http_client: reqwest::Client,
}

/// Contains information about a new client joining the server.
#[derive(Debug)]
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
    pub properties: Properties,
}

/// Settings for [`NetworkPlugin`]. Note that mutations to these fields have no
/// effect after the plugin is built.
#[derive(Resource, Clone)]
pub struct NetworkSettings {
    pub callbacks: ErasedNetworkCallbacks,
    /// The [`Handle`] to the tokio runtime the server will use. If `None` is
    /// provided, the server will create its own tokio runtime at startup.
    ///
    /// # Default Value
    ///
    /// `None`
    pub tokio_handle: Option<Handle>,
    /// The maximum number of simultaneous initial connections to the server.
    ///
    /// This only considers the connections _before_ the play state where the
    /// client is spawned into the world..
    ///
    /// # Default Value
    ///
    /// The default value is left unspecified and may change in future versions.
    pub max_connections: usize,
    /// # Default Value
    ///
    /// `20`
    pub max_players: usize,
    /// The socket address the server will be bound to.
    ///
    /// # Default Value
    ///
    /// `0.0.0.0:25565`, which will listen on every available network interface.
    pub address: SocketAddr,
    /// The connection mode. This determines if client authentication and
    /// encryption should take place and if the server should get the player
    /// data from a proxy.
    ///
    /// **NOTE:** Mutations to this field have no effect if
    ///
    /// # Default Value
    ///
    /// [`ConnectionMode::Online`]
    pub connection_mode: ConnectionMode,
    /// The maximum capacity (in bytes) of the buffer used to hold incoming
    /// packet data.
    ///
    /// A larger capacity reduces the chance that a client needs to be
    /// disconnected due to a full buffer, but increases potential
    /// memory usage.
    ///
    /// # Default Value
    ///
    /// The default value is left unspecified and may change in future versions.
    pub incoming_byte_limit: usize,
    /// The maximum capacity (in bytes) of the buffer used to hold outgoing
    /// packet data.
    ///
    /// A larger capacity reduces the chance that a client needs to be
    /// disconnected due to a full buffer, but increases potential
    /// memory usage.
    ///
    /// # Default Value
    ///
    /// The default value is left unspecified and may change in future versions.
    pub outgoing_byte_limit: usize,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            callbacks: ErasedNetworkCallbacks::default(),
            tokio_handle: None,
            max_connections: 1024,
            max_players: 20,
            address: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 25565).into(),
            connection_mode: ConnectionMode::Online {
                prevent_proxy_connections: false,
            },
            incoming_byte_limit: 2097152, // 2 MiB
            outgoing_byte_limit: 8388608, // 8 MiB
        }
    }
}

/// A type-erased wrapper around an [`NetworkCallbacks`] object.
#[derive(Clone)]
pub struct ErasedNetworkCallbacks {
    // TODO: do some shenanigans when async-in-trait is stabilized.
    inner: Arc<dyn NetworkCallbacks>,
}

impl ErasedNetworkCallbacks {
    pub fn new<C: NetworkCallbacks>(callbacks: C) -> Self {
        Self {
            inner: Arc::new(callbacks),
        }
    }
}

impl Default for ErasedNetworkCallbacks {
    fn default() -> Self {
        Self {
            inner: Arc::new(()),
        }
    }
}

impl<T: NetworkCallbacks> From<T> for ErasedNetworkCallbacks {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

/// This trait uses [`mod@async_trait`].
#[async_trait]
pub trait NetworkCallbacks: Send + Sync + 'static {
    /// Called when the server receives a Server List Ping query.
    /// Data for the response can be provided or the query can be ignored.
    ///
    /// This function is called from within a tokio runtime.
    ///
    /// # Default Implementation
    ///
    /// A default placeholder response is returned.
    async fn server_list_ping(
        &self,
        shared: &SharedNetworkState,
        remote_addr: SocketAddr,
        handshake_data: &HandshakeData,
    ) -> ServerListPing {
        #![allow(unused_variables)]

        ServerListPing::Respond {
            online_players: shared.player_count().load(Ordering::Relaxed) as i32,
            max_players: shared.max_players() as i32,
            player_sample: vec![],
            description: "A Valence Server".into_text(),
            favicon_png: &[],
            version_name: MINECRAFT_VERSION.to_owned(),
            protocol: PROTOCOL_VERSION,
        }
    }

    /// Called when the server receives a Server List Legacy Ping query.
    /// Data for the response can be provided or the query can be ignored.
    ///
    /// This function is called from within a tokio runtime.
    ///
    /// # Default Implementation
    ///
    /// [`server_list_ping`][Self::server_list_ping] re-used.
    async fn server_list_legacy_ping(
        &self,
        shared: &SharedNetworkState,
        remote_addr: SocketAddr,
        payload: ServerListLegacyPingPayload,
    ) -> ServerListLegacyPing {
        #![allow(unused_variables)]

        let handshake_data = match payload {
            ServerListLegacyPingPayload::Pre1_7 {
                protocol,
                hostname,
                port,
            } => HandshakeData {
                protocol_version: protocol,
                server_address: hostname,
                server_port: port,
            },
            _ => HandshakeData::default(),
        };

        match self
            .server_list_ping(shared, remote_addr, &handshake_data)
            .await
        {
            ServerListPing::Respond {
                online_players,
                max_players,
                player_sample,
                description,
                favicon_png,
                version_name,
                protocol,
            } => ServerListLegacyPing::Respond(
                ServerListLegacyPingResponse::new(protocol, online_players, max_players)
                    .version(version_name)
                    .description(description.to_legacy_lossy()),
            ),
            ServerListPing::Ignore => ServerListLegacyPing::Ignore,
        }
    }

    /// This function is called every 1.5 seconds to broadcast a packet over the
    /// local network in order to advertise the server to the multiplayer
    /// screen with a configurable MOTD.
    ///
    /// # Default Implementation
    ///
    /// The default implementation returns [`BroadcastToLan::Disabled`],
    /// disabling LAN discovery.
    async fn broadcast_to_lan(&self, shared: &SharedNetworkState) -> BroadcastToLan {
        #![allow(unused_variables)]

        BroadcastToLan::Disabled
    }

    /// Called for each client (after successful authentication if online mode
    /// is enabled) to determine if they can join the server.
    /// - If `Err(reason)` is returned, then the client is immediately
    ///   disconnected with `reason` as the displayed message.
    /// - Otherwise, `Ok(f)` is returned and the client will continue the login
    ///   process. This _may_ result in a new client being spawned with the
    ///   [`ClientBundle`] components. `f` is stored along with the client and
    ///   is called when the client is disconnected.
    ///
    ///   `f` is a callback function used for handling resource cleanup when the
    /// client is dropped. This is useful because a new client entity is not
    /// necessarily spawned into the world after a successful login.
    ///
    /// This method is called from within a tokio runtime, and is the
    /// appropriate place to perform asynchronous operations such as
    /// database queries which may take some time to complete.
    ///
    /// # Default Implementation
    ///
    /// TODO
    ///
    /// [`Client`]: valence::client::Client
    async fn login(
        &self,
        shared: &SharedNetworkState,
        info: &NewClientInfo,
    ) -> Result<CleanupFn, Text> {
        let _ = info;

        let max_players = shared.max_players();

        let success = shared
            .player_count()
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                (n < max_players).then_some(n + 1)
            })
            .is_ok();

        if success {
            let shared = shared.clone();

            Ok(Box::new(move || {
                let prev = shared.player_count().fetch_sub(1, Ordering::SeqCst);
                debug_assert_ne!(prev, 0, "player count underflowed");
            }))
        } else {
            // TODO: use correct translation key.
            Err("Server Full".into_text())
        }
    }

    /// Called upon every client login to obtain the full URL to use for session
    /// server requests. This is done to authenticate player accounts. This
    /// method is not called unless [online mode] is enabled.
    ///
    /// It is assumed that upon successful request, a structure matching the
    /// description in the [wiki](https://wiki.vg/Protocol_Encryption#Server) was obtained.
    /// Providing a URL that does not return such a structure will result in a
    /// disconnect for every client that connects.
    ///
    /// The arguments are described in the linked wiki article.
    ///
    /// # Default Implementation
    ///
    /// Uses the official Minecraft session server. This is formatted as
    /// `https://sessionserver.mojang.com/session/minecraft/hasJoined?username=<username>&serverId=<auth-digest>&ip=<player-ip>`.
    ///
    /// [online mode]: ConnectionMode::Online
    async fn session_server(
        &self,
        shared: &SharedNetworkState,
        username: &str,
        auth_digest: &str,
        player_ip: &IpAddr,
    ) -> String {
        if shared.connection_mode()
            == (&ConnectionMode::Online {
                prevent_proxy_connections: true,
            })
        {
            format!("https://sessionserver.mojang.com/session/minecraft/hasJoined?username={username}&serverId={auth_digest}&ip={player_ip}")
        } else {
            format!("https://sessionserver.mojang.com/session/minecraft/hasJoined?username={username}&serverId={auth_digest}")
        }
    }
}

/// A callback function called when the associated client is dropped. See
/// [`NetworkCallbacks::login`] for more information.
pub type CleanupFn = Box<dyn FnOnce() + Send + Sync + 'static>;
struct CleanupOnDrop(Option<CleanupFn>);

impl Drop for CleanupOnDrop {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

/// The default network callbacks. Useful as a placeholder.
impl NetworkCallbacks for () {}

/// Describes how new connections to the server are handled.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum ConnectionMode {
    /// The "online mode" fetches all player data (username, UUID, and
    /// properties) from the [configured session server] and enables
    /// encryption.
    ///
    /// This mode should be used by all publicly exposed servers which are not
    /// behind a proxy.
    ///
    /// [configured session server]: NetworkCallbacks::session_server
    Online {
        /// Determines if client IP validation should take place during
        /// authentication.
        ///
        /// When `prevent_proxy_connections` is enabled, clients can no longer
        /// log-in if they connected to the Yggdrasil server using a different
        /// IP than the one used to connect to this server.
        ///
        /// This is used by the default implementation of
        /// [`NetworkCallbacks::session_server`]. A different implementation may
        /// choose to ignore this value.
        prevent_proxy_connections: bool,
    },
    /// Disables client authentication with the configured session server.
    /// Clients can join with any username and UUID they choose, potentially
    /// gaining privileges they would not otherwise have. Additionally,
    /// encryption is disabled and Minecraft's default skins will be used.
    ///
    /// This mode should be used for development purposes only and not for
    /// publicly exposed servers.
    Offline,
    /// This mode should be used under one of the following situations:
    /// - The server is behind a [BungeeCord]/[Waterfall] proxy with IP
    ///   forwarding enabled.
    /// - The server is behind a [Velocity] proxy configured to use the `legacy`
    ///   forwarding mode.
    ///
    /// All player data (username, UUID, and properties) is fetched from the
    /// proxy, but no attempt is made to stop connections originating from
    /// elsewhere. As a result, you must ensure clients connect through the
    /// proxy and are unable to connect to the server directly. Otherwise,
    /// clients can use any username or UUID they choose similar to
    /// [`ConnectionMode::Offline`].
    ///
    /// To protect against this, a firewall can be used. However,
    /// [`ConnectionMode::Velocity`] is recommended as a secure alternative.
    ///
    /// [BungeeCord]: https://www.spigotmc.org/wiki/bungeecord/
    /// [Waterfall]: https://github.com/PaperMC/Waterfall
    /// [Velocity]: https://velocitypowered.com/
    BungeeCord,
    /// This mode is used when the server is behind a [Velocity] proxy
    /// configured with the forwarding mode `modern`.
    ///
    /// All player data (username, UUID, and properties) is fetched from the
    /// proxy and all connections originating from outside Velocity are
    /// blocked.
    ///
    /// [Velocity]: https://velocitypowered.com/
    Velocity {
        /// The secret key used to prevent connections from outside Velocity.
        /// The proxy and Valence must be configured to use the same secret key.
        secret: Arc<str>,
    },
}

/// The result of the Server List Ping [callback].
///
/// [callback]: NetworkCallbacks::server_list_ping
#[derive(Clone, Default, Debug)]
pub enum ServerListPing<'a> {
    /// Responds to the server list ping with the given information.
    Respond {
        /// Displayed as the number of players on the server.
        online_players: i32,
        /// Displayed as the maximum number of players allowed on the server at
        /// a time.
        max_players: i32,
        /// The list of players visible by hovering over the player count.
        ///
        /// Has no effect if this list is empty.
        player_sample: Vec<PlayerSampleEntry>,
        /// A description of the server.
        description: Text,
        /// The server's icon as the bytes of a PNG image.
        /// The image must be 64x64 pixels.
        ///
        /// No icon is used if the slice is empty.
        favicon_png: &'a [u8],
        /// The version name of the server. Displayed when client is using a
        /// different protocol.
        ///
        /// Can be formatted using `§` and format codes. Or use
        /// [`valence_protocol::text::Text::to_legacy_lossy`].
        version_name: String,
        /// The protocol version of the server.
        protocol: i32,
    },
    /// Ignores the query and disconnects from the client.
    #[default]
    Ignore,
}

/// The result of the Server List Legacy Ping [callback].
///
/// [callback]: NetworkCallbacks::server_list_legacy_ping
#[derive(Clone, Default, Debug)]
pub enum ServerListLegacyPing {
    /// Responds to the server list legacy ping with the given information.
    Respond(ServerListLegacyPingResponse),
    /// Ignores the query and disconnects from the client.
    #[default]
    Ignore,
}

/// The result of the Broadcast To Lan [callback].
///
/// [callback]: NetworkCallbacks::broadcast_to_lan
#[derive(Clone, Default, Debug)]
pub enum BroadcastToLan<'a> {
    /// Disabled Broadcast To Lan.
    #[default]
    Disabled,
    /// Send packet to broadcast to LAN every 1.5 seconds with specified MOTD.
    Enabled(Cow<'a, str>),
}

/// Represents an individual entry in the player sample.
#[derive(Clone, Debug, Serialize)]
pub struct PlayerSampleEntry {
    /// The name of the player.
    ///
    /// This string can contain
    /// [legacy formatting codes](https://minecraft.wiki/w/Formatting_codes).
    pub name: String,
    /// The player UUID.
    pub id: Uuid,
}

#[allow(clippy::infinite_loop)]
async fn do_broadcast_to_lan_loop(shared: SharedNetworkState) {
    let port = shared.0.address.port();

    let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await else {
        tracing::error!("Failed to bind to UDP socket for broadcast to LAN");
        return;
    };

    loop {
        let motd = match shared.0.callbacks.inner.broadcast_to_lan(&shared).await {
            BroadcastToLan::Disabled => {
                time::sleep(Duration::from_millis(1500)).await;
                continue;
            }
            BroadcastToLan::Enabled(motd) => motd,
        };

        let message = format!("[MOTD]{motd}[/MOTD][AD]{port}[/AD]");

        if let Err(e) = socket.send_to(message.as_bytes(), "224.0.2.60:4445").await {
            tracing::warn!("Failed to send broadcast to LAN packet: {}", e);
        }

        // wait 1.5 seconds
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    }
}
