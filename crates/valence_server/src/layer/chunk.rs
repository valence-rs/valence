#[allow(clippy::module_inception)]
mod chunk;
pub mod loaded;
mod paletted_container;
pub mod unloaded;

use std::borrow::Cow;
use std::collections::hash_map::{Entry, OccupiedEntry, VacantEntry};
use std::fmt;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
pub use chunk::{MAX_HEIGHT, *};
pub use loaded::LoadedChunk;
use rustc_hash::FxHashMap;
pub use unloaded::UnloadedChunk;
use valence_math::{DVec3, Vec3};
use valence_nbt::Compound;
use valence_protocol::encode::{PacketWriter, WritePacket};
use valence_protocol::packets::play::particle_s2c::Particle;
use valence_protocol::packets::play::{ParticleS2c, PlaySoundS2c};
use valence_protocol::sound::{Sound, SoundCategory, SoundId};
use valence_protocol::{BlockPos, ChunkPos, CompressionThreshold, Encode, Ident, Packet};
use valence_registry::biome::{BiomeId, BiomeRegistry};
use valence_registry::DimensionTypeRegistry;
use valence_server_core::Server;

use super::bvh::GetChunkPos;
use super::message::Messages;
use super::{Layer, UpdateLayersPostClientSet, UpdateLayersPreClientSet};

/// A [`Component`] containing the [chunks](LoadedChunk) and [dimension
/// information](valence_registry::dimension_type::DimensionTypeId) of a
/// Minecraft world.
#[derive(Component, Debug)]
pub struct ChunkLayer {
    messages: ChunkLayerMessages,
    chunks: FxHashMap<ChunkPos, LoadedChunk>,
    info: ChunkLayerInfo,
}

/// Chunk layer information.
pub(crate) struct ChunkLayerInfo {
    dimension_type_name: Ident<String>,
    height: u32,
    min_y: i32,
    biome_registry_len: usize,
    threshold: CompressionThreshold,
}

impl fmt::Debug for ChunkLayerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChunkLayerInfo")
            .field("dimension_type_name", &self.dimension_type_name)
            .field("height", &self.height)
            .field("min_y", &self.min_y)
            .field("biome_registry_len", &self.biome_registry_len)
            .field("threshold", &self.threshold)
            // Ignore sky light mask and array.
            .finish()
    }
}

