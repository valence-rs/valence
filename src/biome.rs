//! Biome definitions.

use crate::ident;
use crate::ident::Ident;
use crate::protocol_inner::packets::play::s2c::Biome as BiomeRegistryBiome;

/// Identifies a particular [`Biome`] on the server.
///
/// The default biome ID refers to the first biome added in the server's
/// [configuration](crate::config::Config).
///
/// To obtain biome IDs for other biomes, call
/// [`biomes`](crate::server::SharedServer::biomes).
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BiomeId(pub(crate) u16);

/// Contains the configuration for a biome.
///
/// Biomes are registered once at startup through
/// [`biomes`](crate::config::Config::biomes).
#[derive(Clone, Debug)]
pub struct Biome {
    /// The unique name for this biome. The name can be
    /// seen in the F3 debug menu.
    pub name: Ident,
    pub precipitation: BiomePrecipitation,
    pub sky_color: u32,
    pub water_fog_color: u32,
    pub fog_color: u32,
    pub water_color: u32,
    pub foliage_color: Option<u32>,
    pub grass_color: Option<u32>,
    pub grass_color_modifier: BiomeGrassColorModifier,
    pub music: Option<BiomeMusic>,
    pub ambient_sound: Option<Ident>,
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
}

impl Biome {
    pub(crate) fn to_biome_registry_item(&self, id: i32) -> BiomeRegistryBiome {
        use crate::protocol_inner::packets::play::s2c::{
            BiomeAdditionsSound, BiomeEffects, BiomeMoodSound, BiomeMusic, BiomeParticle,
            BiomeParticleOptions, BiomeProperty,
        };

        BiomeRegistryBiome {
            name: self.name.clone(),
            id,
            element: BiomeProperty {
                precipitation: match self.precipitation {
                    BiomePrecipitation::Rain => "rain",
                    BiomePrecipitation::Snow => "snow",
                    BiomePrecipitation::None => "none",
                }
                .into(),
                depth: 0.125,
                temperature: 0.8,
                scale: 0.05,
                downfall: 0.4,
                category: "none".into(),
                temperature_modifier: None,
                effects: BiomeEffects {
                    sky_color: self.sky_color as i32,
                    water_fog_color: self.water_fog_color as i32,
                    fog_color: self.fog_color as i32,
                    water_color: self.water_color as i32,
                    foliage_color: self.foliage_color.map(|x| x as i32),
                    grass_color: self.grass_color.map(|x| x as i32),
                    grass_color_modifier: match self.grass_color_modifier {
                        BiomeGrassColorModifier::Swamp => Some("swamp".into()),
                        BiomeGrassColorModifier::DarkForest => Some("dark_forest".into()),
                        BiomeGrassColorModifier::None => None,
                    },
                    music: self.music.as_ref().map(|bm| BiomeMusic {
                        replace_current_music: bm.replace_current_music,
                        sound: bm.sound.clone(),
                        max_delay: bm.max_delay,
                        min_delay: bm.min_delay,
                    }),
                    ambient_sound: self.ambient_sound.clone(),
                    additions_sound: self.additions_sound.as_ref().map(|a| BiomeAdditionsSound {
                        sound: a.sound.clone(),
                        tick_chance: a.tick_chance,
                    }),
                    mood_sound: self.mood_sound.as_ref().map(|m| BiomeMoodSound {
                        sound: m.sound.clone(),
                        tick_delay: m.tick_delay,
                        offset: m.offset,
                        block_search_extent: m.block_search_extent,
                    }),
                },
                particle: self.particle.as_ref().map(|p| BiomeParticle {
                    probability: p.probability,
                    options: BiomeParticleOptions {
                        kind: p.kind.clone(),
                    },
                }),
            },
        }
    }
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
            grass_color: None,
            grass_color_modifier: BiomeGrassColorModifier::None,
            music: None,
            ambient_sound: None,
            additions_sound: None,
            mood_sound: None,
            particle: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum BiomePrecipitation {
    #[default]
    Rain,
    Snow,
    None,
}

/// Minecraft handles grass colors for swamps and dark oak forests in a special
/// way.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum BiomeGrassColorModifier {
    Swamp,
    DarkForest,
    #[default]
    None,
}

#[derive(Clone, Debug)]
pub struct BiomeMusic {
    pub replace_current_music: bool,
    pub sound: Ident,
    pub min_delay: i32,
    pub max_delay: i32,
}

#[derive(Clone, Debug)]
pub struct BiomeAdditionsSound {
    pub sound: Ident,
    pub tick_chance: f64,
}

#[derive(Clone, Debug)]
pub struct BiomeMoodSound {
    pub sound: Ident,
    pub tick_delay: i32,
    pub offset: f64,
    pub block_search_extent: i32,
}

#[derive(Clone, Debug)]
pub struct BiomeParticle {
    pub probability: f32,
    pub kind: Ident,
}
