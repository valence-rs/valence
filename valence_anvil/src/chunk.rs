use std::fmt;

use num_traits::FromPrimitive;
use valence::nbt::{List, Value};
use valence::prelude::*;

use crate::error::{DataFormatError, Error, NbtFormatError};
use crate::palette::{
    parse_identity_list_palette, parse_palette_identities_with_properties, DataFormat,
};
use crate::AnvilWorldConfig;

#[derive(Debug, Copy, Clone)]
pub enum ChunkStatus {
    Empty,
    StructureStarts,
    StructureReferences,
    Biomes,
    Noise,
    Surface,
    Carvers,
    LiquidCarvers,
    Features,
    Light,
    Spawn,
    Heightmaps,
    Full,
}

impl ChunkStatus {
    /// Retrieves the "Status" field from the NBT compound and parses it to
    /// `Self`
    ///
    /// # Arguments
    ///
    /// * `nbt`: The chunk NBT compound
    ///
    /// returns: the status or `Self::Unknown` if no valid status was found.
    pub fn from_nbt(nbt: &Compound) -> Result<Self, Error> {
        match nbt.get("Status") {
            None => Err(Error::NbtFormatError(NbtFormatError::MissingKey {
                tag: None,
                key: "Status".to_string(),
            })),
            Some(Value::String(x)) => match x.as_str() {
                "full" => Ok(Self::Full),
                "empty" => Ok(Self::Empty),
                "structure_starts" => Ok(Self::StructureStarts),
                "structure_references" => Ok(Self::StructureReferences),
                "biomes" => Ok(Self::Biomes),
                "noise" => Ok(Self::Noise),
                "surface" => Ok(Self::Surface),
                "carvers" => Ok(Self::Carvers),
                "liquid_carvers" => Ok(Self::LiquidCarvers),
                "features" => Ok(Self::Features),
                "light" => Ok(Self::Light),
                "spawn" => Ok(Self::Spawn),
                "heightmaps" => Ok(Self::Heightmaps),
                raw => Err(Error::DataFormatError(DataFormatError::InvalidChunkState(
                    raw.to_string(),
                ))),
            },
            Some(_) => Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                tag: None,
                key: "Status".to_string(),
            })),
        }
    }

    pub fn is_fully_generated(&self) -> bool {
        matches!(self, ChunkStatus::Full)
    }

    pub fn raw_status(&self) -> &str {
        match self {
            Self::Full => "full",
            Self::Empty => "empty",
            Self::StructureStarts => "structure_starts",
            Self::StructureReferences => "structure_references",
            Self::Biomes => "biomes",
            Self::Noise => "noise",
            Self::Surface => "surface",
            Self::Carvers => "carvers",
            Self::LiquidCarvers => "liquid_carvers",
            Self::Features => "features",
            Self::Light => "light",
            Self::Spawn => "spawn",
            Self::Heightmaps => "heightmaps",
        }
    }
}

impl fmt::Display for ChunkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.raw_status())
    }
}