type ChunkLayerMessages = Messages<GlobalMsg, LocalMsg>;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum GlobalMsg {
    /// Send packet data to all clients viewing the layer.
    Packet,
    /// Send packet data to all clients viewing the layer, except the client
    /// identified by `except`.
    PacketExcept { except: Entity },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum LocalMsg {
    /// Send packet data to all clients viewing the layer in view of `pos`.
    PacketAt {
        pos: ChunkPos,
    },
    PacketAtExcept {
        pos: ChunkPos,
        except: Entity,
    },
    RadiusAt {
        center: BlockPos,
        radius_squared: u32,
    },
    RadiusAtExcept {
        center: BlockPos,
        radius_squared: u32,
        except: Entity,
    },
    /// Instruct clients to load or unload the chunk at `pos`. Loading and
    /// unloading are combined into a single message so that load/unload order
    /// is not lost when messages are sorted.
    ///
    /// Message content is a single byte indicating load (1) or unload (0).
    ChangeChunkState {
        pos: ChunkPos,
    },
    /// Message content is the data for a single biome in the "change biomes"
    /// packet.
    ChangeBiome {
        pos: ChunkPos,
    },
}

impl GetChunkPos for LocalMsg {
    fn chunk_pos(&self) -> ChunkPos {
        match *self {
            LocalMsg::PacketAt { pos } => pos,
            LocalMsg::PacketAtExcept { pos, .. } => pos,
            LocalMsg::RadiusAt { center, .. } => center.to_chunk_pos(),
            LocalMsg::RadiusAtExcept { center, .. } => center.to_chunk_pos(),
            LocalMsg::ChangeBiome { pos } => pos,
            LocalMsg::ChangeChunkState { pos } => pos,
        }
    }
}

impl ChunkLayer {
    pub(crate) const LOAD: u8 = 0;
    pub(crate) const UNLOAD: u8 = 1;
    pub(crate) const OVERWRITE: u8 = 2;

    /// Creates a new chunk layer.
    #[track_caller]
    pub fn new(
        dimension_type_name: impl Into<Ident<String>>,
        dimensions: &DimensionTypeRegistry,
        biomes: &BiomeRegistry,
        server: &Server,
    ) -> Self {
        let dimension_type_name = dimension_type_name.into();

        let dim = &dimensions[dimension_type_name.as_str_ident()];

        assert!(
            (0..MAX_HEIGHT as i32).contains(&dim.height),
            "invalid dimension height of {}",
            dim.height
        );

        Self {
            messages: Messages::new(),
            chunks: Default::default(),
            info: ChunkLayerInfo {
                dimension_type_name,
                height: dim.height as u32,
                min_y: dim.min_y,
                biome_registry_len: biomes.iter().len(),
                threshold: server.compression_threshold(),
            },
        }
    }

    /// The name of the dimension this chunk layer is using.
    pub fn dimension_type_name(&self) -> Ident<&str> {
        self.info.dimension_type_name.as_str_ident()
    }

    /// The height of this instance's dimension.
    pub fn height(&self) -> u32 {
        self.info.height
    }

    /// The `min_y` of this instance's dimension.
    pub fn min_y(&self) -> i32 {
        self.info.min_y
    }

    /// Get a reference to the chunk at the given position, if it is loaded.
    pub fn chunk(&self, pos: impl Into<ChunkPos>) -> Option<&LoadedChunk> {
        self.chunks.get(&pos.into())
    }

    /// Get a mutable reference to the chunk at the given position, if it is
    /// loaded.
    pub fn chunk_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut LoadedChunk> {
        self.chunks.get_mut(&pos.into())
    }

    /// Insert a chunk into the instance at the given position. The preivous
    /// chunk data is returned.
    pub fn insert_chunk(
        &mut self,
        pos: impl Into<ChunkPos>,
        chunk: UnloadedChunk,
    ) -> Option<UnloadedChunk> {
        match self.chunk_entry(pos) {
            ChunkEntry::Occupied(mut oe) => Some(oe.insert(chunk)),
            ChunkEntry::Vacant(ve) => {
                ve.insert(chunk);
                None
            }
        }
    }

    /// Unload the chunk at the given position, if it is loaded. Returns the
    /// chunk if it was loaded.
    pub fn remove_chunk(&mut self, pos: impl Into<ChunkPos>) -> Option<UnloadedChunk> {
        match self.chunk_entry(pos) {
            ChunkEntry::Occupied(oe) => Some(oe.remove()),
            ChunkEntry::Vacant(_) => None,
        }
    }

    /// Unload all chunks in this instance.
    pub fn clear_chunks(&mut self) {
        self.retain_chunks(|_, _| false)
    }

    /// Retain only the chunks for which the given predicate returns `true`.
    pub fn retain_chunks<F>(&mut self, mut f: F)
    where
        F: FnMut(ChunkPos, &mut LoadedChunk) -> bool,
    {
        self.chunks.retain(|pos, chunk| {
            if !f(*pos, chunk) {
                self.messages
                    .send_local_infallible(LocalMsg::ChangeChunkState { pos: *pos }, |b| {
                        b.push(Self::UNLOAD)
                    });

                false
            } else {
                true
            }
        });
    }

    /// Get a [`ChunkEntry`] for the given position.
    pub fn chunk_entry(&mut self, pos: impl Into<ChunkPos>) -> ChunkEntry {
        match self.chunks.entry(pos.into()) {
            Entry::Occupied(oe) => ChunkEntry::Occupied(OccupiedChunkEntry {
                messages: &mut self.messages,
                entry: oe,
            }),
            Entry::Vacant(ve) => ChunkEntry::Vacant(VacantChunkEntry {
                height: self.info.height,
                messages: &mut self.messages,
                entry: ve,
            }),
        }
    }

    /// Get an iterator over all loaded chunks in the instance. The order of the
    /// chunks is undefined.
    pub fn chunks(&self) -> impl Iterator<Item = (ChunkPos, &LoadedChunk)> + Clone + '_ {
        self.chunks.iter().map(|(pos, chunk)| (*pos, chunk))
    }

    /// Get an iterator over all loaded chunks in the instance, mutably. The
    /// order of the chunks is undefined.
    pub fn chunks_mut(&mut self) -> impl Iterator<Item = (ChunkPos, &mut LoadedChunk)> + '_ {
        self.chunks.iter_mut().map(|(pos, chunk)| (*pos, chunk))
    }

    /// Optimizes the memory usage of the instance.
    pub fn shrink_to_fit(&mut self) {
        for (_, chunk) in self.chunks_mut() {
            chunk.shrink_to_fit();
        }

        self.chunks.shrink_to_fit();
        self.messages.shrink_to_fit();
    }

    pub fn block(&self, pos: impl Into<BlockPos>) -> Option<BlockRef> {
        let (chunk, x, y, z) = self.chunk_and_offsets(pos.into())?;
        Some(chunk.block(x, y, z))
    }

    pub fn set_block(&mut self, pos: impl Into<BlockPos>, block: impl IntoBlock) -> Option<Block> {
        let (chunk, x, y, z) = self.chunk_and_offsets_mut(pos.into())?;
        Some(chunk.set_block(x, y, z, block))
    }

    pub fn block_entity_mut(&mut self, pos: impl Into<BlockPos>) -> Option<&mut Compound> {
        let (chunk, x, y, z) = self.chunk_and_offsets_mut(pos.into())?;
        chunk.block_entity_mut(x, y, z)
    }

    pub fn biome(&self, pos: impl Into<BlockPos>) -> Option<BiomeId> {
        let (chunk, x, y, z) = self.chunk_and_offsets(pos.into())?;
        Some(chunk.biome(x / 4, y / 4, z / 4))
    }

    pub fn set_biome(&mut self, pos: impl Into<BlockPos>, biome: BiomeId) -> Option<BiomeId> {
        let (chunk, x, y, z) = self.chunk_and_offsets_mut(pos.into())?;
        Some(chunk.set_biome(x / 4, y / 4, z / 4, biome))
    }

    #[inline]
    fn chunk_and_offsets(&self, pos: BlockPos) -> Option<(&LoadedChunk, u32, u32, u32)> {
        let Some(y) = pos
            .y
            .checked_sub(self.info.min_y)
            .and_then(|y| y.try_into().ok())
        else {
            return None;
        };

        if y >= self.info.height {
            return None;
        }

        let Some(chunk) = self.chunk(ChunkPos::from_block_pos(pos)) else {
            return None;
        };

        let x = pos.x.rem_euclid(16) as u32;
        let z = pos.z.rem_euclid(16) as u32;

        Some((chunk, x, y, z))
    }

    #[inline]
    fn chunk_and_offsets_mut(
        &mut self,
        pos: BlockPos,
    ) -> Option<(&mut LoadedChunk, u32, u32, u32)> {
        let Some(y) = pos
            .y
            .checked_sub(self.info.min_y)
            .and_then(|y| y.try_into().ok())
        else {
            return None;
        };

        if y >= self.info.height {
            return None;
        }

        let Some(chunk) = self.chunk_mut(ChunkPos::from_block_pos(pos)) else {
            return None;
        };

        let x = pos.x.rem_euclid(16) as u32;
        let z = pos.z.rem_euclid(16) as u32;

        Some((chunk, x, y, z))
    }

    pub(crate) fn info(&self) -> &ChunkLayerInfo {
        &self.info
    }

    pub(crate) fn messages(&self) -> &ChunkLayerMessages {
        &self.messages
    }

    // TODO: move to `valence_particle`.
    /// Puts a particle effect at the given position in the world. The particle
    /// effect is visible to all players in the instance with the
    /// appropriate chunk in view.
    pub fn play_particle(
        &mut self,
        particle: &Particle,
        long_distance: bool,
        position: impl Into<DVec3>,
        offset: impl Into<Vec3>,
        max_speed: f32,
        count: i32,
    ) {
        let position = position.into();

        self.view_writer(ChunkPos::from_pos(position))
            .write_packet(&ParticleS2c {
                particle: Cow::Borrowed(particle),
                long_distance,
                position,
                offset: offset.into(),
                max_speed,
                count,
            });
    }

    // TODO: move to `valence_sound`.
    /// Plays a sound effect at the given position in the world. The sound
    /// effect is audible to all players in the instance with the
    /// appropriate chunk in view.
    pub fn play_sound(
        &mut self,
        sound: Sound,
        category: SoundCategory,
        position: impl Into<DVec3>,
        volume: f32,
        pitch: f32,
    ) {
        let position = position.into();

        self.view_writer(ChunkPos::from_pos(position))
            .write_packet(&PlaySoundS2c {
                id: SoundId::Direct {
                    id: sound.to_ident().into(),
                    range: None,
                },
                category,
                position: (position * 8.0).as_ivec3(),
                volume,
                pitch,
                seed: rand::random(),
            });
    }
}

