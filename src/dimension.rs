/// A handle to a particular [`Dimension`] on the server.
///
/// Dimension IDs must only be used on servers from which they originate.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DimensionId(pub(crate) u16);

impl DimensionId {
    pub fn to_index(self) -> usize {
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
