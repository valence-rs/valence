use std::collections::{BTreeMap, HashMap};

use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use serde::Deserialize;

use crate::ident;

#[derive(Deserialize, Debug)]
struct ParsedBiome {
    id: u16,
    name: String,
    weather: ParsedBiomeWeather,
    color: ParsedBiomeColor,
    spawn_settings: ParsedBiomeSpawnRates,
}

#[derive(Debug)]
struct RenamedBiome {
    id: u16,
    name: String,
    rustified_name: Ident,
    weather: ParsedBiomeWeather,
    color: ParsedBiomeColor,
    spawn_rates: ParsedBiomeSpawnRates,
}

#[derive(Deserialize, Debug)]
struct ParsedBiomeWeather {
    precipitation: String,
    temperature: f32,
    downfall: f32,
}

#[derive(Deserialize, Debug)]
struct ParsedBiomeColor {
    grass_modifier: String,
    grass: Option<i32>,
    foliage: Option<i32>,
    fog: i32,
    sky: i32,
    water_fog: i32,
    water: i32,
}

#[derive(Deserialize, Debug)]
struct ParsedBiomeSpawnRates {
    probability: f32,
    groups: HashMap<String, Vec<ParsedSpawnRate>>,
}

#[derive(Deserialize, Debug)]
struct ParsedSpawnRate {
    name: String,
    min_group_size: u32,
    max_group_size: u32,
    weight: i32,
}

pub fn build() -> anyhow::Result<TokenStream> {
    let biomes: Vec<ParsedBiome> = serde_json::from_str(include_str!("../extracted/biomes.json"))?;

    let biomes = biomes
        .into_iter()
        .map(|biome| RenamedBiome {
            id: biome.id,
            rustified_name: ident(&biome.name.replace("minecraft:", "").to_pascal_case()),
            name: biome.name,
            weather: biome.weather,
            color: biome.color,
            spawn_rates: biome.spawn_settings,
        })
        .collect::<Vec<RenamedBiome>>();

    let mut precipitation_types = BTreeMap::<&str, Ident>::new();
    let mut grass_modifier_types = BTreeMap::<&str, Ident>::new();
    let mut class_spawn_fields = BTreeMap::<&str, Ident>::new();
    for biome in biomes.iter() {
        precipitation_types
            .entry(biome.weather.precipitation.as_str())
            .or_insert_with(|| ident(biome.weather.precipitation.to_pascal_case()));
        grass_modifier_types
            .entry(biome.color.grass_modifier.as_str())
            .or_insert_with(|| ident(biome.color.grass_modifier.to_pascal_case()));
        for class in biome.spawn_rates.groups.keys() {
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

    let biome_kind_definitions = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let id = biome.id as isize;
            quote! {
                #rustified_name = #id,
            }
        })
        .collect::<TokenStream>();

    let biomekind_id_to_variant_lookup = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let id = &biome.id;
            quote! {
                #id => Some(Self::#rustified_name),
            }
        })
        .collect::<TokenStream>();

    let precipitation_names = precipitation_types
        .iter()
        .map(|(_, rust_id)| {
            quote! {
                #rust_id,
            }
        })
        .collect::<TokenStream>();

    let grass_modifier_names = grass_modifier_types
        .iter()
        .map(|(_, rust_id)| {
            quote! {
                #rust_id,
            }
        })
        .collect::<TokenStream>();

    let biomekind_names = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let name = &biome.name;
            quote! {
                Self::#rustified_name => #name,
            }
        })
        .collect::<TokenStream>();

    let biomekind_weather = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let precipitation = precipitation_types
                .get(biome.weather.precipitation.as_str())
                .expect("Could not find previously generated precipitation");
            let downfall = &biome.weather.downfall;
            let temperature = &biome.weather.temperature;
            quote! {
                Self::#rustified_name => BiomeWeather {
                    precipitation: Precipitation::#precipitation,
                    downfall: #downfall,
                    temperature: #temperature,
                },
            }
        })
        .collect::<TokenStream>();

    let biomekind_color = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let grass_modifier = grass_modifier_types
                .get(biome.color.grass_modifier.as_str())
                .expect("Could not find previously generated grass modifier");
            let grass = option_to_quote(&biome.color.grass);
            let foliage = option_to_quote(&biome.color.foliage);
            let fog = &biome.color.fog;
            let sky = &biome.color.sky;
            let water_fog = &biome.color.water_fog;
            let water = &biome.color.water;
            quote! {
                Self::#rustified_name => BiomeColor {
                    grass_modifier: GrassModifier::#grass_modifier,
                    grass: #grass,
                    foliage: #foliage,
                    fog: #fog,
                    sky: #sky,
                    water_fog: #water_fog,
                    water: #water,
                },
            }
        })
        .collect::<TokenStream>();

    let biomekind_spawn_settings_arms = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let probability = biome.spawn_rates.probability;

            let fields = biome.spawn_rates.groups.iter().map(|(class, rates)| {
                let rates = rates.iter().map(|spawn_rate| {
                    let name = &spawn_rate.name;
                    let min_group_size = &spawn_rate.min_group_size;
                    let max_group_size = &spawn_rate.max_group_size;
                    let weight = &spawn_rate.weight;
                    quote! {
                        SpawnEntry {
                            name: #name,
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
                Self::#rustified_name => VanillaBiomeSpawnRates {
                    probability: #probability,
                    #( #fields ),*
                },
            }
        })
        .collect::<TokenStream>();

    let spawn_classes = class_spawn_fields.values();

    Ok(quote! {
        #[derive(Debug, Clone, PartialEq, PartialOrd)]
        pub struct SpawnEntry {
            pub name: &'static str,
            pub min_group_size: u32,
            pub max_group_size: u32,
            pub weight: i32
        }

        #[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
        pub struct BiomeWeather {
            pub precipitation: Precipitation,
            pub temperature: f32,
            pub downfall: f32,
        }

        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Precipitation {
            #precipitation_names
        }

        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct BiomeColor {
            pub grass_modifier: GrassModifier,
            pub grass: Option<i32>,
            pub foliage: Option<i32>,
            pub fog: i32,
            pub sky: i32,
            pub water_fog: i32,
            pub water: i32,
        }

        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum GrassModifier {
            #grass_modifier_names
        }

        #[derive(Debug, Clone, PartialEq, PartialOrd)]
        pub struct VanillaBiomeSpawnRates {
            pub probability: f32,
            #( pub #spawn_classes: &'static [SpawnEntry] ),*
        }

        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum BiomeKind {
            #biome_kind_definitions
        }

        impl BiomeKind {
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

            /// Returns the biome name with both the namespace and path (eg: minecraft:plains)
            pub const fn name(self) -> &'static str {
                match self{
                    #biomekind_names
                }
            }

            /// Gets the biome weather settings
            pub const fn weather(self) -> BiomeWeather {
                match self{
                    #biomekind_weather
                }
            }

            /// Gets the biome color settings
            pub const fn color(self) -> BiomeColor {
                match self{
                    #biomekind_color
                }
            }

            /// Gets the biome spawn rates
            pub const fn spawn_rates(self) -> VanillaBiomeSpawnRates {
                match self{
                    #biomekind_spawn_settings_arms
                }
            }
        }
    })
}
