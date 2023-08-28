use bevy_ecs::prelude::Component;
use bitfield_struct::bitfield;
use valence_protocol::{BlockPos, BlockState, ChunkPos};

use super::{Batch, Block};

#[derive(Clone, PartialEq, Default, Debug, Component)]
pub struct BasicBatch {
    updates: Vec<BlockUpdate>,
    full: Vec<Block>,
}

impl BasicBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_block(&mut self, pos: impl Into<BlockPos>, block: impl Into<Block>) {
        let pos = pos.into();
        let block = block.into();

        if block.nbt.is_none() {
            self.updates
                .push(BlockUpdate::from_block_state(pos, block.state))
        } else {
            let idx = self.full.len() as u32;
            self.full.push(block);
            self.updates.push(BlockUpdate::from_index(pos, idx));
        }
    }

    pub fn clear(&mut self) {
        self.updates.clear();
        self.full.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.updates.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.updates.shrink_to_fit();
        self.full.shrink_to_fit();
    }
}

impl<P, B> FromIterator<(P, B)> for BasicBatch
where
    P: Into<BlockPos>,
    B: Into<Block>,
{
    fn from_iter<T: IntoIterator<Item = (P, B)>>(iter: T) -> Self {
        let mut res = Self::new();

        res.extend(iter);

        res
    }
}

impl<P, B> Extend<(P, B)> for BasicBatch
where
    P: Into<BlockPos>,
    B: Into<Block>,
{
    fn extend<T: IntoIterator<Item = (P, B)>>(&mut self, iter: T) {
        self.updates.extend(iter.into_iter().map(|(p, b)| {
            let pos = p.into();
            let block = b.into();

            if block.nbt.is_none() {
                BlockUpdate::from_block_state(pos, block.state)
            } else {
                let idx = self.full.len() as u32;
                self.full.push(block);
                BlockUpdate::from_index(pos, idx)
            }
        }));
    }
}

/// The basic batch is cleared after application so you can reuse the buffer if
/// desired.
impl<'a> Batch for &'a mut BasicBatch {
    type BlockIter = BlockIter<'a>;

    fn into_batch_iters(mut self) -> Self::BlockIter {
        // Sort in reverse so the dedup keeps the last of consecutive elements.
        self.updates.sort_by(|a, b| b.cmp(a));

        // Eliminate redundant block assignments.
        self.updates
            .dedup_by_key(|u| u.0 & BlockUpdate::BLOCK_POS_MASK);

        BlockIter {
            updates: self.updates.drain(..),
            full: &mut self.full,
        }
    }
}

pub struct BlockIter<'a> {
    updates: std::vec::Drain<'a, BlockUpdate>,
    full: &'a mut Vec<Block>,
}

impl<'a> Iterator for BlockIter<'a> {
    type Item = (BlockPos, Block);

    fn next(&mut self) -> Option<Self::Item> {
        let u = self.updates.next()?;
        let pos = u.block_pos();

        let block = if u.is_index() {
            let full = &mut self.full[u.state() as usize];

            Block {
                state: full.state,
                nbt: full.nbt.take(),
            }
        } else {
            Block {
                state: BlockState::from_raw(u.state() as u16).unwrap(),
                nbt: None,
            }
        };

        Some((pos, block))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.updates.size_hint()
    }
}

impl<'a> ExactSizeIterator for BlockIter<'a> {
    fn len(&self) -> usize {
        self.updates.len()
    }
}

impl<'a> Drop for BlockIter<'a> {
    fn drop(&mut self) {
        self.full.clear();
    }
}

#[bitfield(u128, order = Msb)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct BlockUpdate {
    // Section coordinate.
    #[bits(28)]
    section_x: i32,
    #[bits(28)]
    section_z: i32,
    #[bits(28)]
    section_y: i32,
    // Coordinate within the section.
    #[bits(4)]
    off_x: u32,
    #[bits(4)]
    off_z: u32,
    #[bits(4)]
    off_y: u32,
    /// `false` if `state` is a block state, `true` if it's an index into the
    /// `full` array.
    is_index: bool,
    /// Bits of the [`BlockState`] or an index into the `full` array.
    #[bits(31)]
    state: u32,
}

impl BlockUpdate {
    const CHUNK_POS_MASK: u128 = u128::MAX << 72;
    const SECTION_POS_MASK: u128 = u128::MAX << 44;
    const BLOCK_POS_MASK: u128 = u128::MAX << 32;

    fn from_block_state(pos: BlockPos, state: BlockState) -> Self {
        Self::new()
            .with_section_x(pos.x.div_euclid(16))
            .with_section_y(pos.y.div_euclid(16))
            .with_section_z(pos.z.div_euclid(16))
            .with_off_x(pos.x.rem_euclid(16) as u32)
            .with_off_y(pos.y.rem_euclid(16) as u32)
            .with_off_z(pos.z.rem_euclid(16) as u32)
            .with_state(state.to_raw() as u32)
    }

    fn from_index(pos: BlockPos, idx: u32) -> Self {
        Self::new()
            .with_section_x(pos.x.div_euclid(16))
            .with_section_y(pos.y.div_euclid(16))
            .with_section_z(pos.z.div_euclid(16))
            .with_off_x(pos.x.rem_euclid(16) as u32)
            .with_off_y(pos.y.rem_euclid(16) as u32)
            .with_off_z(pos.z.rem_euclid(16) as u32)
            .with_is_index(true)
            .with_state(idx)
    }

    // fn to_parts(self) -> (BlockPos, BlockState) {
    //     (self.block_pos(), self.block())
    // }

    fn block_pos(self) -> BlockPos {
        BlockPos {
            x: self.section_x() * 16 + self.off_x() as i32,
            y: self.section_y() * 16 + self.off_y() as i32,
            z: self.section_z() * 16 + self.off_z() as i32,
        }
    }

    fn chunk_pos(self) -> ChunkPos {
        ChunkPos {
            x: self.section_x(),
            z: self.section_z(),
        }
    }

    fn block(self) -> BlockState {
        BlockState::from_raw(self.state() as u16).unwrap()
    }
}

/*
impl PartialEq for BlockUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for BlockUpdate {}

impl PartialOrd for BlockUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.ord(other))
    }
}

impl Ord for BlockUpdate {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0 & Self::BLOCK_POS_MASK).cmp(&(other.0 & Self::BLOCK_POS_MASK))
    }
}
*/
