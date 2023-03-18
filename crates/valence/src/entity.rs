use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fmt::Formatter;
use std::num::Wrapping;
use std::ops::Range;

use bevy_app::{App, CoreSet, Plugin};
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use glam::{DVec3, UVec3, Vec3};
use rand::rngs::{OsRng, StdRng};
use rand::{Rng, SeedableRng};
use rustc_hash::FxHashMap;
use tracing::{debug, warn};
use uuid::Uuid;
use valence_protocol::byte_angle::ByteAngle;
use valence_protocol::packet::s2c::play::{
    EntityAnimationS2c, EntityPositionS2c, EntitySetHeadYawS2c, EntitySpawnS2c,
    EntityStatusS2c as EntityEventS2c, EntityTrackerUpdateS2c, EntityVelocityUpdateS2c,
    ExperienceOrbSpawnS2c, MoveRelativeS2c, PlayerSpawnS2c, RotateAndMoveRelativeS2c, RotateS2c,
};
pub use valence_protocol::types::Direction;
use valence_protocol::var_int::VarInt;
use valence_protocol::{Decode, Encode};

use self::data::TrackedData;
use crate::client::FlushPacketsSet;
use crate::component::{
    Despawned, Location, Look, OldLocation, OldPosition, OnGround, Position, UniqueId,
};
use crate::config::DEFAULT_TPS;
use crate::packet::WritePacket;
use crate::util::{velocity_to_packet_units, Aabb};

pub mod data;

include!(concat!(env!("OUT_DIR"), "/entity_event.rs"));
include!(concat!(env!("OUT_DIR"), "/entity.rs"));

/// A Minecraft entity's ID according to the protocol.
///
/// IDs should be unique between all spawned entities and should not be modified
/// after creation. IDs of zero (the default) will be assigned to something else
/// after the entity is added. If you need to know the ID ahead of time, set
/// this component to the value returned by [`EntityManager::next_id`] before
/// spawning.
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct EntityId(i32);

impl EntityId {
    pub fn get(&self) -> i32 {
        self.0
    }
}

#[derive(Component, Default)]
pub struct HeadYaw(pub f32);

/// Entity velocity in m/s.
#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct Velocity(pub Vec3);

#[derive(Component, Default)]
pub struct EntityStatuses(u64);

impl EntityStatuses {
    pub fn trigger(&mut self, status: EntityStatus) {
        self.set(status, true);
    }

    pub fn set(&mut self, status: EntityStatus, triggered: bool) {
        self.0 |= (triggered as u64) << status as u64;
    }

    pub fn get(&self, status: EntityStatus) -> bool {
        (self.0 >> status as u64) & 1 == 1
    }
}

#[derive(Component, Default)]
pub struct EntityAnimations(u8);

impl EntityAnimations {
    pub fn trigger(&mut self, status: EntityAnimation) {
        self.set(status, true);
    }

    pub fn set(&mut self, anim: EntityAnimation, triggered: bool) {
        self.0 |= (triggered as u8) << anim as u8;
    }

    pub fn get(&self, anim: EntityAnimation) -> bool {
        (self.0 >> anim as u8) & 1 == 1
    }
}

/// Extra integer data passed to the entity spawn packet. The meaning depends on
/// the type of entity being spawned.
///
/// Some examples:
/// - **Experience Orb**: Experience count
/// - **(Glowing) Item Frame**: Rotation
/// - **Painting**: Rotation
/// - **Falling Block**: Block state
/// - **Fishing Bobber**: Hook entity ID
/// - **Warden**: Initial pose
#[derive(Component, Default)]
pub struct ObjectData(pub i32);

/// The range of packet bytes for this entity within the cell the entity is
/// located in. For internal use only.
#[derive(Component, Default)]
pub struct PacketByteRange {
    pub(crate) begin: usize,
    pub(crate) end: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Encode, Decode)]
