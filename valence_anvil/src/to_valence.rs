use num::integer::div_ceil;
use thiserror::Error;
use valence::biome::BiomeId;
use valence::chunk::{Chunk, UnloadedChunk};
use valence::protocol::block::{BlockKind, PropName, PropValue};
use valence::protocol::Ident;
use valence_nbt::{Compound, List, Value};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ToValenceError<'a> {
    #[error("missing chunk sections")]
    MissingSections,
    #[error("missing chunk section Y")]
    MissingSectionY,
    #[error("missing block states")]
    MissingBlockStates,
    #[error("missing block palette")]
    MissingBlockPalette,
    #[error("missing block name in palette")]
    MissingBlockName,
    #[error("unknown block name of \"{0}\"")]
    UnknownBlockName(&'a str),
    #[error("unknown property name of \"{0}\"")]
    UnknownPropName(&'a str),
    #[error("property value of block is not a string")]
    BadPropValueType,
    #[error("unknown property value of \"{0}\"")]
    UnknownPropValue(&'a str),
    #[error("missing packed block state data in section")]
    MissingBlockStateData,
    #[error("unexpected number of longs in block state blob")]
    BadBlockStateLongCount,
    #[error("invalid block palette index")]
    InvalidBlockPaletteIndex,
}

/// Reads an Anvil chunk in NBT form and writes its data to a Valence [`Chunk`].
///
/// - `nbt`: The Anvil chunk to read from. This is usually the value returned by
///   [`read_chunk`].
/// - `chunk`: The Valence chunk to write to.
/// - `sect_offset`:
///
/// [`read_chunk`]: crate::AnvilWorld::read_chunk
pub fn to_valence<'a, C, F>(
    nbt: &'a Compound,
    chunk: &mut C,
    sect_offset: i32,
    mut map_biomes: F,
) -> Result<(), ToValenceError<'a>>
where
    C: Chunk,
    F: FnMut(Ident<&str>) -> BiomeId,
{
    let Some(Value::List(List::Compound(sections))) = nbt.get("sections") else {
        return Err(ToValenceError::MissingSections)
    };

    // Maps palette indices to the corresponding block state in the palette.
    let mut converted_block_palette = vec![];

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

        converted_block_palette.clear();

        for block in palette {
            let Some(Value::String(name)) = block.get("Name") else {
                return Err(ToValenceError::MissingBlockName)
            };

            let Some(block_kind) = BlockKind::from_str(ident_path(name)) else {
                return Err(ToValenceError::UnknownBlockName(name.as_str()))
            };

            let mut state = block_kind.to_state();

            if let Some(Value::Compound(properties)) = block.get("Properties") {
                for (key, value) in properties {
                    let Value::String(value) = value else {
                        return Err(ToValenceError::BadPropValueType)
                    };

                    let Some(prop_name) = PropName::from_str(key) else {
                        return Err(ToValenceError::UnknownPropName(key))
                    };

                    let Some(prop_value) = PropValue::from_str(value) else {
                        return Err(ToValenceError::UnknownPropValue(value))
                    };

                    state = state.set(prop_name, prop_value);
                }
            }

            converted_block_palette.push(state);
        }

        if converted_block_palette.len() == 1 {
            chunk.fill_block_states(adjusted_sect_y as usize, converted_block_palette[0]);
        } else if converted_block_palette.len() > 1 {
            let Some(Value::LongArray(data)) = block_states.get("data") else {
                return Err(ToValenceError::MissingBlockStateData)
            };

            let bits_per_idx = bit_width(converted_block_palette.len() - 1).max(4);
            let idxs_per_long = 64 / bits_per_idx;
            let long_count = div_ceil(BLOCKS_PER_SECTION, idxs_per_long);
            let mask = 2_u64.pow(bits_per_idx as u32) - 1;

            if long_count != data.len() {
                return Err(ToValenceError::BadBlockStateLongCount)
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
                        return Err(ToValenceError::InvalidBlockPaletteIndex)
                    };

                    let x = i % 16;
                    let z = i / 16 % 16;
                    let y = i / (16 * 16);

                    chunk.set_block_state(x, adjusted_sect_y as usize * 16 + y, z, block);

                    i += 1;
                }
            }
        }
    }

    Ok(())
}

const BLOCKS_PER_SECTION: usize = 4096;

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
