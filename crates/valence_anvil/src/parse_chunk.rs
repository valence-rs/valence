use std::borrow::Cow;
use std::collections::BTreeMap;

use num_integer::div_ceil;
use thiserror::Error;
use valence_biome::BiomeId;
use valence_block::{BlockKind, PropName, PropValue};
use valence_core::ident::Ident;
use valence_instance::chunk::{Chunk, UnloadedChunk};
use valence_nbt::{Compound, List, Value};

#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub(crate) enum ParseChunkError {
    #[error("missing chunk sections")]
    MissingSections,
    #[error("missing chunk section Y")]
    MissingSectionY,
    #[error("section Y is out of bounds")]
    SectionYOutOfBounds,
    #[error("missing block states")]
    MissingBlockStates,
    #[error("missing block palette")]
    MissingBlockPalette,
    #[error("invalid block palette length")]
    BadBlockPaletteLen,
    #[error("missing block name in palette")]
    MissingBlockName,
    #[error("unknown block name of \"{0}\"")]
    UnknownBlockName(String),
    #[error("unknown property name of \"{0}\"")]
    UnknownPropName(String),
    #[error("property value of block is not a string")]
    BadPropValueType,
    #[error("unknown property value of \"{0}\"")]
    UnknownPropValue(String),
    #[error("missing packed block state data in section")]
    MissingBlockStateData,
    #[error("unexpected number of longs in block state data")]
    BadBlockLongCount,
    #[error("invalid block palette index")]
    BadBlockPaletteIndex,
    #[error("missing biomes")]
    MissingBiomes,
    #[error("missing biome palette")]
    MissingBiomePalette,
    #[error("invalid biome palette length")]
    BadBiomePaletteLen,
    #[error("biome name is not a valid resource identifier")]
    BadBiomeName,
    #[error("missing packed biome data in section")]
    MissingBiomeData,
    #[error("unexpected number of longs in biome data")]
    BadBiomeLongCount,
    #[error("invalid biome palette index")]
    BadBiomePaletteIndex,
    #[error("missing block entities")]
    MissingBlockEntities,
    #[error("missing block entity ident")]
    MissingBlockEntityIdent,
    #[error("invalid block entity ident of \"{0}\"")]
    InvalidBlockEntityName(String),
    #[error("invalid block entity position")]
    InvalidBlockEntityPosition,
}

