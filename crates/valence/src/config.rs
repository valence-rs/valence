use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use async_trait::async_trait;
use bevy_app::{App, Plugin};
use serde::Serialize;
use tokio::runtime::Handle;
use tracing::error;
use uuid::Uuid;
use valence_protocol::text::Text;

use crate::server::{NewClientInfo, SharedServer};

#[derive(Clone)]
#[non_exhaustive]
pub struct ServerPlugin<A> {
    pub callbacks: Arc<A>,
    /// The [`Handle`] to the tokio runtime the server will use. If `None` is
    /// provided, the server will create its own tokio runtime at startup.
    ///
    /// # Default Value
    ///
    /// `None`
    pub tokio_handle: Option<Handle>,
    /// The maximum number of simultaneous connections allowed to the server.
    /// This includes all connections, not just those past the login stage.
    ///
    /// You will want this value to be somewhere above the maximum number of
    /// players, since status pings should still succeed even when the server is
    /// full.
    ///
    /// # Default Value
    ///
    /// `1024`. This may change in a future version.
    pub max_connections: usize,
    /// The socket address the server will be bound to.
    ///
    /// # Default Value
    ///
    /// `0.0.0.0:25565`, which will listen on every available network interface.
    pub address: SocketAddr,
    /// The ticks per second of the server. This is the number of game updates
    /// that should occur in one second.
    ///
    /// On each game update (tick), the server is expected to update game logic
    /// and respond to packets from clients. Once this is complete, the server
    /// will sleep for any remaining time until a full tick has passed.
    ///
    /// The tick rate must be greater than zero.
    ///
    /// Note that the official Minecraft client only processes packets at 20hz,
    /// so there is little benefit to a tick rate higher than 20.
    ///
    /// # Default Value
    ///
    /// [`DEFAULT_TPS`]
    pub tps: i64,
    /// The connection mode. This determines if client authentication and
    /// encryption should take place and if the server should get the player
    /// data from a proxy.
    ///
    /// # Default Value
    ///
    /// [`ConnectionMode::Online`]
    pub connection_mode: ConnectionMode,
    /// The compression threshold to use for compressing packets. For a
    /// compression threshold of `Some(N)`, packets with encoded lengths >= `N`
    /// are compressed while all others are not. `None` disables compression
    /// completely.
    ///
    /// If the server is used behind a proxy on the same machine, you will
    /// likely want to disable compression.
    ///
    /// # Default Value
    ///
    /// Compression is enabled with an unspecified threshold.
    pub compression_threshold: Option<u32>,
    /// The maximum capacity (in bytes) of the buffer used to hold incoming
    /// packet data.
    ///
    /// A larger capacity reduces the chance that a client needs to be
    /// disconnected due to the buffer being full, but increases potential
    /// memory usage.
    ///
    /// # Default Value
    ///
    /// An unspecified value is used that should be adequate for most
    /// situations. This default may change in future versions.
    pub incoming_capacity: usize,
    /// The maximum capacity (in bytes) of the buffer used to hold outgoing
    /// packets.
    ///
    /// A larger capacity reduces the chance that a client needs to be
    /// disconnected due to the buffer being full, but increases potential
    /// memory usage.
    ///
    /// # Default Value
    ///
    /// An unspecified value is used that should be adequate for most
    /// situations. This default may change in future versions.
    pub outgoing_capacity: usize,
}

impl<A: AsyncCallbacks> ServerPlugin<A> {
    pub fn new(callbacks: impl Into<Arc<A>>) -> Self {
        Self {
            callbacks: callbacks.into(),
            tokio_handle: None,
            max_connections: 1024,
            address: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 25565).into(),
            tps: DEFAULT_TPS,
            connection_mode: ConnectionMode::Online {
                // Note: Some people have problems using valence when this is enabled by default.
                prevent_proxy_connections: false,
            },
            compression_threshold: Some(256),
            incoming_capacity: 2097152, // 2 MiB
            outgoing_capacity: 8388608, // 8 MiB
        }
    }

    /// See [`Self::tokio_handle`].
    #[must_use]
    pub fn with_tokio_handle(mut self, tokio_handle: Option<Handle>) -> Self {
        self.tokio_handle = tokio_handle;
        self
    }

    /// See [`Self::max_connections`].
    #[must_use]
    pub fn with_max_connections(mut self, max_connections: usize) -> Self {
        self.max_connections = max_connections;
        self
    }

    /// See [`Self::address`].
    #[must_use]
    pub fn with_address(mut self, address: SocketAddr) -> Self {
        self.address = address;
        self
    }

    /// See [`Self::tps`].
    #[must_use]
    pub fn with_tick_rate(mut self, tick_rate: i64) -> Self {
        self.tps = tick_rate;
        self
    }

    /// See [`Self::connection_mode`].
    #[must_use]
    pub fn with_connection_mode(mut self, connection_mode: ConnectionMode) -> Self {
        self.connection_mode = connection_mode;
        self
    }

    /// See [`Self::compression_threshold`].
    #[must_use]
    pub fn with_compression_threshold(mut self, compression_threshold: Option<u32>) -> Self {
        self.compression_threshold = compression_threshold;
        self
    }

    /// See [`Self::incoming_capacity`].
    #[must_use]
    pub fn with_incoming_capacity(mut self, incoming_capacity: usize) -> Self {
        self.incoming_capacity = incoming_capacity;
        self
    }

    /// See [`Self::outgoing_capacity`].
    #[must_use]
    pub fn with_outgoing_capacity(mut self, outgoing_capacity: usize) -> Self {
        self.outgoing_capacity = outgoing_capacity;
        self
    }
}

