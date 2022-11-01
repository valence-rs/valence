//! Configuration for the server.

use std::borrow::Cow;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

use async_trait::async_trait;
use serde::Serialize;
use tokio::runtime::Handle as TokioHandle;
use uuid::Uuid;

use crate::biome::Biome;
use crate::dimension::Dimension;
use crate::protocol::MAX_PACKET_SIZE;
use crate::server::{NewClientData, Server, SharedServer};
use crate::text::Text;
use crate::username::Username;
use crate::{Ticks, STANDARD_TPS};

/// A trait for the configuration of a server.
///
/// This trait uses the [async_trait] attribute macro. It is exported at the
/// root of this crate. async_trait will be removed once async fns in traits
/// are stabilized.
///
/// [async_trait]: https://docs.rs/async-trait/latest/async_trait/
#[async_trait]
#[allow(unused_variables)]
pub trait Config: Sized + Send + Sync + 'static {
    /// Custom state to store with the [`Server`].
    type ServerState: Send + Sync;
    /// Custom state to store with every [`Client`](crate::client::Client).
    type ClientState: Default + Send + Sync;
    /// Custom state to store with every [`Entity`](crate::entity::Entity).
    type EntityState: Send + Sync;
    /// Custom state to store with every [`World`](crate::world::World).
    type WorldState: Send + Sync;
    /// Custom state to store with every
    /// [`LoadedChunk`](crate::chunk::LoadedChunk).
    type ChunkState: Send + Sync;
    /// Custom state to store with every
    /// [`PlayerList`](crate::player_list::PlayerList).
    type PlayerListState: Send + Sync;

    /// Called once at startup to get the maximum number of simultaneous
    /// connections allowed to the server. This includes all
    /// connections, not just those past the login stage.
    ///
    /// You will want this value to be somewhere above the maximum number of
    /// players, since status pings should still succeed even when the server is
    /// full.
    ///
    /// # Default Implementation
    ///
    /// Currently returns `1024`. This may change in a future version.
    fn max_connections(&self) -> usize {
        1024
    }

    /// Called once at startup to get the socket address the server will
    /// be bound to.
    ///
    /// # Default Implementation
    ///
    /// Returns `0.0.0.0:25565` to listen on every available network interface.
    fn address(&self) -> SocketAddr {
        SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 25565).into()
    }

    /// Called once at startup to get the tick rate, which is the number of game
    /// updates that should occur in one second.
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
    /// # Default Implementation
    ///
    /// Returns [`STANDARD_TPS`].
    fn tick_rate(&self) -> Ticks {
        STANDARD_TPS
    }

    /// Called once at startup to get the connection mode option, which
    /// determines if client authentication and encryption should take place
    /// and if the server should get the player data from a proxy.
    ///
    /// # Default Implementation
    ///
    /// Returns [`ConnectionMode::Online`].
    fn connection_mode(&self) -> ConnectionMode {
        ConnectionMode::Online
    }

    /// Called once at startup to get the "prevent-proxy-connections" option,
    /// which determines if client IP validation should take place.
    ///
    /// When prevent_proxy_connections is enabled, clients can no longer log-in
    /// if they connected to the yggdrasil server using a different IP.
    ///
    /// # Default Implementation
    /// Proxy connections are allowed.
    ///
    /// Returns `false`.
    fn prevent_proxy_connections(&self) -> bool {
        false
    }

    /// Called once at startup to get the maximum capacity (in bytes) of the
    /// buffer used to hold incoming packet data.
    ///
    /// A larger capacity reduces the chance that a client needs to be
    /// disconnected due to the buffer being full, but increases potential
    /// memory usage.
    ///
    /// # Default Implementation
    ///
    /// An unspecified value is returned that should be adequate in most
    /// situations.
    fn incoming_capacity(&self) -> usize {
        MAX_PACKET_SIZE as usize
    }

    /// Called once at startup to get the maximum capacity (in bytes) of the
    /// buffer used to hold outgoing packets.
    ///
    /// A larger capacity reduces the chance that a client needs to be
    /// disconnected due to the buffer being full, but increases potential
    /// memory usage.
    ///
    /// # Default Implementation
    ///
    /// An unspecified value is returned that should be adequate in most
    /// situations.
    fn outgoing_capacity(&self) -> usize {
        MAX_PACKET_SIZE as usize * 4
    }

    /// Called once at startup to get a handle to the tokio runtime the server
    /// will use.
    ///
    /// If a handle is not provided, the server will create its own tokio
    /// runtime.
    ///
    /// # Default Implementation
    ///
    /// Returns `None`.
    fn tokio_handle(&self) -> Option<TokioHandle> {
        None
    }

    /// Called once at startup to get the list of [`Dimension`]s usable on the
    /// server.
    ///
    /// The dimensions returned by [`SharedServer::dimensions`] will be in the
    /// same order as the `Vec` returned by this function.
    ///
    /// The number of elements in the returned `Vec` must be in `1..=u16::MAX`.
    /// Additionally, the documented requirements on the fields of [`Dimension`]
    /// must be met.
    ///
    /// # Default Implementation
    ///
    /// Returns `vec![Dimension::default()]`.
    fn dimensions(&self) -> Vec<Dimension> {
        vec![Dimension::default()]
    }

    /// Called once at startup to get the list of [`Biome`]s usable on the
    /// server.
    ///
    /// The biomes returned by [`SharedServer::biomes`] will be in the same
    /// order as the `Vec` returned by this function.
    ///
    /// The number of elements in the returned `Vec` must be in `1..=u16::MAX`.
    /// Additionally, the documented requirements on the fields of [`Biome`]
    /// must be met.
    ///
    /// **NOTE**: As of 1.19.2, there is a bug in the client which prevents
    /// joining the game when a biome named "minecraft:plains" is not present.
    /// Ensure there is a biome named "plains".
    ///
    /// # Default Implementation
    ///
    /// Returns `vec![Biome::default()]`.
    fn biomes(&self) -> Vec<Biome> {
        vec![Biome::default()]
    }

    /// Called when the server receives a Server List Ping query.
    /// Data for the response can be provided or the query can be ignored.
    ///
    /// This method is called from within a tokio runtime.
    ///
    /// # Default Implementation
    ///
    /// The query is ignored.
    async fn server_list_ping(
        &self,
        shared: &SharedServer<Self>,
        remote_addr: SocketAddr,
        protocol_version: i32,
    ) -> ServerListPing {
        ServerListPing::Ignore
    }

    /// Called asynchronously for each client after successful authentication
    /// (if online mode is enabled) to determine if they can join
    /// the server. On success, the new client is added to the server's
    /// [`Clients`]. If this method returns with `Err(reason)`, then the
    /// client is immediately disconnected with the given reason.
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
    /// [`Clients`]: crate::client::Clients
    async fn login(&self, shared: &SharedServer<Self>, ncd: &NewClientData) -> Result<(), Text> {
        Ok(())
    }

    /// Called upon (every) client connect (if online mode is enabled) to obtain
    /// the full URL to use for session server requests. Defaults to
    /// `https://sessionserver.mojang.com/session/minecraft/hasJoined?username=<username>&serverId=<auth-digest>&ip=<player-ip>`.
    ///
    /// It is assumed, that upon successful request, a structure matching the
    /// description in the [wiki](https://wiki.vg/Protocol_Encryption#Server) was obtained.
    /// Providing a URL that does not return such a structure will result in a
    /// disconnect for every client that connects.
    ///
    /// The arguments are described in the linked wiki article.
    fn format_session_server_url(
        &self,
        server: &SharedServer<Self>,
        username: Username<&str>,
        auth_digest: &str,
        player_ip: &IpAddr,
    ) -> String {
        if self.prevent_proxy_connections() {
            format!("https://sessionserver.mojang.com/session/minecraft/hasJoined?username={username}&serverId={auth_digest}&ip={player_ip}")
        } else {
            format!("https://sessionserver.mojang.com/session/minecraft/hasJoined?username={username}&serverId={auth_digest}")
        }
    }

    /// Called after the server is created, but prior to accepting connections
    /// and entering the update loop.
    ///
    /// This is useful for performing initialization work with a guarantee that
    /// no connections to the server will be made until this function returns.
    ///
    /// This method is called from within a tokio runtime.
    ///
    /// # Default Implementation
    ///
    /// The default implementation does nothing.
    fn init(&self, server: &mut Server<Self>) {}

    /// Called once at the beginning of every server update (also known as
    /// "tick"). This is likely where the majority of your code will be.
    ///
    /// The frequency of ticks can be configured by [`Self::tick_rate`].
    ///
    /// This method is called from within a tokio runtime.
    ///
    /// # Default Implementation
    ///
    /// The default implementation does nothing.
    fn update(&self, server: &mut Server<Self>);
}

