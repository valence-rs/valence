//! Dimension configuration and identification.

use crate::ident;
use crate::protocol_inner::packets::s2c::play::DimensionType;

/// Identifies a particular [`Dimension`] on the server.
///
/// The default dimension ID refers to the first dimension added in the server's
/// [configuration](crate::config::Config).
///
/// To obtain dimension IDs for other dimensions, call
/// [`dimensions`](crate::server::SharedServer::dimensions).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DimensionId(pub(crate) u16);

/// The default dimension ID corresponds to the first element in the `Vec`
/// returned by [`crate::config::Config::dimensions`].
impl Default for DimensionId {
    fn default() -> Self {
        Self(0)
    }
}

/// Contains the configuration for a dimension type.
///
/// On creation, each [`World`] in Valence is assigned a dimension. The
/// dimension determines certain properties of the world such as its height and
/// ambient lighting.
///
/// In Minecraft, "dimension" and "dimension type" are two distinct concepts.
/// For instance, the Overworld and Nether are dimensions, each with
/// their own dimension type. A dimension in this library is analogous to a
/// [`World`] while [`Dimension`] represents a dimension type.
///
/// [`World`]: crate::world::World
#[derive(Clone, Debug)]
pub struct Dimension {
    /// When false, compasses will spin randomly.
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
    // * monster_spawn_light_level
    // * monster_spawn_block_light_level
    // * respawn_anchor_works
    // * has_skylight
    // * bed_works
    // * has_raids
    // * logical_height
    // * coordinate_scale
    // * ultrawarm
    // * has_ceiling
}

impl Dimension {
    pub(crate) fn to_dimension_registry_item(&self) -> DimensionType {
        DimensionType {
            piglin_safe: true,
            has_raids: true,
            monster_spawn_light_level: 0,
            monster_spawn_block_light_limit: 0,
            natural: self.natural,
            ambient_light: self.ambient_light,
            fixed_time: self.fixed_time.map(|t| t as i64),
            infiniburn: "#minecraft:infiniburn_overworld".into(),
            respawn_anchor_works: true,
            has_skylight: true,
            bed_works: true,
            effects: match self.effects {
                DimensionEffects::Overworld => ident!("overworld"),
                DimensionEffects::TheNether => ident!("the_nether"),
                DimensionEffects::TheEnd => ident!("the_end"),
            },
            min_y: self.min_y,
            height: self.height,
            logical_height: self.height,
            coordinate_scale: 1.0,
            ultrawarm: false,
            has_ceiling: false,
        }
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Self {
            natural: true,
            ambient_light: 1.0,
            fixed_time: None,
            effects: DimensionEffects::default(),
            min_y: -64,
            height: 384,
        }
    }
}

/// Determines what skybox/fog effects to use in dimensions.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DimensionEffects {
    #[default]
    Overworld,
    TheNether,
    TheEnd,
}
