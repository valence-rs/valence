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
use glam::IVec3;
use thiserror::Error;
use valence_biome::BiomeId;
use valence_block::{BlockEntityKind, BlockState, ParseBlockStateError};
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::ident::Ident;
use valence_core::packet::var_int::{VarInt, VarIntDecodeError};
use valence_instance::{BlockEntity as ValenceBlockEntity, Instance};
use valence_nbt::{Compound, List, Value};

pub struct Schematic {
    pub metadata: Option<Compound>,
    pub width: u16,
    pub height: u16,
    pub length: u16,
    pub offset: IVec3,
    block_data: Box<[BlockState]>,
    block_entities: Box<[BlockEntity]>,
    // TODO: pub entities: Box<[Entity]>,
    biome_palette: Box<[Ident<String>]>,
    biome_data: Box<[usize]>,
}

pub struct BlockEntity {
    pub pos: BlockPos,
    pub kind: BlockEntityKind,
    pub nbt: Compound,
}

#[derive(Debug, Error)]
pub enum LoadSchematicError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Nbt(#[from] valence_nbt::Error),

    #[error("missing version")]
    MissingVersion,

    #[error("unknown version {0}")]
    UnknownVersion(i32),

    #[error("missing width")]
    MissingWidth,

    #[error("missing height")]
    MissingHeight,

    #[error("missing length")]
    MissingLength,

    #[error("missing offset")]
    MissingOffset,

    #[error("missing palette")]
    MissingPalette,

    #[error("invalid palette")]
    InvalidPalette,

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

// fn read_varints<T>(
//     iter: impl ExactSizeIterator<Item = u8>,
//     transform: impl Fn(i32) -> T,
// ) -> Result<Vec<T>, VarIntDecodeError> {
//     let mut data = vec![];
//     let mut block_data = ReadWrapper(iter);
//     while block_data.0.len() > 0 {
//         let var_int = VarInt::decode_partial(&mut block_data)?;
//         data.push(transform(var_int));
//     }
//     Ok(data)
// }

impl Schematic {
    pub fn load(path: PathBuf) -> Result<Self, LoadSchematicError> {
        let file = File::open(path)?;

        let mut buf = vec![];
        let mut z = GzDecoder::new(BufReader::new(file));
        z.read_to_end(&mut buf)?;

        let data = valence_nbt::from_binary_slice(&mut buf.as_slice())?.0;

        let metadata = data
            .get("Metadata")
            .and_then(|val| val.as_compound())
            .cloned();

        let Some(&Value::Int(version)) = data.get("Version") else {
            return Err(LoadSchematicError::MissingVersion);
        };
        if version != 2 {
            return Err(LoadSchematicError::UnknownVersion(version));
        }
        let Some(&Value::Short(width)) = data.get("Width") else {
            return Err(LoadSchematicError::MissingWidth);
        };
        let width = width as u16;
        let Some(&Value::Short(height)) = data.get("Height") else {
            return Err(LoadSchematicError::MissingHeight);
        };
        let height = height as u16;
        let Some(&Value::Short(length)) = data.get("Length") else {
            return Err(LoadSchematicError::MissingLength);
        };
        let length = length as u16;
        let offset = {
            let &[x, y, z] = data.get("Offset").and_then(|val| val.as_int_array()).map(|arr| arr.as_slice()).unwrap_or(&[0; 3]) else {
                return Err(LoadSchematicError::MissingOffset);
            };
            IVec3::new(x, y, z)
        };
        let palette: HashMap<i32, BlockState> = {
            let Some(Value::Compound(palette)) = data.get("Palette") else {
                return Err(LoadSchematicError::MissingPalette);
            };
            let palette: Result<_, _> = palette
                .into_iter()
                .map(|(state, value)| {
                    let &Value::Int(i) = value else {
                        return Err(LoadSchematicError::InvalidPalette);
                    };
                    let state =
                        BlockState::from_str(state.strip_prefix("minecraft:").unwrap_or(state))?;
                    Ok((i, state))
                })
                .collect();
            palette?
        };
        let block_data = {
            let Some(Value::ByteArray(block_data)) = data.get("BlockData") else {
                return Err(LoadSchematicError::MissingBlockData);
            };

            let data: Result<Vec<_>, LoadSchematicError> =
                VarIntReader(block_data.iter().map(|byte| *byte as u8))
                    .map(|val| Ok(palette[&val?]))
                    .collect();
            let data = data?;

            if u16::try_from(data.len()) != Ok(width * height * length) {
                return Err(LoadSchematicError::InvalidBlockCount);
            }

            data.into_boxed_slice()
        };
        let block_entities = if let Some(Value::List(List::Compound(block_entities))) =
            data.get("BlockEntities")
        {
            let block_entities: Result<Vec<_>, _> = block_entities.iter().map(|block_entity| {
                let Some(&[x, y, z]) = block_entity.get("Pos").and_then(|val| val.as_int_array()).map(|arr| arr.as_slice()) else {
                    return Err(LoadSchematicError::MissingBlockEntityPos);
                };
                let pos = BlockPos::new(x, y, z);
                let kind = {
                    let Some(Value::String(id)) = block_entity.get("Id") else {
                        return Err(LoadSchematicError::MissingBlockEntityId);
                    };
                    if id.is_empty() {
                        block_data[(x + z * width as i32 + y * width as i32 * length as i32) as usize].block_entity_kind().ok_or(LoadSchematicError::MissingBlockEntityId)?
                    } else {
                        let Ok(id) = Ident::new(id) else {
                            return Err(LoadSchematicError::InvalidBlockEntityId(id.clone()));
                        };
                        let Some(kind) = BlockEntityKind::from_ident(id.as_str_ident()) else {
                            return Err(LoadSchematicError::UnknownBlockEntity(id.to_string()));
                        };
                        kind
                    }
                };
                let mut nbt = block_entity.clone();
                nbt.remove("Pos");
                nbt.remove("Id");
                Ok(BlockEntity { pos, kind, nbt })
            }).collect();

            block_entities?.into_boxed_slice()
        } else {
            Box::new([])
        };

        let Some(Value::Compound(biome_palette)) = data.get("BiomePalette") else {
            return Err(LoadSchematicError::MissingBiomePalette);
        };
        let biome_palette: Result<HashMap<_, _>, _> = biome_palette
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
        let biome_palette = biome_palette?;

        let Some(Value::ByteArray(biome_data)) = data.get("BiomeData") else {
            return Err(LoadSchematicError::MissingBiomeData);
        };
        let biome_data: Result<Vec<_>, LoadSchematicError> =
            VarIntReader(biome_data.iter().map(|byte| *byte as u8))
                .map(|val| Ok(&biome_palette[&val?]))
                .collect();
        let biome_data = biome_data?;

        let (biome_palette, biome_data) = {
            let mut palette = vec![];
            let mut map = HashMap::new();
            let biome_data: Vec<_> = biome_data
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

            (palette.into_boxed_slice(), biome_data.into_boxed_slice())
        };

        Ok(Self {
            metadata,
            width,
            height,
            length,
            offset,
            block_data,
            block_entities,
            biome_palette,
            biome_data,
        })
    }

    pub fn paste<F>(&self, instance: &mut Instance, origin: BlockPos, map_biome: F)
    where
        F: FnMut(Ident<&str>) -> BiomeId,
    {
        let blocks = self.block_data.iter().enumerate().map(|(idx, state)| {
            let idx = u16::try_from(idx).unwrap();
            let y = idx / (self.width * self.length);
            let z = (idx % (self.width * self.length)) / self.width;
            let x = (idx % (self.width * self.length)) % self.width;

            ([x, y, z], state)
        });

        let min_y = instance.min_y();
        for ([x, y, z], state) in blocks {
            let block_pos = BlockPos::new(
                x as i32 + origin.x - self.offset.x,
                y as i32 + origin.y - self.offset.y,
                z as i32 + origin.z - self.offset.z,
            );
            let chunk = instance
                .chunk_entry(ChunkPos::from_block_pos(block_pos))
                .or_default();
            chunk.set_block_state(
                block_pos.x.rem_euclid(16) as usize,
                (block_pos.y - min_y) as usize,
                block_pos.z.rem_euclid(16) as usize,
                *state,
            );
        }

        for BlockEntity {
            pos: BlockPos { x, y, z },
            kind,
            nbt,
        } in self.block_entities.iter()
        {
            let block_pos = BlockPos::new(
                x + origin.x - self.offset.x,
                y + origin.y - self.offset.y,
                z + origin.z - self.offset.z,
            );
            let chunk = instance
                .chunk_entry(ChunkPos::from_block_pos(block_pos))
                .or_default();
            let x = block_pos.x.rem_euclid(16) as usize;
            let y = (block_pos.y - min_y) as usize;
            let z = block_pos.z.rem_euclid(16) as usize;
            chunk.set_block_entity(
                x,
                y,
                z,
                ValenceBlockEntity {
                    kind: *kind,
                    nbt: nbt.clone(),
                },
            );
        }

        let section_count = instance.section_count();
        for ([x, z], biome) in self
            .biome_data
            .iter()
            .map(|biome| self.biome_palette[*biome].as_str_ident())
            .map(map_biome)
            .enumerate()
            .map(|(idx, biome)| {
                let idx = u16::try_from(idx).unwrap();
                let z = idx / self.width;
                let x = idx % self.width;

                ([x, z], biome)
            })
        {
            let x = x as i32 + origin.x - self.offset.x;
            let z = z as i32 + origin.z - self.offset.z;
            let chunk = instance
                .chunk_entry(ChunkPos::at(x as f64, z as f64))
                .or_default();

            for y in 0..section_count * 4 {
                chunk.set_biome(
                    (x / 4).rem_euclid(4) as usize,
                    y,
                    (z / 4).rem_euclid(4) as usize,
                    biome,
                );
            }
        }
    }
}