pub(crate) fn parse_chunk(
    mut nbt: Compound,
    biome_map: &BTreeMap<Ident<String>, BiomeId>, // TODO: replace with biome registry arg.
) -> Result<UnloadedChunk, ParseChunkError> {
    let Some(Value::List(List::Compound(sections))) = nbt.remove("sections") else {
        return Err(ParseChunkError::MissingSections)
    };

    if sections.is_empty() {
        return Ok(UnloadedChunk::new());
    }

    let mut chunk =
        UnloadedChunk::with_height((sections.len() * 16).try_into().unwrap_or(u32::MAX));

    let min_sect_y = sections
        .iter()
        .flat_map(|sect| {
            if let Some(Value::Byte(sect_y)) = sect.get("Y") {
                Some(*sect_y)
            } else {
                None
            }
        })
        .min()
        .unwrap() as i32
        * 16;

    let mut converted_block_palette = vec![];
    let mut converted_biome_palette = vec![];

    for mut section in sections {
        let Some(Value::Byte(sect_y)) = section.remove("Y") else {
            return Err(ParseChunkError::MissingSectionY)
        };

        let sect_y = (sect_y as i32 - min_sect_y) as u32;

        if sect_y >= chunk.height() / 16 {
            return Err(ParseChunkError::SectionYOutOfBounds);
        }

        let Some(Value::Compound(mut block_states)) = section.remove("block_states") else {
            return Err(ParseChunkError::MissingBlockStates)
        };

        let Some(Value::List(List::Compound(palette))) = block_states.remove("palette") else {
            return Err(ParseChunkError::MissingBlockPalette)
        };

        if !(1..BLOCKS_PER_SECTION).contains(&palette.len()) {
            return Err(ParseChunkError::BadBlockPaletteLen);
        }

        converted_block_palette.clear();

        for mut block in palette {
            let Some(Value::String(name)) = block.remove("Name") else {
                return Err(ParseChunkError::MissingBlockName)
            };

            let Some(block_kind) = BlockKind::from_str(ident_path(&name)) else {
                return Err(ParseChunkError::UnknownBlockName(name))
            };

            let mut state = block_kind.to_state();

            if let Some(Value::Compound(properties)) = block.remove("Properties") {
                for (key, value) in properties {
                    let Value::String(value) = value else {
                        return Err(ParseChunkError::BadPropValueType)
                    };

                    let Some(prop_name) = PropName::from_str(&key) else {
                        return Err(ParseChunkError::UnknownPropName(key))
                    };

                    let Some(prop_value) = PropValue::from_str(&value) else {
                        return Err(ParseChunkError::UnknownPropValue(value))
                    };

                    state = state.set(prop_name, prop_value);
                }
            }

            converted_block_palette.push(state);
        }

        if converted_block_palette.len() == 1 {
            chunk.fill_block_state_section(sect_y, converted_block_palette[0]);
        } else {
            debug_assert!(converted_block_palette.len() > 1);

            let Some(Value::LongArray(data)) = block_states.remove("data") else {
                return Err(ParseChunkError::MissingBlockStateData)
            };

            let bits_per_idx = bit_width(converted_block_palette.len() - 1).max(4);
            let idxs_per_long = 64 / bits_per_idx;
            let long_count = div_ceil(BLOCKS_PER_SECTION, idxs_per_long);
            let mask = 2_u64.pow(bits_per_idx as u32) - 1;

            if long_count != data.len() {
                return Err(ParseChunkError::BadBlockLongCount);
            };

            let mut i: u32 = 0;
            for long in data {
                let u64 = long as u64;

                for j in 0..idxs_per_long {
                    if i >= BLOCKS_PER_SECTION as u32 {
                        break;
                    }

                    let idx = (u64 >> (bits_per_idx * j)) & mask;

                    let Some(block) = converted_block_palette.get(idx as usize).cloned() else {
                        return Err(ParseChunkError::BadBlockPaletteIndex)
                    };

                    let x = i % 16;
                    let z = i / 16 % 16;
                    let y = i / (16 * 16);

                    chunk.set_block_state(x, y, z, block);

                    i += 1;
                }
            }
        }

        let Some(Value::Compound(biomes)) = section.get("biomes") else {
            return Err(ParseChunkError::MissingBiomes)
        };

        let Some(Value::List(List::String(palette))) = biomes.get("palette") else {
            return Err(ParseChunkError::MissingBiomePalette)
        };

        if !(1..BIOMES_PER_SECTION).contains(&palette.len()) {
            return Err(ParseChunkError::BadBiomePaletteLen);
        }

        converted_biome_palette.clear();

        for biome_name in palette {
            let Ok(ident) = Ident::<Cow<str>>::new(biome_name) else {
                return Err(ParseChunkError::BadBiomeName)
            };

            converted_biome_palette
                .push(biome_map.get(ident.as_str()).copied().unwrap_or_default());
        }

        if converted_biome_palette.len() == 1 {
            chunk.fill_biome_section(sect_y, converted_biome_palette[0]);
        } else {
            debug_assert!(converted_biome_palette.len() > 1);

            let Some(Value::LongArray(data)) = biomes.get("data") else {
                return Err(ParseChunkError::MissingBiomeData)
            };

            let bits_per_idx = bit_width(converted_biome_palette.len() - 1);
            let idxs_per_long = 64 / bits_per_idx;
            let long_count = div_ceil(BIOMES_PER_SECTION, idxs_per_long);
            let mask = 2_u64.pow(bits_per_idx as u32) - 1;

            if long_count != data.len() {
                return Err(ParseChunkError::BadBiomeLongCount);
            };

            let mut i: u32 = 0;
            for &long in data.iter() {
                let u64 = long as u64;

                for j in 0..idxs_per_long {
                    if i >= BIOMES_PER_SECTION as u32 {
                        break;
                    }

                    let idx = (u64 >> (bits_per_idx * j)) & mask;

                    let Some(biome) = converted_biome_palette.get(idx as usize).cloned() else {
                        return Err(ParseChunkError::BadBiomePaletteIndex)
                    };

                    let x = i % 4;
                    let z = i / 4 % 4;
                    let y = i / (4 * 4);

                    chunk.set_biome(x, y, z, biome);

                    i += 1;
                }
            }
        }
    }

    let Some(Value::List(block_entities)) = nbt.remove("block_entities") else {
        return Err(ParseChunkError::MissingBlockEntities);
    };

    if let List::Compound(block_entities) = block_entities {
        for mut comp in block_entities {
            let Some(Value::String(ident)) = comp.remove("id") else {
                return Err(ParseChunkError::MissingBlockEntityIdent);
            };

            if let Err(e) = Ident::new(ident) {
                return Err(ParseChunkError::InvalidBlockEntityName(e.0));
            }

            let Some(Value::Int(x)) = comp.remove("x") else {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            };

            let x = x.rem_euclid(16) as u32;

            let Some(Value::Int(y)) = comp.remove("y") else {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            };

            let Ok(y) = u32::try_from(y.wrapping_sub(min_sect_y * 16)) else {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            };

            if y >= chunk.height() {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            }

            let Some(Value::Int(z)) = comp.remove("z") else {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            };

            let z = z.rem_euclid(16) as u32;

            comp.remove("keepPacked");

            chunk.set_block_entity(x, y, z, Some(comp));
        }
    }

    todo!()
}

const BLOCKS_PER_SECTION: usize = 16 * 16 * 16;
const BIOMES_PER_SECTION: usize = 4 * 4 * 4;

/// Gets the path part of a resource identifier.
fn ident_path(ident: &str) -> &str {
    match ident.rsplit_once(':') {
        Some((_, after)) => after,
        None => ident,
    }
}

/// Returns the minimum number of bits needed to represent the integer `n`.
const fn bit_width(n: usize) -> usize {
    (usize::BITS - n.leading_zeros()) as _
}