pub fn parse_chunk_nbt(
    mut nbt: Compound,
    world_config: &AnvilWorldConfig,
) -> Result<UnloadedChunk, Error> {
    let status: ChunkStatus = ChunkStatus::from_nbt(&nbt)?;
    if !status.is_fully_generated() {
        return Err(Error::DataFormatError(
            DataFormatError::UnexpectedChunkState(status),
        ));
    }

    if let Some(Value::List(List::Compound(nbt_sections))) = nbt.remove("sections") {
        // Parsing sections
        let mut chunk = UnloadedChunk::new(world_config.height);
        for mut nbt_section in nbt_sections.into_iter() {
            let chunk_y_offset: isize = if let Some(Value::Byte(y)) = nbt_section.get("Y") {
                match isize::from_i8(*y) {
                    None => {
                        return Err(Error::DataFormatError(DataFormatError::MissingChunkNBT {
                            tag: Some(nbt_section),
                            key: "Y",
                        }));
                    }
                    Some(height) => height * 16,
                }
            } else {
                return Err(Error::DataFormatError(DataFormatError::MissingChunkNBT {
                    tag: Some(nbt_section),
                    key: "Y",
                }));
            };

            // Block states
            match nbt_section.remove("block_states") {
                Some(Value::Compound(tag)) => {
                    parse_palette_identities_with_properties::<BlockState, _, _, _>(
                        tag,
                        4,
                        16 * 16 * 16,
                        |identity: Ident<String>| {
                            if let Some(block_kind) = BlockKind::from_str(identity.path()) {
                                Ok(BlockState::from_kind(block_kind))
                            } else {
                                Err(Error::DataFormatError(DataFormatError::UnknownType(
                                    identity,
                                )))
                            }
                        },
                        |state: BlockState, property: PropName, value: PropValue| {
                            Ok(state.set(property, value))
                        },
                        |data: DataFormat<BlockState>| match data {
                            DataFormat::All(state) => {
                                if !state.is_air() {
                                    for x in 0..16 {
                                        for y in 0..16isize {
                                            for z in 0..16 {
                                                chunk.set_block_state(
                                                    x,
                                                    (y + chunk_y_offset - world_config.min_y)
                                                        as usize,
                                                    z,
                                                    state,
                                                );
                                            }
                                        }
                                    }
                                }
                                Ok(())
                            }
                            DataFormat::Palette(index, state) => {
                                let y = (index >> 8 & 0b1111) as isize;
                                let z = index >> 4 & 0b1111;
                                let x = index & 0b1111;
                                chunk.set_block_state(
                                    x,
                                    (y + chunk_y_offset - world_config.min_y) as usize,
                                    z,
                                    state,
                                );
                                Ok(())
                            }
                        },
                    )?;
                }
                Some(value) => {
                    nbt_section.insert("block_states", value);
                    return Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                        tag: Some(nbt_section),
                        key: "block_states".to_string(),
                    }));
                }
                None => {
                    return Err(Error::DataFormatError(DataFormatError::MissingChunkNBT {
                        key: "block_states",
                        tag: Some(nbt_section),
                    }));
                }
            }

            match nbt_section.remove("biomes") {
                Some(Value::Compound(tag)) => {
                    parse_identity_list_palette::<BiomeId, _, _>(
                        tag,
                        0,
                        4 * 4 * 4,
                        |biome_identity: Ident<String>| {
                            if let Some(biome) = world_config.biomes.get(&biome_identity) {
                                Ok(*biome)
                            } else {
                                Err(Error::DataFormatError(DataFormatError::UnknownType(
                                    biome_identity,
                                )))
                            }
                        },
                        |data: DataFormat<BiomeId>| {
                            match data {
                                DataFormat::All(biome) => {
                                    for x in 0..4 {
                                        for y in 0..4isize {
                                            for z in 0..4 {
                                                chunk.set_biome(
                                                    x,
                                                    (y + (chunk_y_offset / 4)
                                                        - (world_config.min_y / 4))
                                                        as usize,
                                                    z,
                                                    biome,
                                                );
                                            }
                                        }
                                    }
                                }
                                DataFormat::Palette(index, biome) => {
                                    let y = (index >> 4 & 0b11) as isize;
                                    let z = index >> 2 & 0b11;
                                    let x = index & 0b11;

                                    let final_y =
                                        y + (chunk_y_offset / 4) - (world_config.min_y / 4);
                                    chunk.set_biome(x, final_y as usize, z, biome);
                                }
                            }
                            Ok(())
                        },
                    )?;
                }
                Some(value) => {
                    nbt_section.insert("biomes", value);
                    return Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                        key: "biomes".to_string(),
                        tag: Some(nbt_section),
                    }));
                }
                None => {
                    return Err(Error::DataFormatError(DataFormatError::MissingChunkNBT {
                        key: "biomes",
                        tag: Some(nbt_section),
                    }));
                }
            }
        }
        Ok(chunk)
    } else {
        Err(Error::DataFormatError(DataFormatError::MissingChunkNBT {
            key: "sections",
            tag: Some(nbt),
        }))
    }
}
