use std::collections::{BTreeMap, HashMap};
use std::fmt;

use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::{Ident as TokenIdent, TokenStream};
use quote::{quote, ToTokens};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

use crate::ident;

#[derive(Deserialize, Debug)]
struct ParsedElement {
    id: u16,
    name: ParsedName,
    element: ParsedBiome,
}

#[derive(Deserialize, Debug)]
struct ParsedBiome {
    precipitation: ParsedName,
    temperature: f32,
    downfall: f32,
    effects: ParsedBiomeEffects,
    particle: Option<ParsedParticle>,
    spawn_settings: ParsedBiomeSpawnRates,
}

#[derive(Deserialize, Debug)]
struct ParsedBiomeEffects {
    sky_color: u32,
    water_fog_color: u32,
    fog_color: u32,
    water_color: u32,
    grass_color_modifier: ParsedName,
    grass_color: Option<u32>,
    foliage_color: Option<u32>,
    music: Option<ParsedMusic>,
    ambient_sound: Option<ParsedName>,
    additions_sound: Option<ParsedAdditionsMusic>,
    mood_sound: Option<ParsedMoodSound>,
}

#[derive(Deserialize, Debug)]
struct ParsedMusic {
    replace_current_music: bool,
    sound: ParsedName,
    max_delay: i32,
    min_delay: i32,
}

#[derive(Deserialize, Debug)]
struct ParsedAdditionsMusic {
    sound: ParsedName,
    tick_chance: f64,
}

#[derive(Deserialize, Debug)]
struct ParsedMoodSound {
    sound: ParsedName,
    tick_delay: i32,
    offset: f64,
    block_search_extent: i32,
}

#[derive(Deserialize, Debug)]
struct ParsedParticle {
    kind: ParsedName,
    probability: f32,
}

impl ToTokens for ParsedMusic {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let replace_current_music = &self.replace_current_music;
        let sound = &self.sound.raw;
        let min_delay = &self.min_delay;
        let max_delay = &self.max_delay;
        quote! (
            BiomeMusic {
                replace_current_music: #replace_current_music,
                sound: Ident::from_str(#sound)?,
                min_delay: #min_delay,
                max_delay: #max_delay
            }
        )
        .to_tokens(tokens)
    }
}

impl ToTokens for ParsedAdditionsMusic {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let sound = &self.sound.raw;
        let tick_chance = &self.tick_chance;
        quote! (
            BiomeAdditionsSound {
                sound: Ident::from_str(#sound)?,
                tick_chance: #tick_chance,
            }
        )
        .to_tokens(tokens)
    }
}

impl ToTokens for ParsedMoodSound {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let sound = &self.sound.raw;
        let block_search_extent = &self.block_search_extent;
        let offset = &self.offset;
        let tick_delay = &self.tick_delay;
        quote! (
            BiomeMoodSound {
                sound: Ident::from_str(#sound)?,
                block_search_extent: #block_search_extent,
                offset: #offset,
                tick_delay: #tick_delay
            }
        )
        .to_tokens(tokens)
    }
}

impl ToTokens for ParsedParticle {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let kind = &self.kind.raw;
        let probability = &self.probability;
        quote! (
            BiomeParticle {
                kind: Ident::from_str(#kind)?,
                probability: #probability
            }
        )
        .to_tokens(tokens)
    }
}

#[derive(Deserialize, Debug)]
struct ParsedBiomeSpawnRates {
    probability: f32,
    groups: HashMap<String, Vec<ParsedSpawnRate>>,
}

#[derive(Deserialize, Debug)]
struct ParsedSpawnRate {
    name: ParsedName,
    min_group_size: u32,
    max_group_size: u32,
    weight: i32,
}

#[derive(Debug)]
struct ParsedName {
    token: TokenIdent,
    raw: String,
}

