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
    climate: ParsedBiomeClimate,
    color: ParsedBiomeColor,
    spawn_settings: ParsedBiomeSpawnRates,
}

#[derive(Debug)]
struct RenamedBiome {
    id: u16,
    name: String,
    rustified_name: Ident,
    climate: ParsedBiomeClimate,
    color: ParsedBiomeColor,
    spawn_rates: ParsedBiomeSpawnRates,
}

#[derive(Deserialize, Debug)]
struct ParsedBiomeClimate {
    precipitation: String,
    temperature: f32,
    downfall: f32,
}

#[derive(Deserialize, Debug)]
struct ParsedBiomeColor {
    grass_modifier: String,
    grass: Option<u32>,
    foliage: Option<u32>,
    fog: u32,
    sky: u32,
    water_fog: u32,
    water: u32,
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
    let biomes: Vec<ParsedBiome> =
        serde_json::from_str(include_str!("../../extracted/biomes.json"))?;

    let mut biomes = biomes
        .into_iter()
        .map(|biome| RenamedBiome {
            id: biome.id,
            rustified_name: ident(&biome.name.replace("minecraft:", "").to_pascal_case()),
            name: biome.name,
            climate: biome.climate,
            color: biome.color,
            spawn_rates: biome.spawn_settings,
        })
        .collect::<Vec<RenamedBiome>>();

    //Ensure biomes are sorted, even if the JSON changes later.
    biomes.sort_by(|one, two| one.id.cmp(&two.id));

    let mut precipitation_types = BTreeMap::<&str, Ident>::new();
    let mut grass_modifier_types = BTreeMap::<&str, Ident>::new();
    let mut class_spawn_fields = BTreeMap::<&str, Ident>::new();
    for biome in biomes.iter() {
        precipitation_types
            .entry(biome.climate.precipitation.as_str())
            .or_insert_with(|| ident(biome.climate.precipitation.to_pascal_case()));
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

    let biome_kind_enum_declare = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let id = biome.id as isize;
            quote! {
                #rustified_name = #id,
            }
        })
        .collect::<TokenStream>();

    let biome_kind_enum_names = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            quote! {
                #rustified_name
            }
        })
        .collect::<Vec<TokenStream>>();

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

    let biomekind_name_lookup = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let name = &biome.name;
            quote! {
                #name => Some(Self::#rustified_name),
            }
        })
        .collect::<TokenStream>();

    let biomekind_temperatures_arms = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let temp = &biome.climate.temperature;
            quote! {
                Self::#rustified_name => #temp,
            }
        })
        .collect::<TokenStream>();

    let biomekind_downfall_arms = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let downfall = &biome.climate.downfall;
            quote! {
                Self::#rustified_name => #downfall,
            }
        })
        .collect::<TokenStream>();

    let biomekind_to_biome = biomes
        .iter()
        .map(|biome| {
            let rustified_name = &biome.rustified_name;
            let name = &biome.name;
            let precipitation = ident(&biome.climate.precipitation.to_pascal_case());
            let sky_color = &biome.color.sky;
            let water_fog = &biome.color.water_fog;
            let fog = &biome.color.fog;
            let water_color = &biome.color.water;
            let foliage_color = option_to_quote(&biome.color.foliage);
            let grass_color = option_to_quote(&biome.color.grass);
            let grass_modifier = ident(&biome.color.grass_modifier.to_pascal_case());
            quote! {
                Self::#rustified_name => Ok(Biome{
                    name: Ident::from_str(#name)?,
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
            let rustified_name = &biome.rustified_name;
            let probability = biome.spawn_rates.probability;

            let fields = biome.spawn_rates.groups.iter().map(|(class, rates)| {
                let rates = rates.iter().map(|spawn_rate| {
                    let name = &spawn_rate.name;
                    let min_group_size = &spawn_rate.min_group_size;
                    let max_group_size = &spawn_rate.max_group_size;
                    let weight = &spawn_rate.weight;
                    quote! {
                        SpawnProperty {
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
                Self::#rustified_name => SpawnSettings {
                    probability: #probability,
                    #( #fields ),*
                },
            }
        })
        .collect::<TokenStream>();

    let spawn_classes = class_spawn_fields.values();

    Ok(quote! {
        use valence::biome::{Biome,BiomeGrassColorModifier,BiomePrecipitation};
        use valence::ident::{Ident,IdentError};
        use std::str::FromStr;

        #[derive(Debug, Clone, PartialEq, PartialOrd)]
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
                if ident.namespace() != "minecraft"{
                    return None;
                }
                match ident.path() {
                    #biomekind_name_lookup
                    _ => None
                }
            }

            pub fn biome(self) -> Result<Biome, IdentError<String>> {
                match self{
                    #biomekind_to_biome
                }
            }

            /// Gets the biome spawn rates
            pub const fn spawn_rates(self) -> SpawnSettings {
                match self{
                    #biomekind_spawn_settings_arms
                }
            }

            pub const fn temperature(self) -> f32 {
                match self{
                    #biomekind_temperatures_arms
                }
            }

            pub const fn downfall(self) -> f32 {
                match self{
                    #biomekind_downfall_arms
                }
            }
        }
    })
}
