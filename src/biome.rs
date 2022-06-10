use crate::{ident, Ident};

/// Identifies a particular [`Biome`].
///
/// Biome IDs are always valid and are cheap to copy and store.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BiomeId(pub(crate) u16);

impl BiomeId {
    pub fn to_index(self) -> usize {
        self.0 as usize
    }
}

/// Contains the configuration for a biome.
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
    pub typ: Ident,
}
