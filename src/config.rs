// TODO: rate limit, view distance?

use std::any::Any;
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use anyhow::ensure;
use async_trait::async_trait;
use tokio::runtime::Handle as TokioHandle;

use crate::server::{start_server, ShutdownError};
use crate::{ident, Identifier, NewClientData, Server, SharedServer, ShutdownResult, Text};

/// A builder type used to configure and start the server.
pub struct ServerConfig {
    pub(crate) handler: Option<Box<dyn Handler>>,
    pub(crate) address: SocketAddr,
    pub(crate) update_duration: Duration,
    pub(crate) online_mode: bool,
    pub(crate) max_clients: usize,
    pub(crate) clientbound_packet_capacity: usize,
    pub(crate) serverbound_packet_capacity: usize,
    pub(crate) tokio_handle: Option<TokioHandle>,
    pub(crate) dimensions: Vec<Dimension>,
    pub(crate) biomes: Vec<Biome>,
}

impl ServerConfig {
    /// Constructs a new server configuration with the provided handler.
    pub fn new() -> Self {
        Self {
            handler: None,
            address: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 25565).into(),
            update_duration: Duration::from_secs_f64(1.0 / 20.0),
            online_mode: false,
            max_clients: 32,
            clientbound_packet_capacity: 128,
            serverbound_packet_capacity: 32,
            tokio_handle: None,
            dimensions: Vec::new(),
            biomes: Vec::new(),
        }
    }

    /// Sets the [`Handler`] to use for this server.
    pub fn handler(&mut self, handler: impl Handler) {
        self.handler = Some(Box::new(handler));
    }

    /// Sets the socket address that the server will be bound to.
    ///
    /// The default is `127.0.0.1:25565`.
    pub fn address(&mut self, addr: impl Into<SocketAddr>) {
        self.address = addr.into();
    }

    /// Sets the duration of each game update.
    ///
    /// On each game update (a.k.a. tick), the server is expected to update game
    /// logic and respond to packets from clients. Once this is complete,
    /// the server will sleep for any remaining time until the full update
    /// duration has passed.
    ///
    /// If the server is running behind schedule due to heavy load or some other
    /// reason, the actual duration of a game update will exceed what has been
    /// specified.
    ///
    /// The duration must be nonzero.
    ///
    /// The default value is the same as Minecraft's official server (20 ticks
    /// per second). You may want to use a shorter duration if you can afford to
    /// do so.
    pub fn update_duration(&mut self, duration: Duration) {
        self.update_duration = duration;
    }

    /// Sets the state of "online mode", which determines if client
    /// authentication and encryption should occur.
    ///
    /// When online mode is disabled, malicious clients can give themselves any
    /// username and UUID they want, potentially gaining privileges they
    /// might not otherwise have. Additionally, encryption is only enabled in
    /// online mode. For these reasons online mode should only be disabled
    /// for development purposes and enabled on servers exposed to the
    /// internet.
    ///
    /// By default, online mode is enabled.
    pub fn online_mode(&mut self, online_mode: bool) {
        self.online_mode = online_mode;
    }

    /// Sets the maximum number of clients (past the login stage) allowed on the
    /// server simultaneously.
    ///
    /// The default is 32.
    pub fn max_clients(&mut self, clients: usize) {
        self.max_clients = clients;
    }

    /// The capacity of the buffer used to hold clientbound packets.
    ///
    /// A larger capcity reduces the chance of packet loss but increases
    /// potential memory usage. The default value is unspecified but should be
    /// adequate for most situations.
    ///
    /// The capacity must be nonzero.
    pub fn clientbound_packet_capacity(&mut self, cap: usize) {
        self.clientbound_packet_capacity = cap;
    }

    /// Sets the capacity of the buffer used to hold serverbound packets.
    ///
    /// A larger capcity reduces the chance of packet loss but increases
    /// potential memory usage. The default value is unspecified but should be
    /// adequate for most situations.
    ///
    /// The capacity must be nonzero.
    pub fn serverbound_packet_capacity(&mut self, cap: usize) {
        self.serverbound_packet_capacity = cap;
    }

    /// Sets the handle to the tokio runtime the server will use.
    ///
    /// If a handle is not provided, the server will create its own tokio
    /// runtime.
    pub fn tokio_handle(&mut self, handle: TokioHandle) {
        self.tokio_handle = Some(handle);
    }

    /// Adds a new dimension to the server which is identified by the returned
    /// [`DimensionId`]. The default dimension is added if none are provided.
    ///
    /// Additionally, the documented requirements on the fields of [`Dimension`]
    /// must be met. No more than `u16::MAX` dimensions may be added.
    pub fn push_dimension(&mut self, dimension: Dimension) -> DimensionId {
        let id = self.biomes.len();
        self.dimensions.push(dimension);
        DimensionId(id as u16)
    }

    /// Adds a new biome to the server which is identified by the returned
    /// [`BiomeId`]. The default biome is added if none are provided.
    ///
    /// Additionally, the documented requirements on the fields of [`Biome`]
    /// must be met. No more than `u16::MAX` biomes may be added.
    pub fn push_biome(&mut self, biome: Biome) -> BiomeId {
        let id = self.biomes.len();
        self.biomes.push(biome);
        BiomeId(id as u16)
    }

    /// Consumes the configuration and starts the server.
    ///
    /// The function returns once the server has been shut down, a runtime error
    /// occurs, or the configuration is invalid.
    pub fn start(mut self) -> ShutdownResult {
        if self.biomes.is_empty() {
            self.biomes.push(Biome::default());
        }

        if self.dimensions.is_empty() {
            self.dimensions.push(Dimension::default());
        }

        self.validate().map_err(ShutdownError::from)?;
        start_server(self)
    }

    fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.dimensions.len() <= u16::MAX as usize,
            "more than u16::MAX dimensions added"
        );

        ensure!(
            self.biomes.len() <= u16::MAX as usize,
            "more than u16::MAX biomes added"
        );

        ensure!(
            self.update_duration != Duration::ZERO,
            "update duration must be nonzero"
        );

        ensure!(
            self.clientbound_packet_capacity > 0,
            "clientbound packet capacity must be nonzero"
        );

        ensure!(
            self.serverbound_packet_capacity > 0,
            "serverbound packet capacity must be nonzero"
        );

        for (i, dim) in self.dimensions.iter().enumerate() {
            ensure!(
                dim.min_y % 16 == 0 && (-2032..=2016).contains(&dim.min_y),
                "invalid min_y in dimension #{i}",
            );

            ensure!(
                dim.height % 16 == 0
                    && (0..=4064).contains(&dim.height)
                    && dim.min_y.saturating_add(dim.height) <= 2032,
                "invalid height in dimension #{i}",
            );

            ensure!(
                (0.0..=1.0).contains(&dim.ambient_light),
                "ambient_light is out of range in dimension #{i}",
            );

            if let Some(fixed_time) = dim.fixed_time {
                assert!(
                    (0..=24_000).contains(&fixed_time),
                    "fixed_time is out of range in dimension #{i}",
                );
            }
        }

        let mut names = HashSet::new();

        for biome in self.biomes.iter() {
            ensure!(
                names.insert(biome.name.clone()),
                "biome \"{}\" already added",
                biome.name
            );
        }

        Ok(())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// A trait containing callbacks which are invoked by the running Minecraft
