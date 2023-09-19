use std::borrow::Cow;

use bevy_ecs::prelude::*;
use bitfield_struct::bitfield;
use valence_generated::block::BlockEntityKind;
use valence_nbt::Compound;
use valence_protocol::packets::play::chunk_delta_update_s2c::ChunkDeltaUpdateEntry;
use valence_protocol::packets::play::{BlockEntityUpdateS2c, BlockUpdateS2c, ChunkDeltaUpdateS2c};
use valence_protocol::{BiomePos, BlockPos, ChunkSectionPos, WritePacket};
use valence_registry::biome::BiomeId;

use super::block::Block;
use super::chunk::LoadedChunk;
use super::{ChunkIndex, DimensionInfo};
use crate::dimension_layer::chunk::ChunkOps;
use crate::layer::message::{LayerMessages, MessageScope};
use crate::BlockState;

/// Batched block and biome mutations.
///
/// Changes are automatically applied at the end of the tick or when
/// [`apply_batch`] is called.
///
/// [`apply_batch`]: super::DimensionLayerQueryItem::apply_batch
#[derive(Component, Default)]
pub struct Batch {
    state_updates: Vec<StateUpdate>,
    block_entities: Vec<(BlockPos, BlockEntityKind, Compound)>,
    sect_update_buf: Vec<ChunkDeltaUpdateEntry>,
}

impl Batch {
    pub fn set_block(&mut self, pos: impl Into<BlockPos>, block: impl Into<Block>) {
        let pos = pos.into();
        let block = block.into();

        self.state_updates
            .push(StateUpdate::from_parts(pos, block.state));

        if let (Some(nbt), Some(kind)) = (block.nbt, block.state.block_entity_kind()) {
            self.block_entities.push((pos, kind, nbt));
        }
    }

    pub fn set_biome(&mut self, pos: impl Into<BiomePos>, biome: BiomeId) {
        todo!()
    }

    pub fn clear(&mut self) {
        self.state_updates.clear();
        self.block_entities.clear();
    }

    pub fn shrink_to_fit(&mut self) {
        let Self {
            state_updates,
            block_entities,
            sect_update_buf,
        } = self;

        state_updates.shrink_to_fit();
        block_entities.shrink_to_fit();
        sect_update_buf.shrink_to_fit();
    }

    pub fn is_empty(&self) -> bool {
        self.state_updates.is_empty()
    }

