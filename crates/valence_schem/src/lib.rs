#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;
use std::str::FromStr;

use flate2::bufread::GzDecoder;
use glam::{DVec3, IVec3};
use thiserror::Error;
use valence_biome::BiomeId;
use valence_block::{BlockEntityKind, BlockState, ParseBlockStateError};
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::ident::Ident;
use valence_core::packet::var_int::{VarInt, VarIntDecodeError};
use valence_instance::{Block as ValenceBlock, Instance};
use valence_nbt::{Compound, List, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Schematic {
    pub metadata: Option<Compound>,
    pub width: u16,
    pub height: u16,
    pub length: u16,
    pub offset: IVec3,
    blocks: Option<Box<[Block]>>,
    biomes: Option<Biomes>,
    pub entities: Option<Box<[Entity]>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub state: BlockState,
    pub block_entity: Option<BlockEntity>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockEntity {
    pub kind: BlockEntityKind,
    pub data: Compound,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Biomes {
    palette: Box<[Ident<String>]>,
    data: Box<[usize]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub pos: DVec3,
    /// The id of the entity type
    pub id: Ident<String>,
    pub data: Option<Compound>,
}

#[derive(Debug, Error)]
pub enum LoadSchematicError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Nbt(#[from] valence_nbt::Error),

    #[error("missing schematic")]
    MissingSchematic,

    #[error("missing version")]
    MissingVersion,

    #[error("unknown version {0} (only version 3 is supported)")]
    UnknownVersion(i32),

    #[error("missing width")]
    MissingWidth,

    #[error("missing height")]
    MissingHeight,

    #[error("missing length")]
    MissingLength,

    #[error("invalid offset")]
    InvalidOffset,

    #[error("missing block palette")]
    MissingBlockPalette,

    #[error("invalid block palette")]
    InvalidBlockPalette,

    #[error(transparent)]
    ParseBlockStateError(#[from] ParseBlockStateError),

    #[error("missing block data")]
    MissingBlockData,

    #[error(transparent)]
    VarIntDecodeError(#[from] VarIntDecodeError),

    #[error("invalid block count")]
    InvalidBlockCount,

    #[error("missing block entity pos")]
    MissingBlockEntityPos,

    #[error("invalid block entity pos {0:?}")]
    InvalidBlockEntityPos(Vec<i32>),

    #[error("missing block entity id")]
    MissingBlockEntityId,

    #[error("invalid block entity id '{0}'")]
    InvalidBlockEntityId(String),

    #[error("unknown block entity '{0}'")]
    UnknownBlockEntity(String),

    #[error("missing biome palette")]
    MissingBiomePalette,

    #[error("invalid biome palette")]
    InvalidBiomePalette,

    #[error("invalid biome ident '{0}'")]
    InvalidBiomeIdent(String),

    #[error("missing biome data")]
    MissingBiomeData,

    #[error("invalid biome count")]
    InvalidBiomeCount,

    #[error("missing entity pos")]
    MissingEntityPos,

    #[error("invalid entity pos {0:?}")]
    InvalidEntityPos(Vec<f64>),

    #[error("missing entity id")]
    MissingEntityId,

    #[error("invalid entity id '{0}'")]
    InvalidEntityId(String),
}

struct VarIntReader<I: ExactSizeIterator<Item = u8>>(I);
impl<I: ExactSizeIterator<Item = u8>> Iterator for VarIntReader<I> {
    type Item = Result<i32, VarIntDecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        struct ReadWrapper<I: ExactSizeIterator<Item = u8>>(I);
        impl<I: ExactSizeIterator<Item = u8>> Read for ReadWrapper<I> {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                for (idx, byte) in buf.iter_mut().enumerate() {
                    let Some(val) = self.0.next() else {
                        return Ok(idx);
                    };
                    *byte = val;
                }
                Ok(buf.len())
            }
        }

        if self.0.len() == 0 {
            None
        } else {
            Some(VarInt::decode_partial(ReadWrapper(&mut self.0)))
        }
    }
}

impl Schematic {
    pub fn load(path: PathBuf) -> Result<Self, LoadSchematicError> {
        let file = File::open(path)?;

        let mut buf = vec![];
        let mut z = GzDecoder::new(BufReader::new(file));
        z.read_to_end(&mut buf)?;

        let root = valence_nbt::from_binary_slice(&mut buf.as_slice())?.0;
        let Some(Value::Compound(root)) = root.get("Schematic") else {
            return Err(LoadSchematicError::MissingSchematic);
        };

        let metadata = root
            .get("Metadata")
            .and_then(|val| val.as_compound())
            .cloned();

        let Some(&Value::Int(version)) = root.get("Version") else {
            return Err(LoadSchematicError::MissingVersion);
        };
        //TODO: Allow version 1 and 2
        if version != 3 {
            return Err(LoadSchematicError::UnknownVersion(version));
        }
        let Some(&Value::Short(width)) = root.get("Width") else {
            return Err(LoadSchematicError::MissingWidth);
        };
        let width = width as u16;
        let Some(&Value::Short(height)) = root.get("Height") else {
            return Err(LoadSchematicError::MissingHeight);
        };
        let height = height as u16;
        let Some(&Value::Short(length)) = root.get("Length") else {
            return Err(LoadSchematicError::MissingLength);
        };
        let length = length as u16;
        let offset = {
            let &[x, y, z] = root.get("Offset").and_then(|val| val.as_int_array()).map(|arr| arr.as_slice()).unwrap_or(&[0; 3]) else {
                return Err(LoadSchematicError::InvalidOffset);
            };
            IVec3::new(x, y, z)
        };
        let blocks: Option<Box<[Block]>> = match root.get("Blocks") {
            Some(Value::Compound(blocks)) => {
                let Some(Value::Compound(palette)) = blocks.get("Palette") else {
                    return Err(LoadSchematicError::MissingBlockPalette);
                };
                let palette: Result<HashMap<_, _>, _> = palette
                    .into_iter()
                    .map(|(state, value)| {
                        let &Value::Int(i) = value else {
                            return Err(LoadSchematicError::InvalidBlockPalette);
                        };
                        let state = BlockState::from_str(
                            state.strip_prefix("minecraft:").unwrap_or(state),
                        )?;
                        Ok((i, state))
                    })
                    .collect();
                let palette = palette?;

                let Some(Value::ByteArray(data)) = blocks.get("Data") else {
                    return Err(LoadSchematicError::MissingBlockData);
                };
                let data: Result<Vec<_>, LoadSchematicError> =
                    VarIntReader(data.iter().map(|byte| *byte as u8))
                        .map(|val| {
                            let state = palette[&val?];
                            Ok(Block {
                                state,
                                block_entity: None,
                            })
                        })
                        .collect();
                let mut data = data?;
                if u16::try_from(data.len()) != Ok(width * height * length) {
                    return Err(LoadSchematicError::InvalidBlockCount);
                }
                if let Some(Value::List(List::Compound(block_entities))) =
                    blocks.get("BlockEntities")
                {
                    for block_entity in block_entities {
                        let Some(Value::IntArray(pos)) = block_entity.get("Pos") else {
                            return Err(LoadSchematicError::MissingBlockEntityPos);
                        };
                        let [x, y, z] = pos[..] else {
                            return Err(LoadSchematicError::InvalidBlockEntityPos(pos.clone()));
                        };

                        let Some(Value::String(id)) = block_entity.get("Id") else {
                            return Err(LoadSchematicError::MissingBlockEntityId);
                        };
                        let Ok(id) = Ident::new(&id[..]) else {
                            return Err(LoadSchematicError::InvalidBlockEntityId(id.clone()));
                        };
                        let Some(kind) = BlockEntityKind::from_ident(id.as_str_ident()) else {
                            return Err(LoadSchematicError::UnknownBlockEntity(id.to_string()));
                        };

                        let nbt = match block_entity.get("Data") {
                            Some(Value::Compound(nbt)) => nbt.clone(),
                            _ => Compound::with_capacity(0),
                        };
                        let block_entity = BlockEntity { kind, data: nbt };
                        data[(x + z * width as i32 + y * width as i32 * length as i32) as usize]
                            .block_entity
                            .replace(block_entity);
                    }
                }
                Some(data.into_boxed_slice())
            }
            _ => None,
        };

        let biomes = match root.get("Biomes") {
            Some(Value::Compound(biomes)) => {
                let Some(Value::Compound(palette)) = biomes.get("Palette") else {
                    return Err(LoadSchematicError::MissingBiomePalette);
                };
                let palette: Result<HashMap<_, _>, _> = palette
                    .iter()
                    .map(|(biome, value)| {
                        let &Value::Int(i) = value else {
                            return Err(LoadSchematicError::InvalidBiomePalette);
                        };
                        let Ok(ident) = Ident::new(biome.clone()) else {
                            return Err(LoadSchematicError::InvalidBiomeIdent(biome.clone()));
                        };
                        Ok((i, ident))
                    })
                    .collect();
                let palette = palette?;
                let Some(Value::ByteArray(data)) = biomes.get("Data") else {
                    return Err(LoadSchematicError::MissingBiomeData);
                };
                let data: Result<Vec<_>, LoadSchematicError> =
                    VarIntReader(data.iter().map(|byte| *byte as u8))
                        .map(|val| Ok(&palette[&val?]))
                        .collect();
                let data = data?;

                let mut palette = vec![];
                let mut map = HashMap::new();
                let data: Vec<_> = data
                    .into_iter()
                    .map(|biome| match map.entry(biome) {
                        Entry::Occupied(entry) => *entry.get(),
                        Entry::Vacant(entry) => {
                            let idx = palette.len();
                            palette.push(biome.to_string_ident());
                            entry.insert(idx);
                            idx
                        }
                    })
                    .collect();

                if u16::try_from(data.len()) != Ok(width * height * length) {
                    return Err(LoadSchematicError::InvalidBiomeCount);
                }

                let biomes = Biomes {
                    palette: palette.into_boxed_slice(),
                    data: data.into_boxed_slice(),
                };
                Some(biomes)
            }
            _ => None,
        };

        let entities: Option<Box<[Entity]>> = match root.get("Entities") {
            Some(Value::List(List::Compound(entities))) => {
                let entities: Result<Vec<_>, _> = entities
                    .iter()
                    .map(|entity| {
                        let Some(Value::List(List::Double(pos))) = entity.get("Pos") else {
                            return Err(LoadSchematicError::MissingEntityPos);
                        };
                        let [x, y, z] = pos[..] else {
                            return Err(LoadSchematicError::InvalidEntityPos(pos.clone()));
                        };
                        let pos = DVec3::new(x, y, z);

                        let Some(Value::String(id)) = entity.get("Id") else {
                            return Err(LoadSchematicError::MissingEntityId);
                        };
                        let Ok(id) = Ident::new(id.clone()) else {
                            return Err(LoadSchematicError::InvalidEntityId(id.clone()));
                        };

                        let data = match entity.get("Data") {
                            Some(Value::Compound(data)) => Some(data.clone()),
                            _ => None,
                        };

                        Ok(Entity {
                            pos,
                            id: id.to_string_ident(),
                            data,
                        })
                    })
                    .collect();
                Some(entities?.into_boxed_slice())
            }
            _ => None,
        };

        Ok(Self {
            metadata,
            width,
            height,
            length,
            offset,
            blocks,
            biomes,
            entities,
        })
    }

    pub fn paste<F>(&self, instance: &mut Instance, origin: BlockPos, map_biome: F)
    where
        F: FnMut(Ident<&str>) -> BiomeId,
    {
        let min_y = instance.min_y();
        if let Some(blocks) = &self.blocks {
            let blocks = blocks.iter().enumerate().map(|(idx, block)| {
                let idx = u16::try_from(idx).unwrap();
                let y = idx / (self.width * self.length);
                let z = (idx % (self.width * self.length)) / self.width;
                let x = (idx % (self.width * self.length)) % self.width;

                ([x, y, z], block)
            });

            for (
                [x, y, z],
                Block {
                    state,
                    block_entity,
                },
            ) in blocks
            {
                let block_pos = BlockPos::new(
                    x as i32 + origin.x + self.offset.x,
                    y as i32 + origin.y + self.offset.y,
                    z as i32 + origin.z + self.offset.z,
                );
                let chunk = instance
                    .chunk_entry(ChunkPos::from_block_pos(block_pos))
                    .or_default();
                let block = match block_entity {
                    Some(BlockEntity { kind, data: nbt })
                        if Some(*kind) == state.block_entity_kind() =>
                    {
                        ValenceBlock::with_nbt(*state, nbt.clone())
                    }
                    _ => ValenceBlock::new(*state),
                };
                chunk.set_block(
                    block_pos.x.rem_euclid(16) as usize,
                    (block_pos.y - min_y) as usize,
                    block_pos.z.rem_euclid(16) as usize,
                    block,
                );
            }
        }

        if let Some(Biomes { palette, data }) = &self.biomes {
            for ([x, y, z], biome) in data
                .iter()
                .map(|biome| palette[*biome].as_str_ident())
                .map(map_biome)
                .enumerate()
                .map(|(idx, biome)| {
                    let idx = u16::try_from(idx).unwrap();
                    let y = idx / (self.width * self.length);
                    let z = (idx % (self.width * self.length)) / self.width;
                    let x = (idx % (self.width * self.length)) % self.width;

                    ([x, y, z], biome)
                })
            {
                let x = x as i32 + origin.x + self.offset.x;
                let y = y as i32 + origin.y + self.offset.y;
                let z = z as i32 + origin.z + self.offset.z;
                let chunk = instance
                    .chunk_entry(ChunkPos::at(x as f64, z as f64))
                    .or_default();

                chunk.set_biome(
                    (x / 4).rem_euclid(4) as usize,
                    (y - min_y / 4) as usize,
                    (z / 4).rem_euclid(4) as usize,
                    biome,
                );
            }
        }

        // TODO: Spawn entities
    }
}
