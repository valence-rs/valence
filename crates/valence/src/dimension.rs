//! Dimension configuration and identification.

use std::collections::HashSet;

use anyhow::ensure;
use valence_nbt::{compound, Compound};
use valence_protocol::ident;
use valence_protocol::ident::Ident;

/// Identifies a particular [`Dimension`] on the server.
///
/// The default dimension ID refers to the first dimension added in
/// [`ServerPlugin::dimensions`].
///
/// To obtain dimension IDs for other dimensions, look at
/// [`ServerPlugin::dimensions`].
///
/// [`ServerPlugin::dimensions`]: crate::config::ServerPlugin::dimensions
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DimensionId(pub(crate) u16);

/// The default dimension ID corresponds to the first element in the `Vec`
/// returned by [`ServerPlugin::dimensions`].
///
/// [`ServerPlugin::dimensions`]: crate::config::ServerPlugin::dimensions
impl Default for DimensionId {
    fn default() -> Self {
        Self(0)
    }
}

/// Contains the configuration for a dimension type.
///
/// On creation, each [`Instance`] in Valence is assigned a dimension. The
/// dimension determines certain properties of the world such as its height and
/// ambient lighting.
///
/// In Minecraft, "dimension" and "dimension type" are two distinct concepts.
/// For instance, the Overworld and Nether are dimensions, each with
/// their own dimension type. A dimension in this library is analogous to a
/// [`Instance`] while [`Dimension`] represents a dimension type.
///
/// [`Instance`]: crate::instance::Instance
#[derive(Clone, Debug)]
pub struct Dimension {
    /// The unique name for this dimension.
    pub name: Ident<String>,
    /// When false, compasses will spin randomly.
    pub natural: bool,
    /// Must be between 0.0 and 1.0.
    pub ambient_light: f32,
    /// Must be between 0 and 24000.
    pub fixed_time: Option<u16>,
    /// Determines what skybox/fog effects to use.
    pub effects: DimensionEffects,
    /// The minimum Y coordinate in which blocks can exist in this dimension.
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
    // TODO: add other fields.
    //       * infiniburn
    //       * monster_spawn_light_level
    //       * monster_spawn_block_light_level
    //       * respawn_anchor_works
    //       * has_skylight
    //       * bed_works
    //       * has_raids
    //       * logical_height
    //       * coordinate_scale
    //       * ultrawarm
    //       * has_ceiling
}

impl Dimension {
    pub(crate) fn to_dimension_registry_item(&self, id: i32) -> Compound {
        compound! {
            "name" => self.name.clone(),
            "id" => id,
            "element" => {
                let mut element = compound! {
                    "piglin_safe" => true,
                    "has_raids" => true,
                    "monster_spawn_light_level" => 0,
                    "monster_spawn_block_light_limit" => 0,
                    "natural" => self.natural,
                    "ambient_light" => self.ambient_light,
                    "infiniburn" => "#minecraft:infiniburn_overworld",
                    "respawn_anchor_works" => true,
                    "has_skylight" => true,
                    "bed_works" => true,
                    "effects" => match self.effects {
                        DimensionEffects::Overworld => "overworld",
                        DimensionEffects::TheNether => "the_nether",
                        DimensionEffects::TheEnd => "the_end",
                    },
                    "min_y" => self.min_y,
                    "height" => self.height,
                    "logical_height" => self.height,
                    "coordinate_scale" => 1.0,
                    "ultrawarm" => false,
                    "has_ceiling" => false,
                };

                if let Some(t) = self.fixed_time {
                    element.insert("fixed_time", t as i64);
                }

                element
            },
        }
    }
}

pub(crate) fn validate_dimensions(dimensions: &[Dimension]) -> anyhow::Result<()> {
    ensure!(
        !dimensions.is_empty(),
        "at least one dimension must be present"
    );

    ensure!(
        dimensions.len() <= u16::MAX as usize,
        "more than u16::MAX dimensions present"
    );

    let mut names = HashSet::new();

    for dim in dimensions {
        let name = &dim.name;

        ensure!(
            names.insert(name.clone()),
            "dimension \"{name}\" already exists",
        );

        ensure!(
            dim.min_y % 16 == 0 && (-2032..=2016).contains(&dim.min_y),
            "invalid min_y in dimension {name}",
        );

        ensure!(
            dim.height % 16 == 0
                && (0..=4064).contains(&dim.height)
                && dim.min_y.saturating_add(dim.height) <= 2032,
            "invalid height in dimension {name}",
        );

        ensure!(
            (0.0..=1.0).contains(&dim.ambient_light),
            "ambient_light is out of range in dimension {name}",
        );

        if let Some(fixed_time) = dim.fixed_time {
            ensure!(
                (0..=24_000).contains(&fixed_time),
                "fixed_time is out of range in dimension {name}",
            );
        }
    }

    Ok(())
}

impl Default for Dimension {
    fn default() -> Self {
        Self {
            name: ident!("overworld"),
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
