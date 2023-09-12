use bevy_ecs::prelude::*;
use bitfield_struct::bitfield;
use valence_generated::block::BlockEntityKind;
use valence_nbt::Compound;
use valence_protocol::encode::PacketWriter;
use valence_protocol::packets::play::chunk_delta_update_s2c::ChunkDeltaUpdateEntry;
use valence_protocol::packets::play::{BlockEntityUpdateS2c, BlockUpdateS2c, ChunkDeltaUpdateS2c};
use valence_protocol::{BlockPos, ChunkPos, ChunkSectionPos, WritePacket};

use super::block::Block;
use super::ChunkIndex;
use crate::layer::{LayerViewers, PacketBuf};
use crate::layer_old::chunk::LocalMsg;
use crate::{BlockState, ChunkLayer, Client, Layer};

#[derive(Component, Default)]
pub struct BlockBatch {
    updates: Vec<BlockUpdate>,
    block_entities: Vec<(BlockPos, BlockEntityKind, Compound)>,
}

impl BlockBatch {
    pub fn set_block(&mut self, pos: impl Into<ChunkPos>, block: impl Into<Block>) {
        let pos = pos.into();
        let block = block.into();

        self.updates.push(BlockUpdate::from_parts(pos, block.state));

        if let Some(nbt) = block.nbt {
            self.block_entities
                .push((pos, block.state.block_entity_kind(), nbt));
        }
    }

    pub fn clear(&mut self) {
        self.updates.clear();
        self.block_entities.clear();
    }

    pub fn shrink_to_fit(&mut self) {
        self.updates.shrink_to_fit();
        self.block_entities.shrink_to_fit();
    }

    pub(super) fn apply(
        &mut self,
        chunks: &mut ChunkIndex,
        viewers: &LayerViewers,
        clients: &mut Query<&mut Client>,
        buf: &mut PacketBuf,
    ) {
        todo!();

        buf.broadcast(clients);
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
    /// Bits of the [`BlockState`].
    state: u32,
}

impl BlockUpdate {
    const CHUNK_POS_MASK: u128 = u128::MAX << 72;
    const SECTION_POS_MASK: u128 = u128::MAX << 44;
    const BLOCK_POS_MASK: u128 = u128::MAX << 32;

    fn from_parts(pos: BlockPos, state: BlockState) {
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
            BlockState::from_raw(self.state()).unwrap(),
        )
    }

    // fn from_block_state(pos: BlockPos, state: BlockState) -> Self {
    //     Self::new()
    //         .with_section_x(pos.x.div_euclid(16))
    //         .with_section_y(pos.y.div_euclid(16))
    //         .with_section_z(pos.z.div_euclid(16))
    //         .with_off_x(pos.x.rem_euclid(16) as u32)
    //         .with_off_y(pos.y.rem_euclid(16) as u32)
    //         .with_off_z(pos.z.rem_euclid(16) as u32)
    //         .with_state(state.to_raw() as u32)
    // }

    // fn from_index(pos: BlockPos, idx: u32) -> Self {
    //     Self::new()
    //         .with_section_x(pos.x.div_euclid(16))
    //         .with_section_y(pos.y.div_euclid(16))
    //         .with_section_z(pos.z.div_euclid(16))
    //         .with_off_x(pos.x.rem_euclid(16) as u32)
    //         .with_off_y(pos.y.rem_euclid(16) as u32)
    //         .with_off_z(pos.z.rem_euclid(16) as u32)
    //         .with_is_index(true)
    //         .with_state(idx)
    // }

    // fn to_parts(self) -> (BlockPos, BlockState) {
    //     (self.block_pos(), self.block())
    // }

    // fn block_pos(self) -> BlockPos {
    //     BlockPos {
    //         x: self.section_x() * 16 + self.off_x() as i32,
    //         y: self.section_y() * 16 + self.off_y() as i32,
    //         z: self.section_z() * 16 + self.off_z() as i32,
    //     }
    // }

    // fn chunk_pos(self) -> ChunkPos {
    //     ChunkPos {
    //         x: self.section_x(),
    //         z: self.section_z(),
    //     }
    // }

    // fn block(self) -> BlockState {
    //     BlockState::from_raw(self.state() as u16).unwrap()
    // }
}