impl Layer for ChunkLayer {
    type ExceptWriter<'a> = ExceptWriter<'a>;

    type ViewWriter<'a> = ViewWriter<'a>;

    type ViewExceptWriter<'a> = ViewExceptWriter<'a>;

    type RadiusWriter<'a> = RadiusWriter<'a>;

    type RadiusExceptWriter<'a> = RadiusExceptWriter<'a>;

    fn except_writer(&mut self, except: Entity) -> Self::ExceptWriter<'_> {
        ExceptWriter {
            layer: self,
            except,
        }
    }

    fn view_writer(&mut self, pos: impl Into<ChunkPos>) -> Self::ViewWriter<'_> {
        ViewWriter {
            layer: self,
            pos: pos.into(),
        }
    }

    fn view_except_writer(
        &mut self,
        pos: impl Into<ChunkPos>,
        except: Entity,
    ) -> Self::ViewExceptWriter<'_> {
        ViewExceptWriter {
            layer: self,
            pos: pos.into(),
            except,
        }
    }

    fn radius_writer(
        &mut self,
        center: impl Into<BlockPos>,
        radius: u32,
    ) -> Self::RadiusWriter<'_> {
        RadiusWriter {
            layer: self,
            center: center.into(),
            radius,
        }
    }

    fn radius_except_writer(
        &mut self,
        center: impl Into<BlockPos>,
        radius: u32,
        except: Entity,
    ) -> Self::RadiusExceptWriter<'_> {
        RadiusExceptWriter {
            layer: self,
            center: center.into(),
            radius,
            except,
        }
    }
}

