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
use std::io::{self, BufReader, Read, Write};
use std::path::Path;
use std::str::FromStr;

use flate2::bufread::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use glam::{DVec3, IVec3};
use thiserror::Error;
use valence_biome::BiomeId;
use valence_block::{BlockEntityKind, BlockState, ParseBlockStateError};
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::ident::Ident;
use valence_core::protocol::var_int::{VarInt, VarIntDecodeError};
use valence_core::protocol::Encode;
use valence_instance::chunk::Chunk;
use valence_instance::{Block as ValenceBlock, Instance};
use valence_nbt::{compound, Compound, List, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Schematic {
    pub metadata: Option<Compound>,
    pub width: u16,
    pub height: u16,
    pub length: u16,
    pub offset: IVec3,
    blocks: Option<Box<[Block]>>,
    biomes: Option<Biomes>,
    pub entities: Option<Vec<Entity>>,
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
    data: BiomeData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BiomeData {
    /// One biome per y-column. Used in spec version 2.
    /// Indexed by x + z * Width
    Columns(Box<[usize]>),
    /// One biome per block. Used in spec version 3.
    /// Indexed by x + z * Width + y * Width * Length
    Blocks(Box<[usize]>),
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
    Nbt(#[from] valence_nbt::binary::Error),

    #[error("missing schematic")]
    MissingSchematic,

    #[error("missing version")]
    MissingVersion,

    #[error("unknown version {0} (only versions 1 through 3 are supported)")]
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

    #[error("block {0} not in palette {1:?}")]
    BlockNotInPalette(i32, HashMap<i32, BlockState>),

    #[error("unknown block state id {0}")]
    UnknownBlockStateId(i32),

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

    #[error("biome {0} not in palette {1:?}")]
    BiomeNotInPalette(i32, HashMap<i32, Ident<String>>),

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

struct VarIntWriteWrapper<'a>(&'a mut Vec<i8>);
impl<'a> Write for VarIntWriteWrapper<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend(buf.iter().map(|byte| *byte as i8));
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum SaveSchematicError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Nbt(#[from] valence_nbt::binary::Error),
}

impl Schematic {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, LoadSchematicError> {
        let file = File::open(path)?;

        let mut buf = vec![];
        let mut z = GzDecoder::new(BufReader::new(file));
        z.read_to_end(&mut buf)?;

        let root = Compound::from_binary(&mut buf.as_slice())?.0;
        Self::deserialize(&root)
    }

    pub fn deserialize(root: &Compound) -> Result<Self, LoadSchematicError> {
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
        if !(1..=3).contains(&version) {
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
            let &[x, y, z] = root
                .get("Offset")
                .and_then(|val| val.as_int_array())
                .map(|arr| arr.as_slice())
                .unwrap_or(&[0; 3])
            else {
                return Err(LoadSchematicError::InvalidOffset);
            };
            IVec3::new(x, y, z)
        };
        let blocks = match version {
            1 | 2 => {
                let palette = match root.get("Palette") {
                    Some(Value::Compound(palette)) => {
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
                        Some(palette?)
                    }
                    _ => None,
                };

                let Some(Value::ByteArray(data)) = root.get("BlockData") else {
                    return Err(LoadSchematicError::MissingBlockData);
                };
                let data: Result<Vec<_>, LoadSchematicError> =
                    VarIntReader(data.iter().map(|byte| *byte as u8))
                        .map(|val| {
                            let val = val?;
                            let state = match &palette {
                                Some(palette) => match palette.get(&val) {
                                    Some(val) => *val,
                                    None => {
                                        return Err(LoadSchematicError::BlockNotInPalette(
                                            val,
                                            palette.clone(),
                                        ))
                                    }
                                },
                                None => match BlockState::from_raw(val.try_into().unwrap()) {
                                    Some(val) => val,
                                    None => {
                                        return Err(LoadSchematicError::UnknownBlockStateId(val))
                                    }
                                },
                            };
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

                if let Some(Value::List(List::Compound(block_entities))) = root.get(match version {
                    1 => "TileEntities",
                    2 => "BlockEntities",
                    _ => unreachable!(),
                }) {
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

                        let mut nbt = block_entity.clone();
                        nbt.remove("Pos");
                        nbt.remove("Id");
                        let block_entity = BlockEntity { kind, data: nbt };
                        data[(x + z * width as i32 + y * width as i32 * length as i32) as usize]
                            .block_entity
                            .replace(block_entity);
                    }
                }

                Some(data.into_boxed_slice())
            }
            3 => match root.get("Blocks") {
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
                                let val = val?;
                                let state = match palette.get(&val) {
                                    Some(val) => *val,
                                    None => {
                                        return Err(LoadSchematicError::BlockNotInPalette(
                                            val,
                                            palette.clone(),
                                        ))
                                    }
                                };
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
                            data[(x + z * width as i32 + y * width as i32 * length as i32)
                                as usize]
                                .block_entity
                                .replace(block_entity);
                        }
                    }
                    Some(data.into_boxed_slice())
                }
                _ => None,
            },
            _ => unreachable!(),
        };

        let biomes = match version {
            1 => None,
            2 => {
                let Some(Value::Compound(palette)) = root.get("BiomePalette") else {
                    return Err(LoadSchematicError::MissingBiomePalette);
                };
                let palette: Result<HashMap<_, _>, _> = palette
                    .iter()
                    .map(|(biome, value)| {
                        let &Value::Int(i) = value else {
                            return Err(LoadSchematicError::InvalidBiomePalette);
                        };
                        let Ok(ident) = Ident::new(biome) else {
                            return Err(LoadSchematicError::InvalidBiomeIdent(biome.clone()));
                        };
                        Ok((i, ident.to_string_ident()))
                    })
                    .collect();
                let palette = palette?;

                let Some(Value::ByteArray(data)) = root.get("BiomesData") else {
                    return Err(LoadSchematicError::MissingBiomeData);
                };
                let data: Result<Vec<_>, LoadSchematicError> =
                    VarIntReader(data.iter().map(|byte| *byte as u8))
                        .map(|val| {
                            let val = val?;
                            match palette.get(&val) {
                                Some(val) => Ok(val),
                                None => {
                                    Err(LoadSchematicError::BiomeNotInPalette(val, palette.clone()))
                                }
                            }
                        })
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

                if u16::try_from(data.len()) != Ok(width * length) {
                    return Err(LoadSchematicError::InvalidBiomeCount);
                }

                let biomes = Biomes {
                    palette: palette.into_boxed_slice(),
                    data: BiomeData::Columns(data.into_boxed_slice()),
                };
                Some(biomes)
            }
            3 => match root.get("Biomes") {
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
                        data: BiomeData::Blocks(data.into_boxed_slice()),
                    };
                    Some(biomes)
                }
                _ => None,
            },
            _ => unreachable!(),
        };

        let entities = match root.get("Entities") {
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
                Some(entities?)
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

    /// When saving make sure to use gzip
    pub fn serialize(&self) -> Compound {
        let mut compound = compound! {
            "Version" => 3,
            "DataVersion" => 3218,
            "Width" => self.width as i16,
            "Height" => self.height as i16,
            "Length" => self.length as i16,
        };
        if let Some(metadata) = &self.metadata {
            compound.insert("Metadata", metadata.clone());
        }
        match self.offset {
            IVec3::ZERO => {}
            IVec3 { x, y, z } => {
                compound.insert("Offset", vec![x, y, z]);
            }
        }
        if let Some(blocks) = &self.blocks {
            let blocks: Compound = {
                let mut palette = HashMap::new();
                let mut data: Vec<i8> = vec![];
                let mut block_entities = vec![];
                for (idx, block) in blocks.iter().enumerate() {
                    let palette_len = palette.len();
                    let i = *palette.entry(block.state).or_insert(palette_len);
                    struct WriteWrapper<'a>(&'a mut Vec<i8>);
                    impl<'a> Write for WriteWrapper<'a> {
                        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                            self.0.extend(buf.iter().map(|byte| *byte as i8));
                            Ok(buf.len())
                        }

                        fn flush(&mut self) -> io::Result<()> {
                            Ok(())
                        }
                    }

                    VarInt(i as i32).encode(WriteWrapper(&mut data)).unwrap();
                    if let Some(BlockEntity { kind, data }) = &block.block_entity {
                        let idx = u16::try_from(idx).unwrap();
                        let y = idx / (self.width * self.length);
                        let z = (idx % (self.width * self.length)) / self.width;
                        let x = (idx % (self.width * self.length)) % self.width;

                        let mut block_entity = compound! {
                            "Pos" => vec![x as i32, y as i32, z as i32],
                            "Id" => kind.ident(),
                        };
                        if !data.is_empty() {
                            block_entity.insert("Data", data.clone());
                        }
                        block_entities.push(block_entity);
                    }
                }

                let palette: Compound = palette
                    .into_iter()
                    .map(|(state, idx)| (state.to_string(), Value::Int(idx as i32)))
                    .collect();
                let mut blocks = compound! {
                    "Palette" => palette,
                    "Data" => data,
                };
                if !block_entities.is_empty() {
                    blocks.insert("BlockEntities", Value::List(List::Compound(block_entities)));
                }
                blocks
            };
            compound.insert("Blocks", blocks);
        }

        if let Some(biomes) = &self.biomes {
            let palette: Compound = biomes
                .palette
                .iter()
                .enumerate()
                .map(|(idx, val)| (val.to_string(), Value::Int(idx as i32)))
                .collect();
            let mut data = vec![];
            match &biomes.data {
                BiomeData::Columns(biome_data) => {
                    for _ in 0..self.height {
                        for i in biome_data.iter() {
                            VarInt(*i as i32)
                                .encode(VarIntWriteWrapper(&mut data))
                                .unwrap();
                        }
                    }
                }
                BiomeData::Blocks(biome_data) => {
                    for i in biome_data.iter() {
                        VarInt(*i as i32)
                            .encode(VarIntWriteWrapper(&mut data))
                            .unwrap();
                    }
                }
            }
            compound.insert(
                "Biomes",
                compound! {
                    "Palette" => palette,
                    "Data" => data,
                },
            );
        }

        if let Some(entities) = &self.entities {
            let entities = entities
                .iter()
                .map(
                    |Entity {
                         pos: DVec3 { x, y, z },
                         id,
                         data,
                     }| {
                        let mut compound = compound! {
                            "Pos" => Value::List(List::Double(vec![*x, *y, *z])),
                            "Id" => id.clone(),
                        };
                        if let Some(data) = data {
                            compound.insert("Data", data.clone());
                        }
                        compound
                    },
                )
                .collect();
            compound.insert("Entities", Value::List(List::Compound(entities)));
        }

        compound! {
            "Schematic" => compound,
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), SaveSchematicError> {
        let nbt = self.serialize();
        let file = File::create(path)?;
        let mut z = GzEncoder::new(file, Compression::best());
        nbt.to_binary(&mut z, "")?;
        z.flush()?;
        Ok(())
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
                let block = ValenceBlock::new(
                    *state,
                    block_entity
                        .as_ref()
                        .map(|block_entity| block_entity.data.clone()),
                );
                chunk.set_block(
                    block_pos.x.rem_euclid(16) as u32,
                    (block_pos.y - min_y) as u32,
                    block_pos.z.rem_euclid(16) as u32,
                    block,
                );
            }
        }

        if let Some(Biomes { palette, data }) = &self.biomes {
            let data: Box<dyn Iterator<Item = _>> = match data {
                BiomeData::Columns(data) => Box::new(
                    data.iter()
                        .map(|biome| palette[*biome].as_str_ident())
                        .map(map_biome)
                        .enumerate()
                        .flat_map(|(idx, biome)| {
                            let idx = u16::try_from(idx).unwrap();
                            let z = idx / self.width;
                            let x = idx % self.width;

                            (0..self.height).map(move |y| ([x, y, z], biome))
                        }),
                ),
                BiomeData::Blocks(data) => Box::new(
                    data.iter()
                        .map(|biome| palette[*biome].as_str_ident())
                        .map(map_biome)
                        .enumerate()
                        .map(|(idx, biome)| {
                            let idx = u16::try_from(idx).unwrap();
                            let y = idx / (self.width * self.length);
                            let z = (idx % (self.width * self.length)) / self.width;
                            let x = (idx % (self.width * self.length)) % self.width;

                            ([x, y, z], biome)
                        }),
                ),
            };
            for ([x, y, z], biome) in data {
                let x = x as i32 + origin.x + self.offset.x;
                let y = y as i32 + origin.y + self.offset.y;
                let z = z as i32 + origin.z + self.offset.z;
                let chunk = instance
                    .chunk_entry(ChunkPos::at(x as f64, z as f64))
                    .or_default();

                chunk.set_biome(
                    (x / 4).rem_euclid(4) as u32,
                    ((y - min_y) / 4) as u32,
                    (z / 4).rem_euclid(4) as u32,
                    biome,
                );
            }
        }

        // TODO: Spawn entities
    }

    pub fn copy<F>(
        instance: &Instance,
        corners: (BlockPos, BlockPos),
        origin: BlockPos,
        mut map_biome: F,
    ) -> Self
    where
        F: FnMut(BiomeId) -> Ident<String>,
    {
        let min = BlockPos::new(
            corners.0.x.min(corners.1.x),
            corners.0.y.min(corners.1.y),
            corners.0.z.min(corners.1.z),
        );
        let max = BlockPos::new(
            corners.0.x.max(corners.1.x),
            corners.0.y.max(corners.1.y),
            corners.0.z.max(corners.1.z),
        );
        let width = u16::try_from(max.x - min.x + 1).expect("width too large");
        let height = u16::try_from(max.y - min.y + 1).expect("height too large");
        let length = u16::try_from(max.z - min.z + 1).expect("length too large");
        let offset = IVec3::new(min.x - origin.x, min.y - origin.y, min.z - origin.z);
        let blocks: Vec<_> = (min.y..=max.y)
            .flat_map(|y| {
                (min.z..=max.z).flat_map(move |z| {
                    (min.x..=max.x).map(move |x| {
                        let Some(block) = instance.block([x, y, z]) else {
                            panic!("coordinates ({x} {y} {z}) are out of bounds");
                        };
                        let state = block.state;
                        let block_entity = block.nbt.and_then(|data| {
                            Some(BlockEntity {
                                kind: state.block_entity_kind()?,
                                data: data.clone(),
                            })
                        });
                        Block {
                            state,
                            block_entity,
                        }
                    })
                })
            })
            .collect();
        let biomes = {
            let mut palette = vec![];
            let mut map = HashMap::new();
            let data: Vec<_> = (min.x..=max.x)
                .flat_map(|x| {
                    (min.z..=max.z).flat_map(move |z| {
                        (min.y..=max.y).map(move |y| {
                            instance
                                .chunk(ChunkPos::from_block_pos(BlockPos::new(x, y, z)))
                                .unwrap()
                                .biome(
                                    x.rem_euclid(16) as u32 / 4,
                                    (y - instance.min_y()) as u32 / 4,
                                    z.rem_euclid(16) as u32 / 4,
                                )
                        })
                    })
                })
                .map(|biome| match map.entry(biome) {
                    Entry::Occupied(entry) => *entry.get(),
                    Entry::Vacant(entry) => {
                        let idx = palette.len();
                        palette.push(map_biome(biome));
                        entry.insert(idx);
                        idx
                    }
                })
                .collect();

            Biomes {
                palette: palette.into_boxed_slice(),
                data: BiomeData::Blocks(data.into_boxed_slice()),
            }
        };
        Self {
            metadata: None,
            width,
            height,
            length,
            offset,
            blocks: Some(blocks.into_boxed_slice()),
            biomes: Some(biomes),
            entities: None, // TODO
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use valence::prelude::*;
    use valence_core::ident;

    use super::*;

    #[test]
    fn schematic_copy_paste() {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        app.update();
        let mut instance = Instance::new(
            ident!("overworld"),
            app.world.resource(),
            app.world.resource(),
            app.world.resource(),
        );

        for x in -1..=0 {
            for z in -1..=0 {
                instance.insert_chunk([x, z], UnloadedChunk::default());
            }
        }

        instance.set_block([5, 1, -1], BlockState::GLOWSTONE);
        instance.set_block([5, 2, -1], BlockState::STONE);
        instance.set_block([5, 2, -2], BlockState::GLOWSTONE);
        instance.set_block([4, 2, -1], BlockState::LAPIS_BLOCK);
        instance.set_block([6, 2, -1], BlockState::STONE);
        instance.set_block(
            [5, 3, -1],
            ValenceBlock::new(
                BlockState::OAK_SIGN,
                Some(compound! {"Text1" => "abc".into_text()}),
            ),
        );
        instance.set_block(
            [5, 2, 0],
            BlockState::ANDESITE_WALL
                .set(PropName::Up, PropValue::True)
                .set(PropName::North, PropValue::Low),
        );

        let schematic = Schematic::copy(
            &instance,
            (BlockPos::new(4, 3, -1), BlockPos::new(6, 1, 0)),
            BlockPos::new(5, 3, 0),
            |_| ident!("minecraft:plains").to_string_ident(),
        );

        schematic.paste(&mut instance, BlockPos::new(15, 18, 16), |_| {
            BiomeId::default()
        });

        let block = instance.block([15, 18, 15]).unwrap();
        assert_eq!(block.state, BlockState::OAK_SIGN);
        assert_eq!(block.nbt, Some(&compound! {"Text1" => "abc".into_text()}));

        let block = instance.block([15, 17, 16]).unwrap();
        assert_eq!(
            block.state,
            BlockState::ANDESITE_WALL
                .set(PropName::Up, PropValue::True)
                .set(PropName::North, PropValue::Low)
        );
        assert_eq!(block.nbt, None);

        let block = instance.block([15, 17, 15]).unwrap();
        assert_eq!(block.state, BlockState::STONE);
        assert_eq!(block.nbt, None);

        let block = instance.block([15, 17, 14]).unwrap();
        assert_eq!(block.state, BlockState::AIR);
        assert_eq!(block.nbt, None);

        let block = instance.block([14, 17, 15]).unwrap();
        assert_eq!(block.state, BlockState::LAPIS_BLOCK);
        assert_eq!(block.nbt, None);

        let block = instance.block([16, 17, 15]).unwrap();
        assert_eq!(block.state, BlockState::STONE);
        assert_eq!(block.nbt, None);

        let block = instance.block([15, 16, 15]).unwrap();
        assert_eq!(block.state, BlockState::GLOWSTONE);
        assert_eq!(block.nbt, None);

        let mut schematic = schematic;
        schematic.metadata.replace(compound! {"A" => 123});
        let nbt = schematic.serialize();
        assert_eq!(
            nbt,
            compound! {
                "Schematic" => compound! {
                    "Version" => 3,
                    "DataVersion" => 3218,
                    "Metadata" => compound! {
                        "A" => 123,
                    },
                    "Width" => 3i16,
                    "Height" => 3i16,
                    "Length" => 2i16,
                    "Offset" => vec![-1, -2, -1],
                    "Blocks" => compound! {
                        "Data" => vec![0i8, 1, 0, 0, 0, 0, 2, 3, 3, 0, 4, 0, 0, 5, 0, 0, 0, 0],
                        "Palette" => compound! {
                            "air" => 0,
                            "glowstone" => 1,
                            "lapis_block" => 2,
                            "stone" => 3,
                            "andesite_wall[east=none, north=low, south=none, up=true, waterlogged=false, west=none]" => 4,
                            "oak_sign[rotation=0, waterlogged=false]" => 5,
                        },
                        "BlockEntities" => Value::List(List::Compound(vec![
                            compound! {
                                "Data" => compound!{
                                    "Text1" => "abc".into_text(),
                                },
                                "Id" => "minecraft:sign",
                                "Pos" => vec![1, 2, 0],
                            },
                        ]))
                    },
                    "Biomes" => compound! {
                        "Data" => vec![0i8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                        "Palette" => compound! {
                            "minecraft:plains" => 0,
                        }
                    },
                }
            }
        );
    }

    #[test]
    fn schematic_load_save() {
        let schem1 = Schematic::load("../../assets/example_schem.schem").unwrap();
        const TEST_PATH: &str = "test.schem";
        schem1.save(TEST_PATH).unwrap();
        let schem2 = Schematic::load(TEST_PATH).unwrap();
        assert_eq!(schem1, schem2);
        fs::remove_file(TEST_PATH).unwrap();
    }
}
