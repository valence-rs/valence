//! Biome configuration and identification.
//!
//! **NOTE:**
//!
//! - Modifying the biome registry after the server has started can
//! break invariants within instances and clients! Make sure there are no
//! instances or clients spawned before mutating.
//! - A biome named "minecraft:plains" must exist. Otherwise, vanilla clients
//!   will be disconnected. A biome named "minecraft:plains" is added by
//!   default.

use std::ops::Index;

use anyhow::{bail, Context};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tracing::error;
use valence_core::ident;
use valence_core::ident::Ident;
use valence_nbt::{compound, Value};
use valence_registry::{RegistryCodec, RegistryCodecSet, RegistryValue};

pub struct BiomePlugin;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]

struct BiomeSet;

impl Plugin for BiomePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(BiomeRegistry {
            id_to_biome: vec![],
        })
        .configure_set(
            BiomeSet
                .in_base_set(CoreSet::PostUpdate)
                .before(RegistryCodecSet),
        )
        .add_systems(
            (update_biome_registry, remove_biomes_from_registry)
                .chain()
                .in_set(BiomeSet),
        )
        .add_startup_system(load_default_biomes.in_base_set(StartupSet::PreStartup));
    }
}

fn load_default_biomes(
    mut reg: ResMut<BiomeRegistry>,
    codec: Res<RegistryCodec>,
    mut commands: Commands,
) {
    let mut helper = move || {
        for value in codec.registry(BiomeRegistry::KEY) {
            let downfall = *value
                .element
                .get("downfall")
                .and_then(|v| v.as_float())
                .context("invalid downfall")?;

            let Some(Value::Compound(effects)) = value.element.get("effects") else {
                bail!("missing biome effects")
            };

            let fog_color = *effects
                .get("fog_color")
                .and_then(|v| v.as_int())
                .context("invalid fog color")?;

            let sky_color = *effects
                .get("sky_color")
                .and_then(|v| v.as_int())
                .context("invalid sky color")?;

            let water_color = *effects
                .get("water_color")
                .and_then(|v| v.as_int())
                .context("invalid water color")?;

            let water_fog_color = *effects
                .get("water_fog_color")
                .and_then(|v| v.as_int())
                .context("invalid water fog color")?;

            let grass_color = effects.get("grass_color").and_then(|v| v.as_int()).copied();

            let has_precipitation = *value
                .element
                .get("has_precipitation")
                .and_then(|v| v.as_byte())
                .context("invalid has_precipitation")?
                != 0;

            let temperature = *value
                .element
                .get("temperature")
                .and_then(|v| v.as_float())
                .context("invalid temperature")?;

            let entity = commands
                .spawn(Biome {
                    name: value.name.clone(),
                    downfall,
                    fog_color,
                    sky_color,
                    water_color,
                    water_fog_color,
                    grass_color,
                    has_precipitation,
                    temperature,
                })
                .id();

            reg.id_to_biome.push(entity);
        }

        Ok(())
    };

    if let Err(e) = helper() {
        error!("failed to load default biomes from registry codec: {e:#}");
    }
}

/// Add new biomes to or update existing biomes in the registry.
fn update_biome_registry(
    mut reg: ResMut<BiomeRegistry>,
    mut codec: ResMut<RegistryCodec>,
    biomes: Query<(Entity, &Biome), Changed<Biome>>,
) {
    for (entity, biome) in &biomes {
        let biome_registry = codec.registry_mut(BiomeRegistry::KEY);

        let mut effects = compound! {
            "fog_color" => biome.fog_color,
            "sky_color" => biome.sky_color,
            "water_color" => biome.water_color,
            "water_fog_color" => biome.water_fog_color,
        };

        if let Some(grass_color) = biome.grass_color {
            effects.insert("grass_color", grass_color);
        }

        let biome_compound = compound! {
            "downfall" => biome.downfall,
            "effects" => effects,
            "has_precipitation" => biome.has_precipitation,
            "temperature" => biome.temperature,
        };

        if let Some(value) = biome_registry.iter_mut().find(|v| v.name == biome.name) {
            value.name = biome.name.clone();
            value.element.merge(biome_compound);
        } else {
            biome_registry.push(RegistryValue {
                name: biome.name.clone(),
                element: biome_compound,
            });
            reg.id_to_biome.push(entity);
        }

        assert_eq!(
            biome_registry.len(),
            reg.id_to_biome.len(),
            "biome registry and biome lookup table differ in length"
        );
    }
}

/// Remove deleted biomes from the registry.
fn remove_biomes_from_registry(
    mut biomes: RemovedComponents<Biome>,
    mut reg: ResMut<BiomeRegistry>,
    mut codec: ResMut<RegistryCodec>,
) {
    for biome in biomes.iter() {
        if let Some(idx) = reg.id_to_biome.iter().position(|entity| *entity == biome) {
            reg.id_to_biome.remove(idx);
            codec.registry_mut(BiomeRegistry::KEY).remove(idx);
        }
    }
}

#[derive(Resource)]
pub struct BiomeRegistry {
    id_to_biome: Vec<Entity>,
}

impl BiomeRegistry {
    pub const KEY: Ident<&str> = ident!("minecraft:worldgen/biome");

    pub fn get_by_id(&self, id: BiomeId) -> Option<Entity> {
        self.id_to_biome.get(id.0 as usize).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = (BiomeId, Entity)> + '_ {
        self.id_to_biome
            .iter()
            .enumerate()
            .map(|(id, biome)| (BiomeId(id as _), *biome))
    }
}

impl Index<BiomeId> for BiomeRegistry {
    type Output = Entity;

    fn index(&self, index: BiomeId) -> &Self::Output {
        self.id_to_biome
            .get(index.0 as usize)
            .unwrap_or_else(|| panic!("invalid {index:?}"))
    }
}

/// An index into the biome registry.
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct BiomeId(pub u16);

#[derive(Component, Clone, Debug)]
pub struct Biome {
    pub name: Ident<String>,
    pub downfall: f32,
    pub fog_color: i32,
    pub sky_color: i32,
    pub water_color: i32,
    pub water_fog_color: i32,
    pub grass_color: Option<i32>,
    pub has_precipitation: bool,
    pub temperature: f32,
    // TODO: more stuff.
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            name: ident!("plains").into(),
            downfall: 0.4,
            fog_color: 12638463,
            sky_color: 7907327,
            water_color: 4159204,
            water_fog_color: 329011,
            grass_color: None,
            has_precipitation: true,
            temperature: 0.8,
        }
    }
}