impl WritePacket for ChunkLayer {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.messages.send_global(GlobalMsg::Packet, |b| {
            PacketWriter::new(b, self.info.threshold).write_packet_fallible(packet)
        })
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.messages
            .send_global_infallible(GlobalMsg::Packet, |b| b.extend_from_slice(bytes));
    }
}

pub struct ExceptWriter<'a> {
    layer: &'a mut ChunkLayer,
    except: Entity,
}

impl WritePacket for ExceptWriter<'_> {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.layer.messages.send_global(
            GlobalMsg::PacketExcept {
                except: self.except,
            },
            |b| PacketWriter::new(b, self.layer.info.threshold).write_packet_fallible(packet),
        )
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.layer.messages.send_global_infallible(
            GlobalMsg::PacketExcept {
                except: self.except,
            },
            |b| b.extend_from_slice(bytes),
        )
    }
}

pub struct ViewWriter<'a> {
    layer: &'a mut ChunkLayer,
    pos: ChunkPos,
}

impl WritePacket for ViewWriter<'_> {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.layer
            .messages
            .send_local(LocalMsg::PacketAt { pos: self.pos }, |b| {
                PacketWriter::new(b, self.layer.info.threshold).write_packet_fallible(packet)
            })
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.layer
            .messages
            .send_local_infallible(LocalMsg::PacketAt { pos: self.pos }, |b| {
                b.extend_from_slice(bytes)
            });
    }
}

pub struct ViewExceptWriter<'a> {
    layer: &'a mut ChunkLayer,
    pos: ChunkPos,
    except: Entity,
}

impl WritePacket for ViewExceptWriter<'_> {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.layer.messages.send_local(
            LocalMsg::PacketAtExcept {
                pos: self.pos,
                except: self.except,
            },
            |b| PacketWriter::new(b, self.layer.info.threshold).write_packet_fallible(packet),
        )
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.layer.messages.send_local_infallible(
            LocalMsg::PacketAtExcept {
                pos: self.pos,
                except: self.except,
            },
            |b| b.extend_from_slice(bytes),
        );
    }
}

pub struct RadiusWriter<'a> {
    layer: &'a mut ChunkLayer,
    center: BlockPos,
    radius: u32,
}

impl WritePacket for RadiusWriter<'_> {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.layer.messages.send_local(
            LocalMsg::RadiusAt {
                center: self.center,
                radius_squared: self.radius,
            },
            |b| PacketWriter::new(b, self.layer.info.threshold).write_packet_fallible(packet),
        )
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.layer.messages.send_local_infallible(
            LocalMsg::RadiusAt {
                center: self.center,
                radius_squared: self.radius,
            },
            |b| b.extend_from_slice(bytes),
        );
    }
}

pub struct RadiusExceptWriter<'a> {
    layer: &'a mut ChunkLayer,
    center: BlockPos,
    radius: u32,
    except: Entity,
}

