//! Dimension type configuration and identification.
//!
//! **NOTE:**
//!
//! - Modifying the dimension type registry after the server has started can
//! break invariants within instances and clients! Make sure there are no
//! instances or clients spawned before mutating.

use std::collections::BTreeMap;
use std::str::FromStr;

use anyhow::{bail, Context};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tracing::{error, warn};
use valence_core::ident;
use valence_core::ident::Ident;
use valence_nbt::{compound, Value};
use valence_registry::{RegistryCodec, RegistryCodecSet, RegistryValue};

pub struct DimensionPlugin;

impl Plugin for DimensionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DimensionTypeRegistry {
            name_to_dimension: BTreeMap::new(),
        })
        .add_systems(
            (
                update_dimension_type_registry,
                remove_dimension_types_from_registry,
            )
                .chain()
                .in_base_set(CoreSet::PostUpdate)
                .before(RegistryCodecSet),
        )
        .add_startup_system(load_default_dimension_types.in_base_set(StartupSet::PreStartup));
    }
}

fn update_dimension_type_registry(
    mut reg: ResMut<DimensionTypeRegistry>,
    mut codec: ResMut<RegistryCodec>,
    dimension_types: Query<(Entity, &DimensionType), Changed<DimensionType>>,
) {
    for (entity, dim) in &dimension_types {
        // In case the name was changed.
        reg.name_to_dimension.insert(dim.name.clone(), entity);

        let dimension_type_compound = compound! {
            "ambient_light" => dim.ambient_light,
            "bed_works" => dim.bed_works,
            "coordinate_scale" => dim.coordinate_scale,
            "effects" => Ident::from(dim.effects),
            "has_ceiling" => dim.has_ceiling,
            "has_raids" => dim.has_raids,
            "has_skylight" => dim.has_skylight,
            "height" => dim.height,
            "infiniburn" => &dim.infiniburn,
            "logical_height" => dim.logical_height,
            "min_y" => dim.min_y,
            "monster_spawn_block_light_limit" => dim.monster_spawn_block_light_limit,
            "natural" => dim.natural,
            "piglin_safe" => dim.piglin_safe,
            "respawn_anchor_works" => dim.respawn_anchor_works,
            "ultrawarm" => dim.ultrawarm,
        };

        let dimension_type_reg = codec.registry_mut(DimensionTypeRegistry::KEY);

        if let Some(value) = dimension_type_reg.iter_mut().find(|v| v.name == dim.name) {
            value.name = dim.name.clone();
            value.element.merge(dimension_type_compound);
        } else {
            dimension_type_reg.push(RegistryValue {
                name: dim.name.clone(),
                element: dimension_type_compound,
            });
        }
    }
}

fn remove_dimension_types_from_registry(
    mut reg: ResMut<DimensionTypeRegistry>,
    mut codec: ResMut<RegistryCodec>,
    mut dimension_types: RemovedComponents<DimensionType>,
) {
    for entity in dimension_types.iter() {
        if let Some((name, _)) = reg.name_to_dimension.iter().find(|(_, &e)| e == entity) {
            let name = name.clone();
            reg.name_to_dimension.remove(name.as_str());

            let dimension_type_reg = codec.registry_mut(DimensionTypeRegistry::KEY);

            if let Some(idx) = dimension_type_reg.iter().position(|v| v.name == name) {
                dimension_type_reg.remove(idx);
            }
        }
    }
}

