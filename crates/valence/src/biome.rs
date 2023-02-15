//! Biome configuration and identification.

use std::collections::HashSet;

use anyhow::ensure;
use tracing::warn;
use valence_nbt::{compound, Compound};
use valence_protocol::ident;
use valence_protocol::ident::Ident;

/// Identifies a particular [`Biome`] on the server.
///
/// The default biome ID refers to the first biome added in
/// [`ServerPlugin::biomes`].
///
/// To obtain biome IDs for other biomes, see [`ServerPlugin::biomes`].
///
/// [`ServerPlugin::biomes`]: crate::config::ServerPlugin::biomes
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BiomeId(pub(crate) u16);

/// Contains the configuration for a biome.
///
/// Biomes are registered once at startup through
/// [`ServerPlugin::with_biomes`]
///
/// [`ServerPlugin::with_biomes`]: crate::config::ServerPlugin::with_biomes
#[derive(Clone, Debug)]
pub struct Biome {
    /// The unique name for this biome. The name can be
    /// seen in the F3 debug menu.
    pub name: Ident<String>,
    pub precipitation: BiomePrecipitation,
    pub sky_color: u32,
    pub water_fog_color: u32,
    pub fog_color: u32,
    pub water_color: u32,
    pub foliage_color: Option<u32>,
    pub grass_color: Option<u32>,
    pub grass_color_modifier: BiomeGrassColorModifier,
    pub music: Option<BiomeMusic>,
    pub ambient_sound: Option<Ident<String>>,
    pub additions_sound: Option<BiomeAdditionsSound>,
    pub mood_sound: Option<BiomeMoodSound>,
    pub particle: Option<BiomeParticle>,
    // TODO
    // * depth: f32
    // * temperature: f32
    // * scale: f32
    // * downfall: f32
    // * category
    // * temperature_modifier
}

impl Biome {
    pub(crate) fn to_biome_registry_item(&self, id: i32) -> Compound {
        compound! {
            "name" => self.name.clone(),
            "id" => id,
            "element" => compound! {
                "precipitation" => match self.precipitation {
                    BiomePrecipitation::Rain => "rain",
                    BiomePrecipitation::Snow => "snow",
                    BiomePrecipitation::None => "none",
                },
                "depth" => 0.125_f32,
                "temperature" => 0.8_f32,
                "scale" => 0.05_f32,
                "downfall" => 0.4_f32,
                "category" => "none",
                "effects" => {
                    let mut eff = compound! {
                        "sky_color" => self.sky_color as i32,
                        "water_fog_color" => self.water_fog_color as i32,
                        "fog_color" => self.fog_color as i32,
                        "water_color" => self.water_color as i32,
                    };

                    if let Some(color) = self.foliage_color {
                        eff.insert("foliage_color", color as i32);
                    }

                    if let Some(color) = self.grass_color {
                        eff.insert("grass_color", color as i32);
                    }

                    match self.grass_color_modifier {
                        BiomeGrassColorModifier::Swamp => eff.insert("grass_color_modifier", "swamp"),
                        BiomeGrassColorModifier::DarkForest => eff.insert("grass_color_modifier", "dark_forest"),
                        BiomeGrassColorModifier::None => None
                    };

                    if let Some(music) = &self.music {
                        eff.insert("music", compound! {
                            "replace_current_music" => music.replace_current_music,
                            "sound" => music.sound.clone(),
                            "max_delay" => music.max_delay,
                            "min_delay" => music.min_delay,
                        });
                    }

                    if let Some(s) = &self.ambient_sound {
                        eff.insert("ambient_sound", s.clone());
                    }

                    if let Some(a) = &self.additions_sound {
                        eff.insert("additions_sound", compound! {
                            "sound" => a.sound.clone(),
                            "tick_chance" => a.tick_chance,
                        });
                    }

                    if let Some(m) = &self.mood_sound {
                        eff.insert("mood_sound", compound! {
                            "sound" => m.sound.clone(),
                            "tick_delay" => m.tick_delay,
                            "offset" => m.offset,
                            "block_search_extent" => m.block_search_extent,
                        });
                    }

                    if let Some(p) = &self.particle {
                        eff.insert(
                            "particle",
                            compound! {
                                "probability" => p.probability,
                                "options" => compound! {
                                    "type" => p.kind.clone(),
                                }
                            },
                        );
                    }

                    eff
                },
            }
        }
    }
}

pub(crate) fn validate_biomes(biomes: &[Biome]) -> anyhow::Result<()> {
    ensure!(!biomes.is_empty(), "at least one biome must be present");

    ensure!(
        biomes.len() <= u16::MAX as _,
        "more than u16::MAX biomes present"
    );

    let mut names = HashSet::new();

    for biome in biomes {
        ensure!(
            names.insert(biome.name.clone()),
            "biome \"{}\" already exists",
            biome.name
        );
    }

    if !names.contains(&ident!("plains")) {
        warn!(
            "A biome named \"plains\" is missing from the biome registry! Due to a bug in the \
             vanilla client, players may not be able to join the game!"
        );
    }

    Ok(())
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            name: ident!("plains"),
            precipitation: BiomePrecipitation::default(),
            sky_color: 7907327,
            water_fog_color: 329011,
            fog_color: 12638463,
            water_color: 4159204,
            foliage_color: None,
            grass_color: None,
            grass_color_modifier: BiomeGrassColorModifier::default(),
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
    pub sound: Ident<String>,
    pub min_delay: i32,
    pub max_delay: i32,
}

#[derive(Clone, Debug)]
pub struct BiomeAdditionsSound {
    pub sound: Ident<String>,
    pub tick_chance: f64,
}

#[derive(Clone, Debug)]
pub struct BiomeMoodSound {
    pub sound: Ident<String>,
    pub tick_delay: i32,
    pub offset: f64,
    pub block_search_extent: i32,
}

#[derive(Clone, Debug)]
pub struct BiomeParticle {
    pub probability: f32,
    pub kind: Ident<String>,
}