impl<'de> Deserialize<'de> for ParsedName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IdentVisitor;
        impl<'de> Visitor<'de> for IdentVisitor {
            type Value = ParsedName;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string containing a minecraft ID path")
            }
            fn visit_str<E>(self, id: &str) -> Result<Self::Value, E> {
                Ok(ParsedName {
                    token: ident(id.to_pascal_case()),
                    raw: id.to_string(),
                })
            }
        }
        deserializer.deserialize_str(IdentVisitor)
    }
}

pub fn build() -> anyhow::Result<TokenStream> {
    let mut biomes: Vec<ParsedElement> =
        serde_json::from_str(include_str!("../../extracted/biomes.json"))?;

    //Ensure biomes are sorted, even if the JSON changes later.
    biomes.sort_by(|one, two| one.id.cmp(&two.id));

    let mut class_spawn_fields = BTreeMap::<&str, TokenIdent>::new();
    for biome in biomes.iter().map(|b| &b.element) {
        for class in biome.spawn_settings.groups.keys() {
            class_spawn_fields
                .entry(class)
                .or_insert_with(|| ident(class.to_snake_case()));
        }
    }

    fn option_to_quote<T: quote::ToTokens>(input: &Option<T>) -> TokenStream {
        match input {
            Some(value) => quote!(Some(#value)),
            None => quote!(None),
        }
    }

    let biome_kind_enum_declare = biomes
        .iter()
        .map(|biome| {
            let name = &biome.name.token;
            let id = biome.id as isize;
            quote! {
                #name = #id,
            }
        })
        .collect::<TokenStream>();

    let biome_kind_enum_names = biomes
        .iter()
        .map(|biome| {
            let name = &biome.name.token;
            quote! {
                #name
            }
        })
        .collect::<Vec<TokenStream>>();

    let biomekind_id_to_variant_lookup = biomes
        .iter()
        .map(|biome| {
            let name = &biome.name.token;
            let id = &biome.id;
            quote! {
                #id => Some(Self::#name),
            }
        })
        .collect::<TokenStream>();

    let biomekind_name_lookup = biomes
        .iter()
        .map(|biome| {
            let name = &biome.name.token;
            let raw = &biome.name.raw;
            quote! {
                #raw => Some(Self::#name),
            }
        })
        .collect::<TokenStream>();

    let biomekind_temperatures_arms = biomes
        .iter()
        .map(|biome| {
            let name = &biome.name.token;
            let temp = &biome.element.temperature;
            quote! {
                Self::#name => #temp,
            }
        })
        .collect::<TokenStream>();

    let biomekind_downfall_arms = biomes
        .iter()
        .map(|biome| {
            let name = &biome.name.token;
            let downfall = &biome.element.downfall;
            quote! {
                Self::#name => #downfall,
            }
        })
        .collect::<TokenStream>();

    let biomekind_to_biome = biomes
        .iter()
        .map(|biome| {
            let name = &biome.name.token;
            let raw_name = &biome.name.raw;
            let precipitation = &biome.element.precipitation.token;
            let sky_color = &biome.element.effects.sky_color;
            let water_fog = &biome.element.effects.water_fog_color;
            let fog = &biome.element.effects.fog_color;
            let water_color = &biome.element.effects.water_color;
            let foliage_color = option_to_quote(&biome.element.effects.foliage_color);
            let grass_color = option_to_quote(&biome.element.effects.grass_color);
            let grass_modifier = &biome.element.effects.grass_color_modifier.token;
            let music = option_to_quote(&biome.element.effects.music);
            let ambient_sound = option_to_quote({
                &biome.element.effects.ambient_sound.as_ref().map(|n| {
                    let raw = &n.raw;
                    quote!(Ident::from_str(#raw)?)
                })
            });
            let additions_sound = option_to_quote(&biome.element.effects.additions_sound);
            let mood_sound = option_to_quote(&biome.element.effects.mood_sound);
            let particle = option_to_quote(&biome.element.particle);
            quote! {
                Self::#name => Ok(Biome{
                    name: Ident::from_str(#raw_name)?,
                    precipitation: BiomePrecipitation::#precipitation,
                    sky_color: #sky_color,
                    water_fog_color: #water_fog,
                    fog_color: #fog,
                    water_color: #water_color,
                    foliage_color: #foliage_color,
                    grass_color: #grass_color,
                    grass_color_modifier: BiomeGrassColorModifier::#grass_modifier,
                    music: #music,
                    ambient_sound: #ambient_sound,
                    additions_sound: #additions_sound,
                    mood_sound: #mood_sound,
                    particle: #particle,
                }),
            }
        })
        .collect::<TokenStream>();

    let biomekind_spawn_settings_arms = biomes
        .iter()
        .map(|biome| {
            let name = &biome.name.token;
            let probability = biome.element.spawn_settings.probability;

            let fields = biome
                .element
                .spawn_settings
                .groups
                .iter()
                .map(|(class, rates)| {
                    let rates = rates.iter().map(|spawn_rate| {
                        let name_raw = &spawn_rate.name.raw;
                        let min_group_size = &spawn_rate.min_group_size;
                        let max_group_size = &spawn_rate.max_group_size;
                        let weight = &spawn_rate.weight;
                        quote! {
                            SpawnProperty {
                                name: #name_raw,
                                min_group_size: #min_group_size,
                                max_group_size: #max_group_size,
                                weight: #weight
                            }
                        }
                    });
                    let class = ident(class);
                    quote! {
                        #class: &[#( #rates ),*]
                    }
                });
            quote! {
                Self::#name => SpawnSettings {
                    probability: #probability,
                    #( #fields ),*
                },
            }
        })
        .collect::<TokenStream>();

    let spawn_classes = class_spawn_fields.values();

    Ok(quote! {
        use valence::biome::{Biome, BiomeMusic, BiomeAdditionsSound, BiomeMoodSound, BiomeParticle, BiomeGrassColorModifier, BiomePrecipitation};
        use valence::protocol::ident::{Ident, IdentError};
        use std::str::FromStr;

        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
        pub struct SpawnProperty {
            pub name: &'static str,
            pub min_group_size: u32,
            pub max_group_size: u32,
            pub weight: i32
        }

        #[derive(Debug, Clone, PartialEq, PartialOrd)]
        pub struct SpawnSettings {
            pub probability: f32,
            #( pub #spawn_classes: &'static [SpawnProperty] ),*
        }

        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum BiomeKind {
            #biome_kind_enum_declare
        }

        impl BiomeKind {
            /// All imported vanilla biomes (All variants of `BiomeKind`)
            pub const ALL: &'static [Self] = &[#(Self::#biome_kind_enum_names),*];

            /// Constructs an `BiomeKind` from a raw biome ID.
            ///
            /// If the given ID is invalid, `None` is returned.
            pub const fn from_raw(id: u16) -> Option<Self> {
                match id {
                    #biomekind_id_to_variant_lookup
                    _ => None
                }
            }

            /// Returns the raw biome ID.
            pub const fn to_raw(self) -> u16 {
                self as u16
            }

            pub fn from_ident<S: AsRef<str>>(ident: &Ident<S>) -> Option<Self> {
                if ident.namespace() != "minecraft" {
                    return None;
                }
                match ident.path() {
                    #biomekind_name_lookup
                    _ => None
                }
            }

            pub fn biome(self) -> Result<Biome, IdentError<String>> {
                match self {
                    #biomekind_to_biome
                }
            }

            /// Gets the biome spawn rates
            pub const fn spawn_rates(self) -> SpawnSettings {
                match self {
                    #biomekind_spawn_settings_arms
                }
            }

            pub const fn temperature(self) -> f32 {
                match self {
                    #biomekind_temperatures_arms
                }
            }

            pub const fn downfall(self) -> f32 {
                match self {
                    #biomekind_downfall_arms
                }
            }
        }
    })
}