    pub(super) fn apply(
        &mut self,
        chunks: &mut ChunkIndex,
        info: &DimensionInfo,
        messages: &mut LayerMessages,
    ) {
        debug_assert!(self.sect_update_buf.is_empty());

        // Sort block state updates so that they're grouped by chunk section.
        // Sort in reverse so the dedup keeps the last of consecutive elements.
        self.state_updates.sort_unstable_by(|a, b| b.cmp(a));

        // Eliminate redundant block assignments.
        self.state_updates
            .dedup_by_key(|u| u.0 & StateUpdate::BLOCK_POS_MASK);

        let mut chunk: Option<&mut LoadedChunk> = None;
        let mut sect_pos = ChunkSectionPos::new(i32::MIN, i32::MIN, i32::MIN);

        let mut flush_sect_updates =
            |sect_pos: ChunkSectionPos, buf: &mut Vec<ChunkDeltaUpdateEntry>| {
                let mut w = messages.packet_writer(MessageScope::ChunkView {
                    pos: sect_pos.into(),
                });

                match buf.as_slice() {
                    // Zero updates. Do nothing.
                    &[] => {}
                    // One update. Send singular block update packet.
                    &[update] => {
                        w.write_packet(&BlockUpdateS2c {
                            position: BlockPos {
                                x: sect_pos.x * 16 + update.off_x() as i32,
                                y: sect_pos.y * 16 + update.off_y() as i32,
                                z: sect_pos.z * 16 + update.off_z() as i32,
                            },
                            block_id: BlockState::from_raw(update.block_state() as u16).unwrap(),
                        });

                        buf.clear();
                    }
                    // >1 updates. Send special section update packet.
                    updates => {
                        w.write_packet(&ChunkDeltaUpdateS2c {
                            chunk_sect_pos: sect_pos,
                            blocks: Cow::Borrowed(updates),
                        });

                        buf.clear();
                    }
                }
            };

        // For each block state change...
        for (pos, state) in self.state_updates.drain(..).map(StateUpdate::to_parts) {
            let new_sect_pos = ChunkSectionPos::from(pos);

            // Is this block in a new section? If it is, then flush the changes we've
            // accumulated for the old section.
            if sect_pos != new_sect_pos {
                flush_sect_updates(sect_pos, &mut self.sect_update_buf);

                // Update the chunk ref if the chunk pos changed.
                if sect_pos.x != new_sect_pos.x || sect_pos.z != new_sect_pos.z {
                    chunk = chunks.get_mut(new_sect_pos);
                }

                // Update section pos
                sect_pos = new_sect_pos;
            }

            // Apply block state update to chunk.
            if let Some(chunk) = &mut chunk {
                let chunk_y = pos.y.wrapping_sub(info.min_y) as u32;

                // Is the block pos in bounds of the chunk?
                if chunk_y < info.height as u32 {
                    let chunk_x = pos.x.rem_euclid(16);
                    let chunk_z = pos.z.rem_euclid(16);

                    // Note that we're using `set_block` and not `set_block_state`.
                    let old_state = chunk
                        .set_block(chunk_x as u32, chunk_y, chunk_z as u32, state)
                        .state;

                    // Was the change observable?
                    if old_state != state && chunk.viewer_count > 0 {
                        self.sect_update_buf.push(
                            ChunkDeltaUpdateEntry::new()
                                .with_off_x(chunk_x as u8)
                                .with_off_y((chunk_y % 16) as u8)
                                .with_off_z(chunk_z as u8)
                                .with_block_state(state.to_raw() as u32),
                        );
                    }
                }
            }
        }

        // Flush remaining block state changes.
        flush_sect_updates(sect_pos, &mut self.sect_update_buf);

        // Send block entity updates. This needs to happen after block states are set.
        for (pos, kind, nbt) in self.block_entities.drain(..) {
            let min_y = info.min_y;
            let height = info.height;

            if let Some(chunk) = chunks.get_mut(pos) {
                let chunk_y = pos.y.wrapping_sub(min_y) as u32;

                // Is the block pos in bounds of the chunk?
                if chunk_y < height as u32 {
                    let chunk_x = pos.x.rem_euclid(16);
                    let chunk_z = pos.z.rem_euclid(16);

                    let mut w = messages.packet_writer(MessageScope::ChunkView { pos: pos.into() });
                    w.write_packet(&BlockEntityUpdateS2c {
                        position: pos,
                        kind,
                        data: Cow::Borrowed(&nbt),
                    });

                    chunk.set_block_entity(chunk_x as u32, chunk_y, chunk_z as u32, Some(nbt));
                }
            }
        }
    }
}

#[bitfield(u128, order = Msb)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct StateUpdate {
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
    /// Bits of the [`BlockState`].
    state: u32,
}

impl StateUpdate {
    const CHUNK_POS_MASK: u128 = u128::MAX << 72;
    const SECTION_POS_MASK: u128 = u128::MAX << 44;
    const BLOCK_POS_MASK: u128 = u128::MAX << 32;

    fn from_parts(pos: BlockPos, state: BlockState) -> Self {
        Self::new()
            .with_section_x(pos.x.div_euclid(16))
            .with_section_y(pos.y.div_euclid(16))
            .with_section_z(pos.z.div_euclid(16))
            .with_off_x(pos.x.rem_euclid(16) as u32)
            .with_off_y(pos.y.rem_euclid(16) as u32)
            .with_off_z(pos.z.rem_euclid(16) as u32)
            .with_state(state.to_raw() as u32)
    }

    fn to_parts(self) -> (BlockPos, BlockState) {
        (
            BlockPos {
                x: self.section_x() * 16 + self.off_x() as i32,
                y: self.section_y() * 16 + self.off_y() as i32,
                z: self.section_z() * 16 + self.off_z() as i32,
            },
            BlockState::from_raw(self.state() as u16).unwrap(),
        )
    }
}

// TODO: unit test.