fn load_default_dimension_types(
    mut reg: ResMut<DimensionTypeRegistry>,
    codec: Res<RegistryCodec>,
    mut commands: Commands,
) {
    let mut helper = move || -> anyhow::Result<()> {
        for value in codec.registry(DimensionTypeRegistry::KEY) {
            macro_rules! get {
                ($name:literal, $f:expr) => {{
                    value
                        .element
                        .get($name)
                        .and_then($f)
                        .context(concat!("invalid ", $name))?
                }};
            }

            let entity = commands
                .spawn(DimensionType {
                    name: value.name.clone(),
                    ambient_light: *get!("ambient_light", Value::as_float),
                    bed_works: *get!("bed_works", Value::as_byte) != 0,
                    coordinate_scale: *get!("coordinate_scale", Value::as_double),
                    effects: DimensionEffects::from_str(get!("effects", Value::as_string))?,
                    has_ceiling: *get!("has_ceiling", Value::as_byte) != 0,
                    has_raids: *get!("has_raids", Value::as_byte) != 0,
                    has_skylight: *get!("has_skylight", Value::as_byte) != 0,
                    height: *get!("height", Value::as_int),
                    infiniburn: get!("infiniburn", Value::as_string).clone(),
                    logical_height: *get!("logical_height", Value::as_int),
                    min_y: *get!("min_y", Value::as_int),
                    monster_spawn_block_light_limit: *get!(
                        "monster_spawn_block_light_limit",
                        Value::as_int
                    ),
                    natural: *get!("natural", Value::as_byte) != 0,
                    piglin_safe: *get!("piglin_safe", Value::as_byte) != 0,
                    respawn_anchor_works: *get!("respawn_anchor_works", Value::as_byte) != 0,
                    ultrawarm: *get!("ultrawarm", Value::as_byte) != 0,
                })
                .id();

            if reg
                .name_to_dimension
                .insert(value.name.clone(), entity)
                .is_some()
            {
                warn!("duplicate dimension type name of \"{}\"", &value.name);
            }
        }

        Ok(())
    };

    if let Err(e) = helper() {
        error!("failed to load default dimension types from registry codec: {e:#}");
    }
}

#[derive(Resource)]
pub struct DimensionTypeRegistry {
    name_to_dimension: BTreeMap<Ident<String>, Entity>,
}

impl DimensionTypeRegistry {
    pub const KEY: Ident<&str> = ident!("minecraft:dimension_type");

    pub fn get_by_name(&self, name: Ident<&str>) -> Option<Entity> {
        self.name_to_dimension.get(name.as_str()).copied()
    }

    pub fn dimensions(&self) -> impl Iterator<Item = Entity> + '_ {
        self.name_to_dimension.values().copied()
    }
}

#[derive(Component, Clone, PartialEq, Debug)]
pub struct DimensionType {
    pub name: Ident<String>,
    pub ambient_light: f32,
    pub bed_works: bool,
    pub coordinate_scale: f64,
    pub effects: DimensionEffects,
    pub has_ceiling: bool,
    pub has_raids: bool,
    pub has_skylight: bool,
    pub height: i32,
    pub infiniburn: String,
    pub logical_height: i32,
    pub min_y: i32,
    pub monster_spawn_block_light_limit: i32,
    /// TODO: monster_spawn_light_level
    pub natural: bool,
    pub piglin_safe: bool,
    pub respawn_anchor_works: bool,
    pub ultrawarm: bool,
}

impl Default for DimensionType {
    fn default() -> Self {
        Self {
            name: ident!("minecraft:overworld").into(),
            ambient_light: 1.0,
            bed_works: true,
            coordinate_scale: 1.0,
            effects: DimensionEffects::default(),
            has_ceiling: false,
            has_raids: true,
            has_skylight: true,
            height: 384,
            infiniburn: "#minecraft:infiniburn_overworld".into(),
            logical_height: 384,
            min_y: -64,
            monster_spawn_block_light_limit: 0,
            natural: true,
            piglin_safe: false,
            respawn_anchor_works: true,
            ultrawarm: false,
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

impl From<DimensionEffects> for Ident<&'static str> {
    fn from(value: DimensionEffects) -> Self {
        match value {
            DimensionEffects::Overworld => ident!("overworld"),
            DimensionEffects::TheNether => ident!("the_nether"),
            DimensionEffects::TheEnd => ident!("the_end"),
        }
    }
}

impl FromStr for DimensionEffects {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Ident::new(s)?.as_str() {
            "minecraft:overworld" => Ok(DimensionEffects::Overworld),
            "minecraft:the_nether" => Ok(DimensionEffects::TheNether),
            "minecraft:the_end" => Ok(DimensionEffects::TheEnd),
            other => bail!("unknown dimension effect \"{other}\""),
        }
    }
}
