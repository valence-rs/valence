//! Handles getting and setting biomes in chunk layers.

use valence_math::DVec3;
use valence_protocol::{BlockPos, ChunkPos};
use valence_registry::biome::BiomeId;

use super::{ChunkOps, LoadedChunk};
use crate::ChunkLayer;

/// Identifies the position of a biome in a world.
///
/// Every biome occupies a 4m³ area, so conversion from [`BlockPos`] is done by
/// dividing all components by 4 (rounding towards negative infinity).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct BiomePos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl From<BlockPos> for BiomePos {
    fn from(pos: BlockPos) -> Self {
        Self {
            x: pos.x.div_euclid(4),
            y: pos.y.div_euclid(4),
            z: pos.z.div_euclid(4),
        }
    }
}

impl From<BiomePos> for ChunkPos {
    fn from(pos: BiomePos) -> Self {
        Self {
            x: pos.x.div_euclid(4),
            z: pos.z.div_euclid(4),
        }
    }
}

impl From<DVec3> for BiomePos {
    fn from(pos: DVec3) -> Self {
        Self {
            x: (pos.x / 4.0).floor() as i32,
            y: (pos.y / 4.0).floor() as i32,
            z: (pos.z / 4.0).floor() as i32,
        }
    }
}

impl ChunkLayer {
    pub fn biome(&self, pos: impl Into<BiomePos>) -> Option<BiomeId> {
        let pos = pos.into();

        let (chunk, x, y, z) = self.chunk_and_biome_offsets(pos)?;
        Some(chunk.biome(x, y, z))

        // Biomes
        // if self.changed_biomes {
        //     self.changed_biomes = false;

        //     messages.send_local_infallible(LocalMsg::ChangeBiome { pos },
        // |buf| {         for sect in self.sections.iter() {
        //             sect.biomes
        //                 .encode_mc_format(
        //                     &mut *buf,
        //                     |b| b.to_index() as _,
        //                     0,
        //                     3,
        //                     bit_width(info.biome_registry_len - 1),
        //                 )
        //                 .expect("paletted container encode should always
        // succeed");         }
        //     });
        // }
    }

    pub fn set_biome(&mut self, pos: impl Into<BiomePos>, biome: BiomeId) -> Option<BiomeId> {
        let pos = pos.into();

        let (chunk, x, y, z) = self.chunk_and_biome_offsets_mut(pos)?;

        todo!()
    }

    #[inline]
    fn chunk_and_biome_offsets(&self, pos: BiomePos) -> Option<(&LoadedChunk, u32, u32, u32)> {
        let Some(y) = pos
            .y
            .checked_sub(self.info.min_y.div_euclid(4))
            .and_then(|y| y.try_into().ok())
        else {
            return None;
        };

        if y >= self.info.height / 4 {
            return None;
        }

        let Some(chunk) = self.chunk(pos) else {
            return None;
        };

        let x = pos.x.rem_euclid(4) as u32;
        let z = pos.z.rem_euclid(4) as u32;

        Some((chunk, x, y, z))
    }

    #[inline]
    fn chunk_and_biome_offsets_mut(
        &mut self,
        pos: BiomePos,
    ) -> Option<(&mut LoadedChunk, u32, u32, u32)> {
        let Some(y) = pos
            .y
            .checked_sub(self.info.min_y.div_euclid(4))
            .and_then(|y| y.try_into().ok())
        else {
            return None;
        };

        if y >= self.info.height / 4 {
            return None;
        }

        let Some(chunk) = self.chunk_mut(pos) else {
            return None;
        };

        let x = pos.x.rem_euclid(4) as u32;
        let z = pos.z.rem_euclid(4) as u32;

        Some((chunk, x, y, z))
    }
}