pub struct VillagerData {
    pub kind: VillagerKind,
    pub profession: VillagerProfession,
    pub level: i32,
}

impl VillagerData {
    pub const fn new(kind: VillagerKind, profession: VillagerProfession, level: i32) -> Self {
        Self {
            kind,
            profession,
            level,
        }
    }
}

impl Default for VillagerData {
    fn default() -> Self {
        Self {
            kind: Default::default(),
            profession: Default::default(),
            level: 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum VillagerKind {
    Desert,
    Jungle,
    #[default]
    Plains,
    Savanna,
    Snow,
    Swamp,
    Taiga,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum VillagerProfession {
    #[default]
    None,
    Armorer,
    Butcher,
    Cartographer,
    Cleric,
    Farmer,
    Fisherman,
    Fletcher,
    Leatherworker,
    Librarian,
    Mason,
    Nitwit,
    Shepherd,
    Toolsmith,
    Weaponsmith,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum Pose {
    #[default]
    Standing,
    FallFlying,
    Sleeping,
    Swimming,
    SpinAttack,
    Sneaking,
    LongJumping,
    Dying,
    Croaking,
    UsingTongue,
    Roaring,
    Sniffing,
    Emerging,
    Digging,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum BoatKind {
    #[default]
    Oak,
    Spruce,
    Birch,
    Jungle,
    Acacia,
    DarkOak,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum CatKind {
    Tabby,
    #[default]
    Black,
    Red,
    Siamese,
    BritishShorthair,
    Calico,
    Persian,
    Ragdoll,
    White,
    Jellie,
    AllBlack,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum FrogKind {
    #[default]
    Temperate,
    Warm,
    Cold,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum PaintingKind {
    #[default]
    Kebab,
    Aztec,
    Alban,
    Aztec2,
    Bomb,
    Plant,
    Wasteland,
    Pool,
    Courbet,
    Sea,
    Sunset,
    Creebet,
    Wanderer,
    Graham,
    Match,
    Bust,
    Stage,
    Void,
    SkullAndRoses,
    Wither,
    Fighters,
    Pointer,
    Pigscene,
    BurningSkull,
    Skeleton,
    Earth,
    Wind,
    Water,
    Fire,
    DonkeyKong,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Encode, Decode)]
pub struct EulerAngle {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

#[derive(Copy, Clone)]
pub struct OptionalInt(pub Option<i32>);

impl Encode for OptionalInt {
    fn encode(&self, w: impl std::io::Write) -> anyhow::Result<()> {
        if let Some(n) = self.0 {
            VarInt(n.wrapping_add(1))
        } else {
            VarInt(0)
        }
        .encode(w)
    }
}

impl Decode<'_> for OptionalInt {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let n = VarInt::decode(r)?.0;
        
        Ok(Self(if n == 0 {
            None
        } else {
            Some(n.wrapping_sub(1))
        }))
    }
}

/// Maintains information about all spawned Minecraft entities.
#[derive(Resource)]
pub struct EntityManager {
    /// Maps protocol IDs to ECS entities.
    id_to_entity: FxHashMap<i32, Entity>,
    uuid_to_entity: FxHashMap<Uuid, Entity>,
    next_id: Wrapping<i32>,
    uuid_rng: StdRng,
}

impl EntityManager {
    fn new() -> Self {
        Self {
            id_to_entity: FxHashMap::default(),
            uuid_to_entity: FxHashMap::default(),
            next_id: Wrapping(1), // Skip 0.
            uuid_rng: StdRng::from_rng(OsRng).unwrap(),
        }
    }

    /// Returns the next unique entity ID and increments the counter.
    pub fn next_id(&mut self) -> EntityId {
        if self.next_id.0 == 0 {
            warn!("entity ID overflow!");
            // ID 0 is reserved for clients, so skip over it.
            self.next_id.0 = 1;
        }

        let id = EntityId(self.next_id.0);

        self.next_id += 1;

        id
    }

    /// Gets the entity with the given entity ID.
    pub fn get_with_id(&self, entity_id: i32) -> Option<Entity> {
        self.id_to_entity.get(&entity_id).cloned()
    }

    /// Gets the entity with the given UUID.
    pub fn get_with_uuid(&self, uuid: Uuid) -> Option<Entity> {
        self.uuid_to_entity.get(&uuid).cloned()
    }
}

pub(crate) struct EntityPlugin;

/// When new Minecraft entities are initialized and added to
/// [`McEntityManager`]. Systems that need all Minecraft entities to be in a
/// valid state should run after this.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct InitEntitiesSet;

impl Plugin for EntityPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EntityManager::new())
            .configure_set(InitEntitiesSet.in_base_set(CoreSet::PostUpdate))
            .add_system(init_entities.in_set(InitEntitiesSet))
            .add_system(
                remove_despawned_from_manager
                    .in_base_set(CoreSet::PostUpdate)
                    .after(init_entities),
            );
    }
}

fn init_entities(
    mut entities: Query<(Entity, &mut EntityId, &mut UniqueId), Added<BaseEntity>>,
    mut manager: ResMut<EntityManager>,
) {
    for (entity, mut id, mut uuid) in &mut entities {
        if id.0 == 0 {
            *id = manager.next_id();
        }

        if uuid.0.is_nil() {
            uuid.0 = Uuid::from_bytes(manager.uuid_rng.gen());
        }

        if let Some(conflict) = manager.id_to_entity.insert(id.0, entity) {
            warn!(
                "entity {entity:?} has conflicting entity ID of {} with entity {conflict:?}",
                id.0
            );
        }

        if let Some(conflict) = manager.uuid_to_entity.insert(uuid.0, entity) {
            warn!(
                "entity {entity:?} has conflicting UUID of {} with entity {conflict:?}",
                uuid.0
            );
        }
    }
}

fn remove_despawned_from_manager(
    entities: Query<(Entity, &EntityId, &UniqueId), (With<BaseEntity>, With<Despawned>)>,
    mut manager: ResMut<EntityManager>,
) {
    for (entity, id, uuid) in &entities {
        manager.id_to_entity.remove(&id.0);
        manager.uuid_to_entity.remove(&uuid.0);
    }
}

/*
/// Sets the protocol ID of new mcentities and adds them to the
/// [`McEntityManager`].
fn init_mcentities(
    mut entities: Query<(Entity, &mut McEntity), Added<McEntity>>,
    mut manager: ResMut<McEntityManager>,
) {
    for (entity, mut mc_entity) in &mut entities {
        if manager.next_protocol_id == 0 {
            warn!("entity protocol ID overflow");
            // ID 0 is reserved for clients so we skip over it.
            manager.next_protocol_id = 1;
        }

        mc_entity.protocol_id = manager.next_protocol_id;
        manager.next_protocol_id = manager.next_protocol_id.wrapping_add(1);

        manager
            .protocol_id_to_mcentity
            .insert(mc_entity.protocol_id, entity);
    }
}

/// Removes despawned mcentities from the mcentity manager.
fn remove_despawned_from_manager(
    entities: Query<&mut McEntity, With<Despawned>>,
    mut manager: ResMut<McEntityManager>,
) {
    for entity in &entities {
        manager.protocol_id_to_mcentity.remove(&entity.protocol_id);
    }
}

fn update_mcentities(mut mcentities: Query<&mut McEntity, Changed<McEntity>>) {
    for mut ent in &mut mcentities {
        ent.data.clear_modifications();
        ent.old_position = ent.position;
        ent.old_instance = ent.instance;
        ent.statuses = 0;
        ent.animations = 0;
        ent.yaw_or_pitch_modified = false;
        ent.head_yaw_modified = false;
        ent.velocity_modified = false;
    }
}

/// A [`Resource`] which maintains information about all the [`McEntity`]
/// components on the server.
#[derive(Resource, Debug)]
pub struct McEntityManager {
    protocol_id_to_mcentity: FxHashMap<i32, Entity>,
    next_protocol_id: i32,
}

impl McEntityManager {
    fn new() -> Self {
        Self {
            protocol_id_to_mcentity: HashMap::default(),
            next_protocol_id: 1,
        }
    }

    /// Gets the [`Entity`] of the [`McEntity`] with the given protocol ID.
    pub fn get_with_protocol_id(&self, id: i32) -> Option<Entity> {
        self.protocol_id_to_mcentity.get(&id).cloned()
    }
}

/// A component for Minecraft entities. For Valence to recognize a
/// Minecraft entity, it must have this component attached.
///
/// ECS entities with this component are not allowed to be removed from the
/// [`World`] directly. Instead, you must mark these entities with [`Despawned`]
/// to allow deinitialization to occur.
///
/// Every entity has common state which is accessible directly from this struct.
/// This includes position, rotation, velocity, and UUID. To access data that is
/// not common to every kind of entity, see [`Self::data`].
#[derive(Component)]
pub struct McEntity {
    pub(crate) data: TrackedData,
    protocol_id: i32,
    uuid: Uuid,
    /// The range of bytes in the partition cell containing this entity's update
    /// packets.
    pub(crate) self_update_range: Range<usize>,
    /// Contains a set bit for every status triggered this tick.
    statuses: u64,
    /// Contains a set bit for every animation triggered this tick.
    animations: u8,
    instance: Entity,
    old_instance: Entity,
    position: DVec3,
    old_position: DVec3,
    yaw: f32,
    pitch: f32,
    yaw_or_pitch_modified: bool,
    head_yaw: f32,
    head_yaw_modified: bool,
    velocity: Vec3,
    velocity_modified: bool,
    on_ground: bool,
}

    /// Sends the appropriate packets to initialize the entity. This will spawn
    /// the entity and initialize tracked data.
    pub(crate) fn write_init_packets(
        &self,
        mut writer: impl WritePacket,
        position: DVec3,
        scratch: &mut Vec<u8>,
    ) {
        let with_object_data = |data| EntitySpawnS2c {
            entity_id: VarInt(self.protocol_id),
            object_uuid: self.uuid,
            kind: VarInt(self.kind() as i32),
            position: position.to_array(),
            pitch: ByteAngle::from_degrees(self.pitch),
            yaw: ByteAngle::from_degrees(self.yaw),
            head_yaw: ByteAngle::from_degrees(self.head_yaw),
            data: VarInt(data),
            velocity: velocity_to_packet_units(self.velocity),
        };

        match &self.data {
            TrackedData::Marker(_) => {}
            TrackedData::ExperienceOrb(_) => writer.write_packet(&ExperienceOrbSpawnS2c {
                entity_id: VarInt(self.protocol_id),
                position: position.to_array(),
                count: 0, // TODO
            }),
            TrackedData::Player(_) => {
                writer.write_packet(&PlayerSpawnS2c {
                    entity_id: VarInt(self.protocol_id),
                    player_uuid: self.uuid,
                    position: position.to_array(),
                    yaw: ByteAngle::from_degrees(self.yaw),
                    pitch: ByteAngle::from_degrees(self.pitch),
                });

                // Player spawn packet doesn't include head yaw for some reason.
                writer.write_packet(&EntitySetHeadYawS2c {
                    entity_id: VarInt(self.protocol_id),
                    head_yaw: ByteAngle::from_degrees(self.head_yaw),
                });
            }
            TrackedData::ItemFrame(e) => writer.write_packet(&with_object_data(e.get_rotation())),
            TrackedData::GlowItemFrame(e) => {
                writer.write_packet(&with_object_data(e.get_rotation()))
            }

            TrackedData::Painting(_) => writer.write_packet(&with_object_data(
                match ((self.yaw + 45.0).rem_euclid(360.0) / 90.0) as u8 {
                    0 => 3,
                    1 => 4,
                    2 => 2,
                    _ => 5,
                },
            )),
            // TODO: set block state ID for falling block.
            TrackedData::FallingBlock(_) => writer.write_packet(&with_object_data(1)),
            TrackedData::FishingBobber(e) => {
                writer.write_packet(&with_object_data(e.get_hook_entity_id()))
            }
            TrackedData::Warden(e) => {
                writer.write_packet(&with_object_data((e.get_pose() == Pose::Emerging).into()))
            }
            _ => writer.write_packet(&with_object_data(0)),
        }

        scratch.clear();
        self.data.write_initial_tracked_data(scratch);
        if !scratch.is_empty() {
            writer.write_packet(&EntityTrackerUpdateS2c {
                entity_id: VarInt(self.protocol_id),
                metadata: scratch.as_slice().into(),
            });
        }
    }

    /// Writes the appropriate packets to update the entity (Position, tracked
    /// data, events, animations).
    pub(crate) fn write_update_packets(&self, mut writer: impl WritePacket, scratch: &mut Vec<u8>) {
        let entity_id = VarInt(self.protocol_id);

        let position_delta = self.position - self.old_position;
        let needs_teleport = position_delta.abs().max_element() >= 8.0;
        let changed_position = self.position != self.old_position;

        if changed_position && !needs_teleport && self.yaw_or_pitch_modified {
            writer.write_packet(&RotateAndMoveRelativeS2c {
                entity_id,
                delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                yaw: ByteAngle::from_degrees(self.yaw),
                pitch: ByteAngle::from_degrees(self.pitch),
                on_ground: self.on_ground,
            });
        } else {
            if changed_position && !needs_teleport {
                writer.write_packet(&MoveRelativeS2c {
                    entity_id,
                    delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                    on_ground: self.on_ground,
                });
            }

            if self.yaw_or_pitch_modified {
                writer.write_packet(&RotateS2c {
                    entity_id,
                    yaw: ByteAngle::from_degrees(self.yaw),
                    pitch: ByteAngle::from_degrees(self.pitch),
                    on_ground: self.on_ground,
                });
            }
        }

        if needs_teleport {
            writer.write_packet(&EntityPositionS2c {
                entity_id,
                position: self.position.to_array(),
                yaw: ByteAngle::from_degrees(self.yaw),
                pitch: ByteAngle::from_degrees(self.pitch),
                on_ground: self.on_ground,
            });
        }

        if self.velocity_modified {
            writer.write_packet(&EntityVelocityUpdateS2c {
                entity_id,
                velocity: velocity_to_packet_units(self.velocity),
            });
        }

        if self.head_yaw_modified {
            writer.write_packet(&EntitySetHeadYawS2c {
                entity_id,
                head_yaw: ByteAngle::from_degrees(self.head_yaw),
            });
        }

        scratch.clear();
        self.data.write_updated_tracked_data(scratch);
        if !scratch.is_empty() {
            writer.write_packet(&EntityTrackerUpdateS2c {
                entity_id,
                metadata: scratch.as_slice().into(),
            });
        }

        if self.statuses != 0 {
            for i in 0..std::mem::size_of_val(&self.statuses) {
                if (self.statuses >> i) & 1 == 1 {
                    writer.write_packet(&EntityEventS2c {
                        entity_id: entity_id.0,
                        entity_status: i as u8,
                    });
                }
            }
        }

        if self.animations != 0 {
            for i in 0..std::mem::size_of_val(&self.animations) {
                if (self.animations >> i) & 1 == 1 {
                    writer.write_packet(&EntityAnimationS2c {
                        entity_id,
                        animation: i as u8,
                    });
                }
            }
        }
    }
}

impl fmt::Debug for McEntity {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("McEntity")
            .field("kind", &self.kind())
            .field("protocol_id", &self.protocol_id)
            .field("uuid", &self.uuid)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}
*/
