use std::any::Any;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::panic::{RefUnwindSafe, UnwindSafe};

use async_trait::async_trait;
use tokio::runtime::Handle as TokioHandle;

use crate::{ident, Id, Identifier, NewClientData, Server, SharedServer, Text, Ticks};

/// A trait containing callbacks which are invoked by the running Minecraft
/// server.
///
/// The config is used from multiple threads and must therefore implement
/// `Send` and `Sync`. From within a single thread, methods are never invoked
/// recursively. In other words, a mutex can always be aquired at the beginning
/// of a method and released at the end without risk of deadlocking.
///
/// This trait uses the [async_trait](https://docs.rs/async-trait/latest/async_trait/) attribute macro.
/// This will be removed once `impl Trait` in return position in traits is
/// available in stable rust.
#[async_trait]
#[allow(unused_variables)]
pub trait Config: Any + Send + Sync + UnwindSafe + RefUnwindSafe {
    /// Called once at startup to get the maximum number of connections allowed
    /// to the server. Note that this includes all connections, not just those
    /// past the login stage.
    ///
    /// You will want this value to be somewhere above the maximum number of
    /// players, since status pings should still succeed even when the server is
    /// full.
    fn max_connections(&self) -> usize;

    /// Called once at startup to get the socket address the server will
    /// be bound to.
    ///
    /// # Default Implementation
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
    /// # Default Implementation
    /// Returns `20`, which is the same as Minecraft's official server.
    fn tick_rate(&self) -> Ticks {
        20
    }

    /// Called once at startup to get the "online mode" option, which determines
    /// if client authentication and encryption should take place.
    ///
    /// When online mode is disabled, malicious clients can give themselves any
    /// username and UUID they want, potentially gaining privileges they
    /// might not otherwise have. Additionally, encryption is only enabled in
    /// online mode. For these reasons online mode should only be disabled
    /// for development purposes and enabled on servers exposed to the
    /// internet.
    ///
    /// # Default Implementation
    /// Returns `true`.
    fn online_mode(&self) -> bool {
        true
    }

    /// Called once at startup to get the capacity of the buffer used to
    /// hold incoming packets.
    ///
    /// A larger capcity reduces the chance of packet loss but increases
    /// potential memory usage.
    ///
    /// # Default Implementation
    /// An unspecified value is returned that should be adequate in most
    /// situations.
    fn incoming_packet_capacity(&self) -> usize {
        32
    }

    /// Called once at startup to get the capacity of the buffer used to
    /// hold outgoing packets.
    ///
    /// A larger capcity reduces the chance of packet loss due to a full buffer
    /// but increases potential memory usage.
    ///
    /// # Default Implementation
    /// An unspecified value is returned that should be adequate in most
    /// situations.
    fn outgoing_packet_capacity(&self) -> usize {
        128
    }

    /// Called once at startup to get a handle to the tokio runtime the server
    /// will use.
    ///
    /// If a handle is not provided, the server will create its own tokio
    /// runtime.
    ///
    /// # Default Implementation
    /// Returns `None`.
    fn tokio_handle(&self) -> Option<TokioHandle> {
        None
    }

    /// Called once at startup to get the list of [`Dimension`]s usable on the
    /// server.
    ///
    /// The dimensions traversed by [`Server::dimensions`] will be in the same
    /// order as the `Vec` returned by this function.
    ///
    /// The number of elements in the returned `Vec` must be in \[1, u16::MAX].
    /// Additionally, the documented requirements on the fields of [`Dimension`]
    /// must be met.
    ///
    /// # Default Implementation
    /// Returns `vec![Dimension::default()]`.
    fn dimensions(&self) -> Vec<Dimension> {
        vec![Dimension::default()]
    }

    /// Called once at startup to get the list of [`Biome`]s usable on the
    /// server.
    ///
    /// The biomes traversed by [`Server::biomes`] will be in the same
    /// order as the `Vec` returned by this function.
    ///
    /// The number of elements in the returned `Vec` must be in \[1, u16::MAX].
    /// Additionally, the documented requirements on the fields of [`Biome`]
    /// must be met.
    ///
    /// # Default Implementation
    /// Returns `vec![Dimension::default()]`.
    fn biomes(&self) -> Vec<Biome> {
        vec![Biome::default()]
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
    /// The default implementation does nothing.
    fn init(&self, server: &mut Server) {}

    /// Called once at the beginning of every server update (also known as
    /// a "tick").
    ///
    /// The frequency of server updates can be configured by `update_duration`
    /// in [`ServerConfig`].
    ///
    /// This method is called from within a tokio runtime.
    ///
    /// # Default Implementation
    /// The default implementation does nothing.
    fn update(&self, server: &mut Server) {}

    /// Called when the server receives a Server List Ping query.
    /// Data for the response can be provided or the query can be ignored.
    ///
    /// This method is called from within a tokio runtime.
    ///
    /// # Default Implementation
    /// The query is ignored.
    async fn server_list_ping(
        &self,
        server: &SharedServer,
        remote_addr: SocketAddr,
    ) -> ServerListPing {
        ServerListPing::Ignore
    }

    /// Called asynchronously for each client after successful authentication
    /// (if online mode is enabled) to determine if they are allowed to join the
    /// server. On success, a client-backed entity is spawned.
    ///
    /// This function is the appropriate place to perform
    /// player count checks, whitelist checks, database queries, etc.
    ///
    /// This method is called from within a tokio runtime.
    ///
    /// # Default Implementation
    /// The client is allowed to join unconditionally.
    async fn login(&self, server: &SharedServer, ncd: &NewClientData) -> Login {
        Login::Join
    }
}

/// The result of the [`server_list_ping`](Handler::server_list_ping) callback.
#[derive(Debug)]
pub enum ServerListPing<'a> {
    /// Responds to the server list ping with the given information.
    Respond {
        online_players: i32,
        max_players: i32,
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

/// The result of the [`login`](Handler::login) callback.
#[derive(Debug)]
pub enum Login {
    /// The client may join the server.
    Join,
    /// The client may not join the server and will be disconnected with the
    /// provided reason.
    Disconnect(Text),
}

/// A handle to a particular [`Dimension`] on the server.
///
/// Dimension IDs must only be used on servers from which they originate.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DimensionId(pub(crate) u16);

/// All dimension IDs are valid.
impl Id for DimensionId {
    fn idx(self) -> usize {
        self.0 as usize
    }
}

/// The default dimension ID corresponds to the first element in the `Vec`
/// returned by [`Config::dimensions`].
impl Default for DimensionId {
    fn default() -> Self {
        Self(0)
    }
}

/// Contains the configuration for a dimension type.
///
/// In Minecraft, "dimension" and "dimension type" are two different concepts.
/// For instance, the Overworld and Nether are dimensions, each with
/// their own dimension type. A dimension in this library is analogous to a
/// [`World`](crate::World) while [`Dimension`] represents a
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
            ambient_light: 1.0,
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

/// All Biome IDs are valid.
impl Id for BiomeId {
    fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Contains the configuration for a biome.
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
