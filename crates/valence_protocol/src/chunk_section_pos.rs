use std::fmt;
use std::io::Write;

use bitfield_struct::bitfield;
use derive_more::From;
use thiserror::Error;

use crate::{BiomePos, BlockPos, Decode, Encode};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ChunkSectionPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkSectionPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn packed(self) -> Result<PackedChunkSectionPos, Error> {
        match (self.x, self.y, self.z) {
            (-2097152..=2097151, -524288..=524287, -2097152..=2097151) => {
                Ok(PackedChunkSectionPos::new()
                    .with_x(self.x)
                    .with_y(self.y)
                    .with_z(self.z))
            }
            _ => Err(Error(self)),
        }
    }
}

impl fmt::Display for ChunkSectionPos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&(self.x, self.y, self.z), f)
    }
}

impl Encode for ChunkSectionPos {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.packed()?.encode(w)
    }
}

impl Decode<'_> for ChunkSectionPos {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        PackedChunkSectionPos::decode(r).map(Into::into)
    }
}

impl From<BlockPos> for ChunkSectionPos {
    fn from(pos: BlockPos) -> Self {
        Self {
            x: pos.x.div_euclid(16),
            y: pos.y.div_euclid(16),
            z: pos.z.div_euclid(16),
        }
    }
}

impl From<BiomePos> for ChunkSectionPos {
    fn from(pos: BiomePos) -> Self {
        Self {
            x: pos.x.div_euclid(4),
            y: pos.y.div_euclid(4),
            z: pos.z.div_euclid(4),
        }
    }
}

#[bitfield(u64)]
#[derive(PartialEq, Eq, Ord, PartialOrd, Encode, Decode)]
pub struct PackedChunkSectionPos {
    #[bits(20)]
    pub y: i32,
    #[bits(22)]
    pub z: i32,
    #[bits(22)]
    pub x: i32,
}

impl From<PackedChunkSectionPos> for ChunkSectionPos {
    fn from(pos: PackedChunkSectionPos) -> Self {
        Self {
            x: pos.x(),
            y: pos.y(),
            z: pos.z(),
        }
    }
}

impl TryFrom<ChunkSectionPos> for PackedChunkSectionPos {
    type Error = Error;

    fn try_from(pos: ChunkSectionPos) -> Result<Self, Self::Error> {
        pos.packed()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Error, From)]
#[error("chunk section position of {0} is out of range")]
pub struct Error(pub ChunkSectionPos);
