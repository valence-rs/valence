//! Configuration for the server.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::panic::{RefUnwindSafe, UnwindSafe};

use async_trait::async_trait;
use tokio::runtime::Handle as TokioHandle;

use crate::biome::Biome;
use crate::dimension::Dimension;
use crate::server::{NewClientData, Server, SharedServer};
use crate::text::Text;
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
pub trait Config: Sized + Send + Sync + UnwindSafe + RefUnwindSafe + 'static {
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
    fn max_connections(&self) -> usize;

    /// Called once at startup to get the socket address the server will
    /// be bound to.
    ///
    /// # Default Implementation
    ///
    /// Returns `127.0.0.1:25565`.
    fn address(&self) -> SocketAddr {
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 25565).into()
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

    /// Called once at startup to get the connection mode option, which determines
    /// if client authentication and encryption should take place
    /// and if the server should get the player data from a proxy.
    ///
    /// # `ConnectionMode::Online`
    /// The "online mode" fetches all player data (uuid, username and skin) from mojang
    /// and enables encryption.
    /// This should be used for all publicly exposed servers, which are not behind a proxy.
    ///
    /// # `ConnectionMode::Offline`
    /// If this mode is enabled all players can join with any username and uuid, which can be
    /// used by malicious actors to get privileges by for example using the username (and uuid)
    /// of the owner of the server.
    /// This should only be enabled for development purposes and not for publicly exposed servers.
    /// Furthermore encryption is disabled and minecraft's default skins will be used.
    ///
    /// # `ConnectionMode::Bungeecord`
    /// This should be used if the server runs behind a Bungeecord/Waterfall Proxy with ip forwarding enabled
    /// or a Velocity Proxy, which is configured to run with the player info forwarding mode `legacy`.
    /// It fetches all player data (uuid, username and skin) from the bungeecord proxy but does not
    /// block connections, which are not from the proxy.
    /// If the server is publicly exposed anyone can connect with any name, uuid and skin similar to offline mode.
    ///
    /// # `ConnectionMode::Velocity`
    /// This should be used if the server runs behind a Velocity Proxy, which is configured to run
    /// with the player info forwarding mode `modern`.
    /// It fetches all player data (uuid, username and skin) from the velocity proxy and blocks
    /// all connections, which are not from the proxy.
    /// To ensure this the return of `velocity_secret()` has to match the secret key of the velocity proxy.
    ///
    ///  # Default Implementation
    ///
    /// Returns `ConnectionMode::Online`
    fn connection_mode(&self) -> ConnectionMode {
        ConnectionMode::Online
    }

    /// Called once at startup to get the velocity secret which is only needed if the
    /// connection mode is `ConnectionMode::Velocity`
    ///
    /// # Default Implementation
    ///
    /// Returns ``
    fn velocity_secret(&self) -> &'static str {
        ""
    }

    /// Called once at startup to get the capacity of the buffer used to
    /// hold incoming packets.
    ///
    /// A larger capacity reduces the chance that a client needs to be
    /// disconnected due to a full buffer, but increases potential memory usage.
    ///
    /// # Default Implementation
    ///
    /// An unspecified value is returned that should be adequate in most
    /// situations.
    fn incoming_packet_capacity(&self) -> usize {
        64
    }

    /// Called once at startup to get the capacity of the buffer used to
    /// hold outgoing packets.
    ///
    /// A larger capacity reduces the chance that a client needs to be
    /// disconnected due to a full buffer, but increases potential memory usage.
    ///
    /// # Default Implementation
    ///
    /// An unspecified value is returned that should be adequate in most
    /// situations.
    fn outgoing_packet_capacity(&self) -> usize {
        2048
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
    /// # Default Implementation
    ///
    /// Returns `vec![Dimension::default()]`.
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

    /// Called after the server is created, but prior to accepting connections
    /// and entering the update loop.
    ///
    /// This is useful for performing initialization work with a guarantee that
    /// no connections to the server will be made until this function returns.
    ///
    /// This method is called from within a tokio runtime.
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
#[derive(Debug)]
pub enum ServerListPing<'a> {
    /// Responds to the server list ping with the given information.
    Respond {
        /// Displayed as the number of players on the server.
        online_players: i32,
        /// Displayed as the maximum number of players allowed on the server at
        /// a time.
        max_players: i32,
        /// A description of the server.
        description: Text,
        /// The server's icon as the bytes of a PNG image.
        /// The image must be 64x64 pixels.
        ///
        /// No icon is used if the value is `None`.
        favicon_png: Option<&'a [u8]>,
    },
    /// Ignores the query and disconnects from the client.
    Ignore,
}

#[non_exhaustive]
#[derive(Copy, Clone, Eq, PartialEq, Default)]
pub enum ConnectionMode {
    #[default]
    Online,
    Offline,
    Bungeecord,
    Velocity,
}
