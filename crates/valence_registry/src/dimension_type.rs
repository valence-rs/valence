//! Contains dimension types and the dimension type registry. Minecraft's
//! default dimensions are added to the registry by default.
//!
//! ### **NOTE:**
//! - Modifying the dimension type registry after the server has started can
//! break invariants within instances and clients! Make sure there are no
//! instances or clients spawned before mutating.

use std::ops::{Deref, DerefMut};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::error;
use valence_ident::{ident, Ident};
use valence_nbt::serde::CompoundSerializer;

use crate::codec::{RegistryCodec, RegistryValue};
use crate::{Registry, RegistryIdx, RegistrySet};
pub struct DimensionTypePlugin;

impl Plugin for DimensionTypePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DimensionTypeRegistry>()
            .add_systems(PreStartup, load_default_dimension_types)
            .add_systems(
                PostUpdate,
                update_dimension_type_registry.before(RegistrySet),
            );
    }
}

/// Loads the default dimension types from the registry codec.
fn load_default_dimension_types(mut reg: ResMut<DimensionTypeRegistry>, codec: Res<RegistryCodec>) {
    let mut helper = move || -> anyhow::Result<()> {
        for value in codec.registry(DimensionTypeRegistry::KEY) {
            let mut dimension_type = DimensionType::deserialize(value.element.clone())?;

            // HACK: We don't have a lighting engine implemented. To avoid shrouding the
            // world in darkness, give all dimensions the max ambient light.
            dimension_type.ambient_light = 1.0;

            reg.insert(value.name.clone(), dimension_type);
        }

        Ok(())
    };

    if let Err(e) = helper() {
        error!("failed to load default dimension types from registry codec: {e:#}");
    }
}

/// Updates the registry codec as the dimension type registry is modified by
/// users.
fn update_dimension_type_registry(
    reg: Res<DimensionTypeRegistry>,
    mut codec: ResMut<RegistryCodec>,
) {
    if reg.is_changed() {
        let dimension_types = codec.registry_mut(DimensionTypeRegistry::KEY);

        dimension_types.clear();

        dimension_types.extend(reg.iter().map(|(_, name, dim)| {
            RegistryValue {
                name: name.into(),
                element: dim
                    .serialize(CompoundSerializer)
                    .expect("failed to serialize dimension type"),
            }
        }));
    }
}

#[derive(Resource, Default, Debug)]
pub struct DimensionTypeRegistry {
    reg: Registry<DimensionTypeId, DimensionType>,
}

impl DimensionTypeRegistry {
    pub const KEY: Ident<&str> = ident!("dimension_type");
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct DimensionTypeId(u16);

impl RegistryIdx for DimensionTypeId {
    const MAX: usize = u16::MAX as _;

    fn to_index(self) -> usize {
        self.0 as _
    }

    fn from_index(idx: usize) -> Self {
        Self(idx as _)
    }
}

impl Deref for DimensionTypeRegistry {
    type Target = Registry<DimensionTypeId, DimensionType>;

    fn deref(&self) -> &Self::Target {
        &self.reg
    }
}

impl DerefMut for DimensionTypeRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reg
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct DimensionType {
    pub ambient_light: f32,
    pub bed_works: bool,
    pub coordinate_scale: f64,
    pub effects: DimensionEffects,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_time: Option<i32>,
    pub has_ceiling: bool,
    pub has_raids: bool,
    pub has_skylight: bool,
    pub height: i32,
    pub infiniburn: String,
    pub logical_height: i32,
    pub min_y: i32,
    pub monster_spawn_block_light_limit: i32,
    pub monster_spawn_light_level: MonsterSpawnLightLevel,
    pub natural: bool,
    pub piglin_safe: bool,
    pub respawn_anchor_works: bool,
    pub ultrawarm: bool,
}

impl Default for DimensionType {
    fn default() -> Self {
        Self {
            ambient_light: 0.0,
            bed_works: true,
            coordinate_scale: 1.0,
            effects: DimensionEffects::default(),
            fixed_time: None,
            has_ceiling: false,
            has_raids: true,
            has_skylight: true,
            height: 384,
            infiniburn: "#minecraft:infiniburn_overworld".into(),
            logical_height: 384,
            min_y: -64,
            monster_spawn_block_light_limit: 0,
            monster_spawn_light_level: MonsterSpawnLightLevel::Int(7),
            natural: true,
            piglin_safe: false,
            respawn_anchor_works: false,
            ultrawarm: false,
        }
    }
}

/// Determines what skybox/fog effects to use in dimensions.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DimensionEffects {
    #[serde(rename = "minecraft:overworld")]
    #[default]
    Overworld,
    #[serde(rename = "minecraft:the_nether")]
    TheNether,
    #[serde(rename = "minecraft:the_end")]
    TheEnd,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MonsterSpawnLightLevel {
    Int(i32),
    Tagged(MonsterSpawnLightLevelTagged),
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum MonsterSpawnLightLevelTagged {
    #[serde(rename = "minecraft:uniform")]
    Uniform {
        min_inclusive: i32,
        max_inclusive: i32,
    },
}

impl From<i32> for MonsterSpawnLightLevel {
    fn from(value: i32) -> Self {
        Self::Int(value)
    }
}
