//! Handles getting and setting blocks in chunk layers.

mod basic;

use std::borrow::Cow;

use valence_protocol::encode::PacketWriter;
use valence_protocol::packets::play::chunk_delta_update_s2c::ChunkDeltaUpdateEntry;
use valence_protocol::packets::play::{BlockEntityUpdateS2c, BlockUpdateS2c, ChunkDeltaUpdateS2c};
use valence_protocol::{BlockPos, ChunkPos, ChunkSectionPos, WritePacket};

use super::{block_offsets, Block, BlockRef, ChunkOps, LoadedChunk};
use crate::layer_old::chunk::LocalMsg;
use crate::{BlockState, ChunkLayer, Layer};

impl ChunkLayer {
    pub fn block(&self, pos: impl Into<BlockPos>) -> Option<BlockRef> {
        let pos = pos.into();
        let chunk_pos = ChunkPos::from(pos);

        let chunk = self.chunk(chunk_pos)?;
        let [x, y, z] = block_offsets(pos, self.info.min_y, self.info.height as i32)?;

        Some(chunk.block(x, y, z))
    }

    pub fn set_block(
        &mut self,
        pos: impl Into<BlockPos>,
        block: impl Into<Block>,
    ) -> Option<Block> {
        let pos = pos.into();
        let chunk_pos = ChunkPos::from(pos);
        let block: Block = block.into();

        let [x, y, z] = block_offsets(pos, self.info.min_y, self.info.height as i32)?;

        let mut writer = self.view_writer(chunk_pos);

        writer.write_packet(&BlockUpdateS2c {
            position: pos,
            block_id: block.state,
        });

        if let (Some(nbt), Some(kind)) = (&block.nbt, block.state.block_entity_kind()) {
            writer.write_packet(&BlockEntityUpdateS2c {
                position: pos,
                kind,
                data: Cow::Borrowed(nbt),
            });
        }

        let chunk = self.chunk_mut(chunk_pos)?;

        Some(chunk.set_block(x, y, z, block))
    }

    pub fn apply_batch(&mut self, batch: impl Batch) {
        let block_iter = batch.into_batch_iters();

        let mut chunk: Option<&mut LoadedChunk> = None;
        let mut sect_pos = ChunkSectionPos::new(i32::MIN, i32::MIN, i32::MIN);

        for (pos, block) in block_iter {
            let new_sect_pos = ChunkSectionPos::from(pos);

            // Is this block in a new section? If it is, then flush the changes we've
            // accumulated for the old section.
            if sect_pos != new_sect_pos {
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
                            PacketWriter::new(w, self.info.threshold).write_packet(
                                &ChunkDeltaUpdateS2c {
                                    chunk_section_pos: sect_pos,
                                    blocks: Cow::Borrowed(updates),
                                },
                            )
                        });
                    }
                }

                // Send block entity update.
                if let (Some(nbt), Some(kind)) = (&block.nbt, block.state.block_entity_kind()) {
                    self.messages.send_local_infallible(msg, |w| {
                        PacketWriter::new(w, self.info.threshold).write_packet(
                            &BlockEntityUpdateS2c {
                                position: pos,
                                kind,
                                data: Cow::Borrowed(nbt),
                            },
                        )
                    });
                }

                self.block_update_buf.clear();

                // Update the chunk ref if the chunk pos changed.
                if sect_pos.x != new_sect_pos.x || sect_pos.z != new_sect_pos.z {
                    chunk = self.chunks.get_mut(&ChunkPos::from(new_sect_pos));
                }

                // Update section pos.
                sect_pos = new_sect_pos;
            }

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

        self.block_update_buf.clear();
    }
}