/// server.
///
/// The handler is used from multiple threads and must therefore implement
/// `Send` and `Sync`. From within a single thread, callbacks are never invoked
/// recursively. In other words, a mutex can be aquired at the beginning of a
/// callback and released at the end without risk of deadlocking.
///
/// All methods are called from within a tokio context.
#[async_trait]
#[allow(unused_variables)]
pub trait Handler: Any + Send + Sync {
    /// Called after the server is created, but prior to accepting connections
    /// and entering the update loop.
    ///
    /// This is useful for performing initialization work with a guarantee that
    /// no connections to the server will be made until this function returns.
    ///
    /// # Default Implementation
    /// The default implementation does nothing.
    fn init(&self, server: &mut Server) {}

    /// Called once at the beginning of every server update (also known as
    /// a "tick").
    ///
    /// The frequency of server updates can be configured by `update_duration`
    /// in [`ServerConfig`].
    ///
    /// # Default Implementation
    /// The default implementation does nothing.
    fn update(&self, server: &mut Server) {}

    /// Called when the server receives a Server List Ping query.
    /// Data for the query can be provided or the query can be ignored.
    ///
    /// # Default Implementation
    /// A placeholder response is returned.
    async fn server_list_ping(
        &self,
        server: &SharedServer,
        remote_addr: SocketAddr,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: server.client_count() as i32,
            max_players: server.max_clients() as i32,
            description: "A Minecraft Server".into(),
            favicon_png: None,
        }
    }

    /// Called when a client is disconnected due to the server being full.
    /// The return value is the disconnect message to use.
    ///
    /// # Default Implementation
    /// A placeholder message is returned.
    async fn max_client_message(&self, server: &SharedServer, npd: &NewClientData) -> Text {
        // TODO: Standard translated text for this purpose?
        "The server is full!".into()
    }

    /// Called asynchronously for each client after successful authentication
    /// (if online mode is enabled) to determine if they are allowed to join the
    /// server. On success, a client-backed entity is spawned.
    ///
    /// This function is the appropriate place to perform
    /// whitelist checks, database queries, etc.
    ///
    /// # Default Implementation
    /// The client is allowed to join unconditionally.
    async fn login(&self, server: &SharedServer, ncd: &NewClientData) -> Login {
        Login::Join
    }
}

/// The result of the [`server_list_ping`](Handler::server_list_ping) callback.
pub enum ServerListPing {
    /// Responds to the server list ping with the given information.
    Respond {
        online_players: i32,
        max_players: i32,
        description: Text,
        /// The server's icon as the bytes of a PNG image.
        /// The image must be 64x64 pixels.
        ///
        /// No icon is used if the value is `None`.
        favicon_png: Option<Arc<[u8]>>,
    },
    /// Ignores the query and disconnects from the client.
    Ignore,
}