impl<A: AsyncCallbacks + Default> Default for ServerPlugin<A> {
    fn default() -> Self {
        Self::new(A::default())
    }
}

impl<A: AsyncCallbacks> Plugin for ServerPlugin<A> {
    fn build(&self, app: &mut App) {
        if let Err(e) = crate::server::build_plugin(self, app) {
            error!("failed to build Valence plugin: {e:#}");
        }
    }
}

#[async_trait]
pub trait AsyncCallbacks: Send + Sync + 'static {
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
        shared: &SharedServer,
        remote_addr: SocketAddr,
        protocol_version: i32,
    ) -> ServerListPing {
        #![allow(unused_variables)]
        ServerListPing::Respond {
            online_players: 0, // TODO: get online players.
            max_players: -1,
            player_sample: vec![],
            description: "A Valence Server".into(),
            favicon_png: &[],
        }
    }

    /// Called for each client after successful authentication (if online mode
    /// is enabled) to determine if they can join the server. On success, a
    /// new entity is spawned with the [`Client`] component. If this method
    /// returns with `Err(reason)`, then the client is immediately
    /// disconnected with `reason` as the displayed message.
    ///
    /// This method is the appropriate place to perform asynchronous
    /// operations such as database queries which may take some time to
    /// complete.
    ///
    /// This method is called from within a tokio runtime.
    ///
    /// # Default Implementation
    ///
    /// The client is allowed to join unconditionally.
    ///
    /// [`Client`]: crate::client::Client
    async fn login(&self, shared: &SharedServer, info: &NewClientInfo) -> Result<(), Text> {
        #![allow(unused_variables)]
        Ok(())
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
    /// [online mode]: crate::config::ConnectionMode::Online
    async fn session_server(
        &self,
        shared: &SharedServer,
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

/// The default async callbacks.
impl AsyncCallbacks for () {}

/// Describes how new connections to the server are handled.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum ConnectionMode {
    /// The "online mode" fetches all player data (username, UUID, and skin)
    /// from the [configured session server] and enables encryption.
    ///
    /// This mode should be used by all publicly exposed servers which are not
    /// behind a proxy.
    ///
    /// [configured session server]: AsyncCallbacks::session_server
    Online {
        /// Determines if client IP validation should take place during
        /// authentication.
        ///
        /// When `prevent_proxy_connections` is enabled, clients can no longer
        /// log-in if they connected to the Yggdrasil server using a different
        /// IP than the one used to connect to this server.
        ///
        /// This is used by the default implementation of
        /// [`AsyncCallbacks::session_server`]. A different implementation may
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
    /// All player data (username, UUID, and skin) is fetched from the proxy,
    /// but no attempt is made to stop connections originating from
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
    /// All player data (username, UUID, and skin) is fetched from the proxy and
    /// all connections originating from outside Velocity are blocked.
    ///
    /// [Velocity]: https://velocitypowered.com/
    Velocity {
        /// The secret key used to prevent connections from outside Velocity.
        /// The proxy and Valence must be configured to use the same secret key.
        secret: Arc<str>,
    },
}

/// Minecraft's standard ticks per second (TPS).
pub const DEFAULT_TPS: i64 = 20;

/// The result of the Server List Ping [callback].
///
/// [callback]: crate::config::AsyncCallbacks
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
    },
    /// Ignores the query and disconnects from the client.
    #[default]
    Ignore,
}

/// Represents an individual entry in the player sample.
#[derive(Clone, Debug, Serialize)]
pub struct PlayerSampleEntry {
    /// The name of the player.
    ///
    /// This string can contain
    /// [legacy formatting codes](https://minecraft.fandom.com/wiki/Formatting_codes).
    pub name: String,
    /// The player UUID.
    pub id: Uuid,
}
