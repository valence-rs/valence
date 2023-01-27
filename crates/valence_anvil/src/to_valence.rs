use num_integer::div_ceil;
use thiserror::Error;
use valence::biome::BiomeId;
use valence::chunk::Chunk;
use valence::protocol::block::{BlockKind, PropName, PropValue};
use valence::protocol::Ident;
use valence_nbt::{Compound, List, Value};

#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub enum ToValenceError {
    #[error("missing chunk sections")]
    MissingSections,
    #[error("missing chunk section Y")]
    MissingSectionY,
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
    #[error("missing biome name")]
    MissingBiomeName,
    #[error("missing packed biome data in section")]
    MissingBiomeData,
    #[error("unexpected number of longs in biome data")]
    BadBiomeLongCount,
    #[error("invalid biome palette index")]
    BadBiomePaletteIndex,
}

/// Reads an Anvil chunk in NBT form and writes its data to a Valence [`Chunk`].
/// An error is returned if the NBT data does not match the expected structure
/// for an Anvil chunk.
///
/// # Arguments
///
/// - `nbt`: The Anvil chunk to read from. This is usually the value returned by
///   [`read_chunk`].
/// - `chunk`: The Valence chunk to write to.
/// - `sect_offset`: A constant to add to all sector Y positions in `nbt`. After
///   applying the offset, only the sectors in the range
///   `0..chunk.sector_count()` are written.
/// - `map_biome`: A function to map biome resource identifiers in the NBT data
///   to Valence [`BiomeId`]s.
///
/// [`read_chunk`]: crate::AnvilWorld::read_chunk
pub fn to_valence<C, F>(
    nbt: &Compound,
    chunk: &mut C,
    sect_offset: i32,
    mut map_biome: F,
) -> Result<(), ToValenceError>
where
    C: Chunk,
    F: FnMut(Ident<&str>) -> BiomeId,
{
    let Some(Value::List(List::Compound(sections))) = nbt.get("sections") else {
        return Err(ToValenceError::MissingSections)
    };

    let mut converted_block_palette = vec![];
    let mut converted_biome_palette = vec![];

    for section in sections {
        let Some(Value::Byte(sect_y)) = section.get("Y") else {
            return Err(ToValenceError::MissingSectionY)
        };

        let adjusted_sect_y = *sect_y as i32 + sect_offset;

        if adjusted_sect_y < 0 || adjusted_sect_y as usize >= chunk.section_count() {
            // Section is out of bounds. Skip it.
            continue;
        }

        let Some(Value::Compound(block_states)) = section.get("block_states") else {
            return Err(ToValenceError::MissingBlockStates)
        };

        let Some(Value::List(List::Compound(palette))) = block_states.get("palette") else {
            return Err(ToValenceError::MissingBlockPalette)
        };

        if !(1..BLOCKS_PER_SECTION).contains(&palette.len()) {
            return Err(ToValenceError::BadBlockPaletteLen);
        }

        converted_block_palette.clear();

        for block in palette {
            let Some(Value::String(name)) = block.get("Name") else {
                return Err(ToValenceError::MissingBlockName)
            };

            let Some(block_kind) = BlockKind::from_str(ident_path(name)) else {
                return Err(ToValenceError::UnknownBlockName(name.into()))
            };

            let mut state = block_kind.to_state();

            if let Some(Value::Compound(properties)) = block.get("Properties") {
                for (key, value) in properties {
                    let Value::String(value) = value else {
                        return Err(ToValenceError::BadPropValueType)
                    };

                    let Some(prop_name) = PropName::from_str(key) else {
                        return Err(ToValenceError::UnknownPropName(key.into()))
                    };

                    let Some(prop_value) = PropValue::from_str(value) else {
                        return Err(ToValenceError::UnknownPropValue(value.into()))
                    };

                    state = state.set(prop_name, prop_value);
                }
            }

            converted_block_palette.push(state);
        }

        if converted_block_palette.len() == 1 {
            chunk.fill_block_states(adjusted_sect_y as usize, converted_block_palette[0]);
        } else {
            debug_assert!(converted_block_palette.len() > 1);

            let Some(Value::LongArray(data)) = block_states.get("data") else {
                return Err(ToValenceError::MissingBlockStateData)
            };

            let bits_per_idx = bit_width(converted_block_palette.len() - 1).max(4);
            let idxs_per_long = 64 / bits_per_idx;
            let long_count = div_ceil(BLOCKS_PER_SECTION, idxs_per_long);
            let mask = 2_u64.pow(bits_per_idx as u32) - 1;

            if long_count != data.len() {
                return Err(ToValenceError::BadBlockLongCount);
            };

            let mut i = 0;
            for &long in data.iter() {
                let u64 = long as u64;

                for j in 0..idxs_per_long {
                    if i >= BLOCKS_PER_SECTION {
                        break;
                    }

                    let idx = (u64 >> (bits_per_idx * j)) & mask;

                    let Some(block) = converted_block_palette.get(idx as usize).cloned() else {
                        return Err(ToValenceError::BadBlockPaletteIndex)
                    };

                    let x = i % 16;
                    let z = i / 16 % 16;
                    let y = i / (16 * 16);

                    chunk.set_block_state(x, adjusted_sect_y as usize * 16 + y, z, block);

                    i += 1;
                }
            }
        }

        let Some(Value::Compound(biomes)) = section.get("biomes") else {
            return Err(ToValenceError::MissingBiomes)
        };

        let Some(Value::List(List::String(palette))) = biomes.get("palette") else {
            return Err(ToValenceError::MissingBiomePalette)
        };

        if !(1..BIOMES_PER_SECTION).contains(&palette.len()) {
            return Err(ToValenceError::BadBiomePaletteLen);
        }

        converted_biome_palette.clear();

        for biome_name in palette {
            let Ok(ident) = Ident::new(biome_name.as_str()) else {
                return Err(ToValenceError::BadBiomeName)
            };

            converted_biome_palette.push(map_biome(ident));
        }

        if converted_biome_palette.len() == 1 {
            chunk.fill_biomes(adjusted_sect_y as usize, converted_biome_palette[0]);
        } else {
            debug_assert!(converted_biome_palette.len() > 1);

            let Some(Value::LongArray(data)) = biomes.get("data") else {
                return Err(ToValenceError::MissingBiomeData)
            };

            let bits_per_idx = bit_width(converted_biome_palette.len() - 1);
            let idxs_per_long = 64 / bits_per_idx;
            let long_count = div_ceil(BIOMES_PER_SECTION, idxs_per_long);
            let mask = 2_u64.pow(bits_per_idx as u32) - 1;

            if long_count != data.len() {
                return Err(ToValenceError::BadBiomeLongCount);
            };

            let mut i = 0;
            for &long in data.iter() {
                let u64 = long as u64;

                for j in 0..idxs_per_long {
                    if i >= BIOMES_PER_SECTION {
                        break;
                    }

                    let idx = (u64 >> (bits_per_idx * j)) & mask;

                    let Some(biome) = converted_biome_palette.get(idx as usize).cloned() else {
                        return Err(ToValenceError::BadBiomePaletteIndex)
                    };

                    let x = i % 4;
                    let z = i / 4 % 4;
                    let y = i / (4 * 4);

                    chunk.set_biome(x, adjusted_sect_y as usize * 4 + y, z, biome);

                    i += 1;
                }
            }
        }
    }

    Ok(())
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
