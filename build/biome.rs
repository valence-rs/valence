use std::collections::{BTreeMap};

use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::{Ident, TokenStream};
use quote::{quote};
use serde::Deserialize;

use crate::ident;

#[derive(Deserialize, Debug)]
pub struct ParsedBiome {
    id: u16,
    name: String,
    weather: ParsedBiomeWeather,
    color: ParsedBiomeColor,
    spawn_settings: ParsedBiomeSpawnSettings,
}

#[derive(Debug)]
pub struct RenamedBiome {
    id: u16,
    name: String,
    rustified_name: Ident,
    weather: ParsedBiomeWeather,
    color: ParsedBiomeColor,
    spawn_settings: ParsedBiomeSpawnSettings,
}

#[derive(Deserialize, Debug)]
pub struct ParsedBiomeWeather {
    precipitation: String,
    temperature: f32,
    downfall: f32,
}

#[derive(Deserialize, Debug)]
pub struct ParsedBiomeColor {
    grass_modifier: String,
    grass: Option<i32>,
    foliage: Option<i32>,
    fog: i32,
    sky: i32,
    water_fog: i32,
    water: i32,
}

#[derive(Deserialize, Debug)]
pub struct ParsedBiomeSpawnSettings {
    probability: f32,
    groups: Vec<ParsedBiomeGroupSpawnSettings>,
}

#[derive(Deserialize, Debug)]
pub struct ParsedBiomeGroupSpawnSettings {
    name: String,
    capacity: i32,
    despawn_range_start: i32,
    despawn_range_immediate: i32,
    is_peaceful: bool,
    is_rare: bool,
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
            spawn_settings: biome.spawn_settings,
        })
        .collect::<Vec<RenamedBiome>>();

    let mut precipitation_types = BTreeMap::<&str, Ident>::new();
    let mut grass_modifier_types = BTreeMap::<&str, Ident>::new();
    let mut biome_group_spawn_types = BTreeMap::<&str, (Ident, Ident)>::new();
    for biome in biomes.iter() {
        precipitation_types
            .entry(biome.weather.precipitation.as_str())
            .or_insert_with(|| ident(biome.weather.precipitation.to_pascal_case()));
        grass_modifier_types
            .entry(biome.color.grass_modifier.as_str())
            .or_insert_with(|| ident(biome.color.grass_modifier.to_pascal_case()));
        for group in biome.spawn_settings.groups.iter() {
            biome_group_spawn_types
                .entry(group.name.as_str())
                .or_insert_with(|| {
                    (
                        ident({
                            let mut identity = group.name.to_snake_case();
                            identity.insert_str(0, "group_");
                            identity
                        }),
                        ident({
                            let mut identity = group.name.to_pascal_case();
                            identity.push_str("SpawnSettings");
                            identity
                        }),
                    )
                });
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
                pub #rust_id,
            }
        })
        .collect::<TokenStream>();

    let grass_modifier_names = grass_modifier_types
        .iter()
        .map(|(_, rust_id)| {
            quote! {
                pub #rust_id,
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

    let biome_spawn_settings_fields = biome_group_spawn_types
        .iter()
        .map(|(_, (field, ident))| {
            quote! {
                pub #field: #ident,
            }
        })
        .collect::<TokenStream>();

    let biome_spawn_settings_structs = biome_group_spawn_types.iter().map(|(_, (_, ident))| ident);

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
            let probability = biome.spawn_settings.probability;

            let fields = biome.spawn_settings.groups.iter().map(|parsed_biome|{
                let (_, (field, declaration)) = biome_group_spawn_types.iter().find(|(name,_)| parsed_biome.name.as_str() == **name).expect("Could not find previously generated spawn type");
                let capacity = &parsed_biome.capacity;
                let despawn_range_start = &parsed_biome.despawn_range_start;
                let despawn_range_immediate = &parsed_biome.despawn_range_immediate;
                let is_peaceful = &parsed_biome.is_peaceful;
                let is_rare = &parsed_biome.is_rare;
                quote! {
                    #field: #declaration{
                        capacity: #capacity,
                        despawn_range_start: #despawn_range_start,
                        despawn_range_immediate: #despawn_range_immediate,
                        is_peaceful: #is_peaceful,
                        is_rare: #is_rare
                    }
                }
            });
            quote! {
                Self::#rustified_name => VanillaBiomeSpawnSettings {
                    probability: #probability,
                    #( #fields ),*
                },
            }
        })
        .collect::<TokenStream>();

    Ok(quote! {
        pub trait BiomeSpawnSettings {
            fn capacity(&self) -> i32;
            fn despawn_range_start(&self) -> i32;
            fn despawn_range_immediate(&self) -> i32;
            fn is_peaceful(&self) -> bool;
            fn is_rare(&self) -> bool;
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

        #[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
        pub struct VanillaBiomeSpawnSettings {
            pub probability: f32,
            #biome_spawn_settings_fields
        }

        #( #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct #biome_spawn_settings_structs {
            pub capacity: i32,
            pub despawn_range_start: i32,
            pub despawn_range_immediate: i32,
            pub is_peaceful: bool,
            pub is_rare: bool,
        }

        impl BiomeSpawnSettings for #biome_spawn_settings_structs{
            fn capacity(&self) -> i32 {
                self.capacity
            }
            fn despawn_range_start(&self) -> i32 {
                self.despawn_range_start
            }
            fn despawn_range_immediate(&self) -> i32 {
                self.despawn_range_immediate
            }
            fn is_peaceful(&self) -> bool {
                self.is_peaceful
            }
            fn is_rare(&self) -> bool {
                self.is_rare
            }
        } )*

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

            /// Gets the biome spawn settings
            pub const fn spawn_settings(self) -> VanillaBiomeSpawnSettings {
                match self{
                    #biomekind_spawn_settings_arms
                }
            }
        }
    })
}
