use std::borrow::Cow;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

use async_trait::async_trait;
use bevy_ecs::world::World;
use serde::Serialize;
use tokio::runtime::Handle;
use uuid::Uuid;
use valence_protocol::{Text, Username};

use crate::biome::Biome;
use crate::dimension::Dimension;
use crate::server::{NewClientInfo, SharedServer};

/// Contains basic server settings with reasonable defaults. Use [`run_server`]
/// to consume the configuration and start the server.
///
/// [`run_server`]: crate::run_server.
#[non_exhaustive]
pub struct Config {
    /// The Bevy ECS [`World`] to use for storing entities. Note that this is
    /// unrelated to Minecraft's concept of a "world."
    ///
    /// # Default Value
    ///
    /// `World::new()`
    pub world: World,
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
    /// The tick rate. This is the number of game updates that should occur in
    /// one second.
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
    pub tick_rate: i64,
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
    /// The list of [`Dimension`]s usable on the server.
    ///
    /// The dimensions returned by [`Server::dimensions`] will be in the
    /// same order as this `Vec`.
    ///
    /// The number of elements in the `Vec` must be in `1..=u16::MAX`.
    /// Additionally, the documented requirements on the fields of [`Dimension`]
    /// must be met.
    ///
    /// # Default Value
    ///
    /// `vec![Dimension::default()]`
    pub dimensions: Vec<Dimension>,
    /// The list of [`Biome`]s usable on the server.
    ///
    /// The biomes returned by [`SharedServer::biomes`] will be in the same
    /// order as this `Vec`.
    ///
    /// The number of elements in the `Vec` must be in `1..=u16::MAX`.
    /// Additionally, the documented requirements on the fields of [`Biome`]
    /// must be met.
    ///
    /// **NOTE**: As of 1.19.2, there is a bug in the client which prevents
    /// joining the game when a biome named "minecraft:plains" is not present.
    /// Ensure there is a biome named "plains".
    ///
    /// # Default Value
    ///
    /// `vec![Biome::default()]`.
    pub biomes: Vec<Biome>,
}

impl Config {
    /// See [`Self::world`].
    pub fn with_world(mut self, world: World) -> Self {
        self.world = world;
        self
    }

    /// See [`Self::tokio_handle`].
    pub fn with_tokio_handle(mut self, tokio_handle: Option<Handle>) -> Self {
        self.tokio_handle = tokio_handle;
        self
    }

    /// See [`Self::max_connections`].
    pub fn with_max_connections(mut self, max_connections: usize) -> Self {
        self.max_connections = max_connections;
        self
    }

    /// See [`Self::address`].
    pub fn with_address(mut self, address: SocketAddr) -> Self {
        self.address = address;
        self
    }

    /// See [`Self::tick_rate`].
    pub fn with_tick_rate(mut self, tick_rate: i64) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    /// See [`Self::connection_mode`].
    pub fn with_connection_mode(mut self, connection_mode: ConnectionMode) -> Self {
        self.connection_mode = connection_mode;
        self
    }

    /// See [`Self::compression_threshold`].
    pub fn with_compression_threshold(mut self, compression_threshold: Option<u32>) -> Self {
        self.compression_threshold = compression_threshold;
        self
    }

    /// See [`Self::incoming_capacity`].
    pub fn with_incoming_capacity(mut self, incoming_capacity: usize) -> Self {
        self.incoming_capacity = incoming_capacity;
        self
    }

    /// See [`Self::outgoing_capacity`].
    pub fn with_outgoing_capacity(mut self, outgoing_capacity: usize) -> Self {
        self.outgoing_capacity = outgoing_capacity;
        self
    }

    /// See [`Self::dimensions`].
    pub fn with_dimensions(mut self, dimensions: impl Into<Vec<Dimension>>) -> Self {
        self.dimensions = dimensions.into();
        self
    }

    /// See [`Self::biomes`].
    pub fn with_biomes(mut self, biomes: impl Into<Vec<Biome>>) -> Self {
        self.biomes = biomes.into();
        self
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
            player_sample: Cow::default(),
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
        username: Username<&str>,
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

impl AsyncCallbacks for () {}

impl Default for Config {
    fn default() -> Self {
        Self {
            world: World::default(),
            tokio_handle: None,
            max_connections: 1024,
            address: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 25565).into(),
            tick_rate: 20,
            connection_mode: ConnectionMode::Online {
                prevent_proxy_connections: true,
            },
            compression_threshold: Some(256),
            incoming_capacity: 2097152, // 2 MiB
            outgoing_capacity: 8388608, // 8 MiB
            dimensions: vec![Dimension::default()],
            biomes: vec![Biome::default()],
        }
    }
}

/// Describes how new connections to the server are handled.
#[non_exhaustive]
#[derive(Clone, PartialEq)]
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
        secret: String,
    },
}

/// Minecraft's standard ticks per second (TPS).
pub const DEFAULT_TPS: i64 = 20;

/// The result of the [Server List Ping] callback.
///
/// [Server List Ping]: Config::server_list_ping_cb
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
        player_sample: Cow<'a, [PlayerSampleEntry<'a>]>,
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
pub struct PlayerSampleEntry<'a> {
    /// The name of the player.
    ///
    /// This string can contain
    /// [legacy formatting codes](https://minecraft.fandom.com/wiki/Formatting_codes).
    pub name: Cow<'a, str>,
    /// The player UUID.
    pub id: Uuid,
}
