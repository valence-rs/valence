#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

use std::ops::{Deref, DerefMut};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::error;
use valence_core::ident;
use valence_core::ident::Ident;
use valence_nbt::serde::CompoundSerializer;
use valence_registry::codec::{RegistryCodec, RegistryValue};
use valence_registry::{Registry, RegistryIdx, RegistrySet};

pub struct BiomePlugin;

impl Plugin for BiomePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BiomeRegistry>()
            .add_startup_system(load_default_biomes.in_base_set(CoreSet::PreUpdate))
            .add_system(
                update_biome_registry
                    .in_base_set(CoreSet::PostUpdate)
                    .before(RegistrySet),
            );
    }
}

fn load_default_biomes(mut reg: ResMut<BiomeRegistry>, codec: Res<RegistryCodec>) {
    let mut helper = move || -> anyhow::Result<()> {
        for value in codec.registry(BiomeRegistry::KEY) {
            let biome = Biome::deserialize(value.element.clone())?;

            reg.insert(value.name.clone(), biome);
        }

        // Move "plains" to the front so that `BiomeId::default()` is the ID of plains.
        reg.swap_to_front(ident!("plains"));

        Ok(())
    };

    if let Err(e) = helper() {
        error!("failed to load default biomes from registry codec: {e:#}");
    }
}

fn update_biome_registry(reg: Res<BiomeRegistry>, mut codec: ResMut<RegistryCodec>) {
    if reg.is_changed() {
        let biomes = codec.registry_mut(BiomeRegistry::KEY);

        biomes.clear();

        biomes.extend(reg.iter().map(|(_, name, biome)| {
            RegistryValue {
                name: name.into(),
                element: biome
                    .serialize(CompoundSerializer)
                    .expect("failed to serialize biome"),
            }
        }));
    }
}

#[derive(Resource, Default, Debug)]
pub struct BiomeRegistry {
    reg: Registry<BiomeId, Biome>,
}

impl BiomeRegistry {
    pub const KEY: Ident<&str> = ident!("worldgen/biome");
}

impl Deref for BiomeRegistry {
    type Target = Registry<BiomeId, Biome>;

    fn deref(&self) -> &Self::Target {
        &self.reg
    }
}

impl DerefMut for BiomeRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reg
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct BiomeId(u32);

impl RegistryIdx for BiomeId {
    const MAX: usize = u32::MAX as _;

    #[inline]
    fn to_index(self) -> usize {
        self.0 as _
    }

    #[inline]
    fn from_index(idx: usize) -> Self {
        Self(idx as _)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Biome {
    pub downfall: f32,
    pub effects: BiomeEffects,
    pub has_precipitation: bool,
    pub temperature: f32,
    // TODO: more stuff.
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BiomeEffects {
    pub fog_color: u32,
    pub sky_color: u32,
    pub water_color: u32,
    pub water_fog_color: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grass_color: Option<u32>,
    // TODO: more stuff.
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            downfall: 0.4,
            effects: BiomeEffects::default(),
            has_precipitation: true,
            temperature: 0.8,
        }
    }
}

impl Default for BiomeEffects {
    fn default() -> Self {
        Self {
            fog_color: 12638463,
            sky_color: 7907327,
            water_color: 4159204,
            water_fog_color: 329011,
            grass_color: None,
        }
    }
}