/*
impl ChunkLayer {
    pub fn set_block(
        &mut self,
        pos: impl Into<BlockPos>,
        block: impl Into<Block>,
    ) -> Option<Block> {
        let pos = pos.into();
        let block: Block = block.into();

        let chunk = self.chunk_mut(pos)?;

        let [x, y, z] = block_offsets(pos, self.info.min_y, self.info.height as i32)?;

        self.block_updates
            .updates
            .push(BlockUpdate::from_parts(pos, block.state));

        if let (Some(data), Some(kind)) = (block.nbt, block.state.to_kind()) {
            PacketWriter::new(
                &mut self.block_updates.block_entity_buf,
                self.info.threshold,
            )
            .write_packet(&BlockEntityUpdateS2c {
                position: pos,
                kind,
                data: Cow::Borrowed(&data),
            });
        }

        Some(chunk.set_block(x, y, z, block))
    }

    pub fn flush_block_updates(&mut self) {
        // Sort in reverse so the dedup keeps the last of consecutive elements.
        self.block_updates.updates.sort_by(|a, b| b.cmp(a));

        // Eliminate redundant block assignments.
        self.block_updates
            .updates
            .dedup_by_key(|u| u.0 & BlockUpdate::BLOCK_POS_MASK);

        let sect_pos = ChunkSectionPos::new(i32::MIN, i32::MIN, i32::MIN);

        for update in self.block_updates.updates.drain(..) {
            let (pos, state) = update.to_parts();

            let new_sect_pos = ChunkSectionPos::from(pos);

            if sect_pos != new_sect_pos {
                let msg = LocalMsg::PacketAt {
                    pos: sect_pos.into(),
                };

                match self.block_updates.entry_buf.as_slice() {
                    // Zero updates. Do nothing.
                    &[] => {}
                    // One update. Send singular block update packet.
                    &[update] => self.messages.send_local_infallible(msg, |w| {
                        PacketWriter::new(w, self.info.threshold).write_packet(&BlockUpdateS2c {
                            position: BlockPos {
                                x: sect_pos.x * 16 + update.off_x() as i32,
                                y: sect_pos.y * 16 + update.off_y() as i32,
                                z: sect_pos.z * 16 + update.off_z() as i32,
                            },
                            block_id: BlockState::from_raw(update.block_state() as u16).unwrap(),
                        });
                    }),
                    // >1 updates. Send special section update packet.
                    updates => {
                        self.messages.send_local_infallible(msg, |w| {
                            PacketWriter::new(w, self.info.threshold).write_packet(
                                &ChunkDeltaUpdateS2c {
                                    chunk_section_pos: sect_pos,
                                    blocks: Cow::Borrowed(updates),
                                },
                            )
                        });
                    }
                }

                self.block_updates.entry_buf.clear();
            }
        }

        /*
        let mut chunk: Option<&mut LoadedChunk> = None;
        let mut sect_pos = ChunkSectionPos::new(i32::MIN, i32::MIN, i32::MIN);
        for update in self.block_updates.updates.drain(..) {
            let (pos, state, has_nbt) = update.to_parts();
            let new_sect_pos = ChunkSectionPos::from(pos);

            // Is this block in a new section? If it is, then flush the changes we've
            // accumulated for the old section.
            if sect_pos != new_sect_pos {
                let msg = LocalMsg::PacketAt {
                    pos: sect_pos.into(),
                };

                match self.block_updates.entry_buf.as_slice() {
                    // Zero updates. Do nothing.
                    &[] => {}
                    // One update. Send singular block update packet.
                    &[update] => self.messages.send_local_infallible(msg, |w| {
                        PacketWriter::new(w, self.info.threshold).write_packet(&BlockUpdateS2c {
                            position: BlockPos {
                                x: sect_pos.x * 16 + update.off_x() as i32,
                                y: sect_pos.y * 16 + update.off_y() as i32,
                                z: sect_pos.z * 16 + update.off_z() as i32,
                            },
                            block_id: BlockState::from_raw(update.block_state() as u16).unwrap(),
                        });
                    }),
                    // >1 updates. Send special section update packet.
                    updates => {
                        self.messages.send_local_infallible(msg, |w| {
                            PacketWriter::new(w, self.info.threshold).write_packet(
                                &ChunkDeltaUpdateS2c {
                                    chunk_section_pos: sect_pos,
                                    blocks: Cow::Borrowed(updates),
                                },
                            )
                        });
                    }
                }

                self.block_updates.entry_buf.clear();

                // Send the block entity update packets.
                // It's important that this is ordered after block state updates are sent.
                for (pos, kind, nbt) in nbt.take(nbt_count) {
                    let msg = LocalMsg::PacketAt {
                        pos: sect_pos.into(),
                    };

                    self.messages.send_local_infallible(msg, |w| {
                        PacketWriter::new(w, self.info.threshold).write_packet(
                            &BlockEntityUpdateS2c {
                                position: pos,
                                kind,
                                data: Cow::Owned(nbt),
                            },
                        )
                    });
                }

                // Update the chunk ref if the chunk pos changed.
                if sect_pos.x != new_sect_pos.x || sect_pos.z != new_sect_pos.z {
                    chunk = self.chunks.get_mut(&ChunkPos::from(new_sect_pos));
                }

                // Update section pos.
                sect_pos = new_sect_pos;
            }

            // // Send block entity updates for the current block.
            // if has_nbt {
            //     let nbt = nbt.next().unwrap();

            //     if let Some(kind) = state.to_kind() {
            //         let msg = LocalMsg::PacketAt {
            //             pos: sect_pos.into(),
            //         };

            //         self.messages.send_local_infallible(msg, |w| {
            //             PacketWriter::new(w, self.info.threshold).write_packet(
            //                 &BlockEntityUpdateS2c {
            //                     position: pos,
            //                     kind,
            //                     data: Cow::Owned(nbt),
            //                 },
            //             )
            //         });
            //     }
            // }

            if let Some(chunk) = &mut chunk {
                let chunk_y = pos.y.wrapping_sub(self.info.min_y) as u32;

                // Is the block pos in bounds of the chunk?
                if chunk_y < self.info.height {
                    let chunk_x = pos.x.rem_euclid(16);
                    let chunk_z = pos.z.rem_euclid(16);

                    // Make change to the chunk and push section update.

                    if chunk.viewer_count_mut() > 0 {
                        self.block_update_buf.push(
                            ChunkDeltaUpdateEntry::new()
                                .with_off_x(chunk_x as u8)
                                .with_off_y((chunk_y % 16) as u8)
                                .with_off_z(chunk_z as u8)
                                .with_block_state(block.state.to_raw() as u32),
                        );
                    }

                    chunk.set_block(chunk_x as u32, chunk_y, chunk_z as u32, block);
                }
            }
        }

        // Flush any remaining block changes.

        let msg = LocalMsg::PacketAt {
            pos: sect_pos.into(),
        };

        match self.block_update_buf.as_slice() {
            // Zero updates. Do nothing.
            &[] => {}
            // One update. Send singular block update packet.
            &[update] => self.messages.send_local_infallible(msg, |w| {
                PacketWriter::new(w, self.info.threshold).write_packet(&BlockUpdateS2c {
                    position: BlockPos {
                        x: sect_pos.x * 16 + update.off_x() as i32,
                        y: sect_pos.y * 16 + update.off_y() as i32,
                        z: sect_pos.z * 16 + update.off_z() as i32,
                    },
                    block_id: BlockState::from_raw(update.block_state() as u16).unwrap(),
                });
            }),
            // >1 updates. Send special section update packet.
            updates => {
                self.messages.send_local_infallible(msg, |w| {
                    PacketWriter::new(w, self.info.threshold).write_packet(&ChunkDeltaUpdateS2c {
                        chunk_section_pos: sect_pos,
                        blocks: Cow::Borrowed(updates),
                    })
                });
            }
        }

        self.block_update_buf.clear();*/
    }
}

pub(super) struct BlockUpdates {
    updates: Vec<BlockUpdate>,
    block_entities: Vec<(BlockPos, BlockEntityKind, Compound)>,
    entry_buf: Vec<ChunkDeltaUpdateEntry>,
}

impl BlockUpdates {
    pub(super) fn shrink_to_fit(&mut self) {
        self.updates.shrink_to_fit();
        self.block_entities.shrink_to_fit();
        self.entry_buf.shrink_to_fit();
    }
}
*/