/// The result of the [`login`](Handler::login) callback.
#[derive(Debug)]
pub enum Login {
    /// The client may join the server.
    Join,
    /// The client may not join the server and will be disconnected with the
    /// provided reason.
    Disconnect(Text),
}

/// Identifies a particular [`Dimension`].
///
/// Dimension IDs are always valid and are cheap to copy and store.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Debug)]
pub struct DimensionId(pub(crate) u16);

/// Contains the configuration for a custom dimension type.
///
/// In Minecraft, "dimension" and "dimension type" are two different concepts.
/// For instance, the Overworld and Nether are dimensions, each with
/// their own dimension type. A dimension in this library is analogous to a
/// [`World`](crate::World) while the [`Dimension`] struct represents a
/// dimension type.
#[derive(Clone, Debug)]
pub struct Dimension {
    /// When false, compases will spin randomly.
    pub natural: bool,
    /// Must be between 0.0 and 1.0.
    pub ambient_light: f32,
    /// Must be between 0 and 24000.
    pub fixed_time: Option<u16>,
    /// Determines what skybox/fog effects to use.
    pub effects: DimensionEffects,
    /// The minimum height in which blocks can exist in this dimension.
    ///
    /// `min_y` must meet the following conditions:
    /// * `min_y % 16 == 0`
    /// * `-2032 <= min_y <= 2016`
    pub min_y: i32,
    /// The total height in which blocks can exist in this dimension.
    ///
    /// `height` must meet the following conditions:
    /// * `height % 16 == 0`
    /// * `0 <= height <= 4064`
    /// * `min_y + height <= 2032`
    pub height: i32,
    // TODO: The following fields should be added if they can affect the
    // appearance of the dimension to clients.
    // * infiniburn
    // * respawn_anchor_works
    // * has_skylight
    // * bed_works
    // * has_raids
    // * logical_height
    // * coordinate_scale
    // * ultrawarm
    // * has_ceiling
}

impl Default for Dimension {
    fn default() -> Self {
        Self {
            natural: true,
            ambient_light: 0.0,
            fixed_time: None,
            effects: DimensionEffects::Overworld,
            min_y: -64,
            height: 384,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DimensionEffects {
    Overworld,
    TheNether,
    TheEnd,
}

/// Identifies a particular [`Biome`].
///
/// Biome IDs are always valid and are cheap to copy and store.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BiomeId(pub(crate) u16);

/// Contains the configuration for a custom biome.
#[derive(Clone, Debug)]
pub struct Biome {
    /// The unique name for this biome. The name can be
    /// seen in the F3 debug menu.
    pub name: Identifier,
    pub precipitation: BiomePrecipitation,
    pub sky_color: u32,
    pub water_fog_color: u32,
    pub fog_color: u32,
    pub water_color: u32,
    pub foliage_color: Option<u32>,
    pub grass_color_modifier: BiomeGrassColorModifier,
    pub music: Option<BiomeMusic>,
    pub ambient_sound: Option<Identifier>,
    pub additions_sound: Option<BiomeAdditionsSound>,
    pub mood_sound: Option<BiomeMoodSound>,
    pub particle: Option<BiomeParticle>,
    // TODO: The following fields should be added if they can affect the appearance of the biome to
    // clients.
    // * depth: f32
    // * temperature: f32
    // * scale: f32
    // * downfall: f32
    // * category
    // * temperature_modifier
    // * grass_color (misleading name?)
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            name: ident!("plains"),
            precipitation: BiomePrecipitation::Rain,
            sky_color: 7907327,
            water_fog_color: 329011,
            fog_color: 12638463,
            water_color: 4159204,
            foliage_color: None,
            grass_color_modifier: BiomeGrassColorModifier::None,
            music: None,
            ambient_sound: None,
            additions_sound: None,
            mood_sound: None,
            particle: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BiomePrecipitation {
    Rain,
    Snow,
    None,
}

impl Default for BiomePrecipitation {
    fn default() -> Self {
        Self::Rain
    }
}

/// Minecraft handles grass colors for swamps and dark oak forests in a special
/// way.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BiomeGrassColorModifier {
    Swamp,
    DarkForest,
    None,
}

impl Default for BiomeGrassColorModifier {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug)]
pub struct BiomeMusic {
    pub replace_current_music: bool,
    pub sound: Identifier,
    pub min_delay: i32,
    pub max_delay: i32,
}

#[derive(Clone, Debug)]
pub struct BiomeAdditionsSound {
    pub sound: Identifier,
    pub tick_chance: f64,
}

#[derive(Clone, Debug)]
pub struct BiomeMoodSound {
    pub sound: Identifier,
    pub tick_delay: i32,
    pub offset: f64,
    pub block_search_extent: i32,
}

#[derive(Clone, Debug)]
pub struct BiomeParticle {
    pub probability: f32,
    pub typ: Identifier,
}