impl WritePacket for RadiusExceptWriter<'_> {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.layer.messages.send_local(
            LocalMsg::RadiusAtExcept {
                center: self.center,
                radius_squared: self.radius,
                except: self.except,
            },
            |b| PacketWriter::new(b, self.layer.info.threshold).write_packet_fallible(packet),
        )
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.layer.messages.send_local_infallible(
            LocalMsg::RadiusAtExcept {
                center: self.center,
                radius_squared: self.radius,
                except: self.except,
            },
            |b| b.extend_from_slice(bytes),
        );
    }
}

#[derive(Debug)]
pub enum ChunkEntry<'a> {
    Occupied(OccupiedChunkEntry<'a>),
    Vacant(VacantChunkEntry<'a>),
}

impl<'a> ChunkEntry<'a> {
    pub fn or_default(self) -> &'a mut LoadedChunk {
        match self {
            ChunkEntry::Occupied(oe) => oe.into_mut(),
            ChunkEntry::Vacant(ve) => ve.insert(UnloadedChunk::new()),
        }
    }
}

#[derive(Debug)]
pub struct OccupiedChunkEntry<'a> {
    messages: &'a mut ChunkLayerMessages,
    entry: OccupiedEntry<'a, ChunkPos, LoadedChunk>,
}

impl<'a> OccupiedChunkEntry<'a> {
    pub fn get(&self) -> &LoadedChunk {
        self.entry.get()
    }

    pub fn get_mut(&mut self) -> &mut LoadedChunk {
        self.entry.get_mut()
    }

    pub fn insert(&mut self, chunk: UnloadedChunk) -> UnloadedChunk {
        self.messages.send_local_infallible(
            LocalMsg::ChangeChunkState {
                pos: *self.entry.key(),
            },
            |b| b.push(ChunkLayer::OVERWRITE),
        );

        self.entry.get_mut().insert(chunk)
    }

    pub fn into_mut(self) -> &'a mut LoadedChunk {
        self.entry.into_mut()
    }

    pub fn key(&self) -> &ChunkPos {
        self.entry.key()
    }

    pub fn remove(self) -> UnloadedChunk {
        self.messages.send_local_infallible(
            LocalMsg::ChangeChunkState {
                pos: *self.entry.key(),
            },
            |b| b.push(ChunkLayer::UNLOAD),
        );

        self.entry.remove().remove()
    }

    pub fn remove_entry(mut self) -> (ChunkPos, UnloadedChunk) {
        let pos = *self.entry.key();
        let chunk = self.entry.get_mut().remove();

        self.messages.send_local_infallible(
            LocalMsg::ChangeChunkState {
                pos: *self.entry.key(),
            },
            |b| b.push(ChunkLayer::UNLOAD),
        );

        (pos, chunk)
    }
}

#[derive(Debug)]
pub struct VacantChunkEntry<'a> {
    height: u32,
    messages: &'a mut ChunkLayerMessages,
    entry: VacantEntry<'a, ChunkPos, LoadedChunk>,
}

impl<'a> VacantChunkEntry<'a> {
    pub fn insert(self, chunk: UnloadedChunk) -> &'a mut LoadedChunk {
        let mut loaded = LoadedChunk::new(self.height);
        loaded.insert(chunk);

        self.messages.send_local_infallible(
            LocalMsg::ChangeChunkState {
                pos: *self.entry.key(),
            },
            |b| b.push(ChunkLayer::LOAD),
        );

        self.entry.insert(loaded)
    }

    pub fn into_key(self) -> ChunkPos {
        *self.entry.key()
    }

    pub fn key(&self) -> &ChunkPos {
        self.entry.key()
    }
}

pub(super) fn build(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            update_chunk_layers_pre_client.in_set(UpdateLayersPreClientSet),
            update_chunk_layers_post_client.in_set(UpdateLayersPostClientSet),
        ),
    );
}

fn update_chunk_layers_pre_client(mut layers: Query<&mut ChunkLayer>) {
    for layer in &mut layers {
        let layer = layer.into_inner();

        for (&pos, chunk) in &mut layer.chunks {
            chunk.update_pre_client(pos, &layer.info, &mut layer.messages);
        }

        layer.messages.ready();
    }
}

fn update_chunk_layers_post_client(mut layers: Query<&mut ChunkLayer>) {
    for mut layer in &mut layers {
        layer.messages.unready();
    }
}