/// The result of the [`server_list_ping`](Config::server_list_ping) callback.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
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
        /// No icon is used if the value is `None`.
        favicon_png: Option<Cow<'a, [u8]>>,
    },
    /// Ignores the query and disconnects from the client.
    Ignore,
}

/// Describes how new connections to the server are handled.
#[non_exhaustive]
#[derive(Clone, PartialEq, Default)]
pub enum ConnectionMode {
    /// The "online mode" fetches all player data (username, UUID, and skin)
    /// from the [configured session server] and enables encryption.
    ///
    /// This mode should be used for all publicly exposed servers which are not
    /// behind a proxy.
    ///
    /// [configured session server]: Config::format_session_server_url
    #[default]
    Online,
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

/// A minimal `Config` implementation for testing purposes.
#[cfg(test)]
pub(crate) struct MockConfig<S = (), Cl = (), E = (), W = (), Ch = (), P = ()> {
    _marker: std::marker::PhantomData<(S, Cl, E, W, Ch, P)>,
}

#[cfg(test)]
impl<S, Cl, E, W, Ch, P> Config for MockConfig<S, Cl, E, W, Ch, P>
where
    S: Send + Sync + 'static,
    Cl: Default + Send + Sync + 'static,
    E: Send + Sync + 'static,
    W: Send + Sync + 'static,
    Ch: Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    type ServerState = S;
    type ClientState = Cl;
    type EntityState = E;
    type WorldState = W;
    type ChunkState = Ch;
    type PlayerListState = P;

    fn update(&self, _server: &mut Server<Self>) {}
}
