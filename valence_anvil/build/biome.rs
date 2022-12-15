use std::collections::{BTreeMap, HashMap};
use std::fmt;

use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::{Ident as TokenIdent, TokenStream};
use quote::quote;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

use crate::ident;

#[derive(Deserialize, Debug)]
struct ParsedElement {
    id: u16,
    #[serde(deserialize_with = "parse_ident")]
    name: ParsedName,
    element: ParsedBiome,
}

#[derive(Deserialize, Debug)]
struct ParsedBiome {
    #[serde(deserialize_with = "parse_ident")]
    precipitation: ParsedName,
    temperature: f32,
    downfall: f32,
    effects: ParsedBiomeEffects,
    spawn_settings: ParsedBiomeSpawnRates,
}

#[derive(Deserialize, Debug)]
struct ParsedBiomeEffects {
    sky_color: u32,
    water_fog_color: u32,
    fog_color: u32,
    water_color: u32,
    #[serde(deserialize_with = "parse_ident")]
    grass_color_modifier: ParsedName,
    grass_color: Option<u32>,
    foliage_color: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct ParsedBiomeSpawnRates {
    probability: f32,
    groups: HashMap<String, Vec<ParsedSpawnRate>>,
}

#[derive(Deserialize, Debug)]
struct ParsedSpawnRate {
    #[serde(deserialize_with = "parse_ident")]
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

fn parse_ident<'de, D>(deserializer: D) -> Result<ParsedName, D::Error>
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
                    music: None,
                    ambient_sound: None,
                    additions_sound: None,
                    mood_sound: None,
                    particle: None,
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
        use valence::biome::{Biome, BiomeGrassColorModifier, BiomePrecipitation};
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
