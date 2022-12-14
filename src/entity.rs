//! Entities in a world.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::iter::FusedIterator;
use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut, Index, IndexMut, Range};

use bitfield_struct::bitfield;
pub use data::{EntityKind, TrackedData};
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use valence_protocol::entity_meta::{Facing, PaintingKind, Pose};
use valence_protocol::packets::s2c::play::{
    EntityAnimationS2c, EntityEvent as EntityEventPacket, SetEntityMetadata, SetEntityVelocity,
    SetHeadRotation, SpawnEntity, SpawnExperienceOrb, SpawnPlayer, TeleportEntity,
    UpdateEntityPosition, UpdateEntityPositionAndRotation, UpdateEntityRotation,
};
use valence_protocol::{ByteAngle, RawBytes, VarInt};
use vek::{Aabb, Vec3};

use crate::config::Config;
use crate::packet::WritePacket;
use crate::server::PlayPacketSender;
use crate::slab_versioned::{Key, VersionedSlab};
use crate::util::aabb_from_bottom_and_size;
use crate::world::WorldId;
use crate::STANDARD_TPS;

pub mod data;

include!(concat!(env!("OUT_DIR"), "/entity_event.rs"));

/// A container for all [`Entity`]s on a server.
///
/// # Spawning Player Entities
///
/// [`Player`] entities are treated specially by the client. For the player
/// entity to be visible to clients, the player's UUID must be added to the
/// [`PlayerList`] _before_ being loaded by the client.
///
/// [`Player`]: crate::entity::data::Player
/// [`PlayerList`]: crate::player_list::PlayerList
pub struct Entities<C: Config> {
    slab: VersionedSlab<Entity<C>>,
    uuid_to_entity: HashMap<Uuid, EntityId>,
    raw_id_to_entity: HashMap<NonZeroU32, u32>,
}

impl<C: Config> Entities<C> {
    pub(crate) fn new() -> Self {
        Self {
            slab: VersionedSlab::new(),
            uuid_to_entity: HashMap::new(),
            raw_id_to_entity: HashMap::new(),
        }
    }

    /// Spawns a new entity with a random UUID. A reference to the entity along
    /// with its ID is returned.
    pub fn insert(
        &mut self,
        kind: EntityKind,
        state: C::EntityState,
    ) -> (EntityId, &mut Entity<C>) {
        self.insert_with_uuid(kind, Uuid::from_bytes(rand::random()), state)
            .expect("UUID collision")
    }

    /// Like [`Self::insert`], but requires specifying the new
    /// entity's UUID.
    ///
    /// The provided UUID must not conflict with an existing entity UUID. If it
    /// does, `None` is returned and the entity is not spawned.
    pub fn insert_with_uuid(
        &mut self,
        kind: EntityKind,
        uuid: Uuid,
        state: C::EntityState,
    ) -> Option<(EntityId, &mut Entity<C>)> {
        match self.uuid_to_entity.entry(uuid) {
            Entry::Occupied(_) => None,
            Entry::Vacant(ve) => {
                let (k, e) = self.slab.insert(Entity {
                    state,
                    variants: TrackedData::new(kind),
                    self_update_range: 0..0,
                    events: vec![],
                    bits: EntityBits::new(),
                    world: WorldId::NULL,
                    old_world: WorldId::NULL,
                    position: Vec3::default(),
                    old_position: Vec3::default(),
                    yaw: 0.0,
                    pitch: 0.0,
                    head_yaw: 0.0,
                    velocity: Vec3::default(),
                    uuid,
                });

                // TODO check for overflowing version?
                self.raw_id_to_entity.insert(k.version(), k.index());

                ve.insert(EntityId(k));

                Some((EntityId(k), e))
            }
        }
    }

    /// Returns the number of entities in this container.
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    /// Returns `true` if there are no entities.
    pub fn is_empty(&self) -> bool {
        self.slab.len() == 0
    }

    /// Gets the [`EntityId`] of the entity with the given UUID in an efficient
    /// manner. The returned ID is guaranteed to be valid.
    ///
    /// If there is no entity with the UUID, `None` is returned.
    pub fn get_with_uuid(&self, uuid: Uuid) -> Option<EntityId> {
        self.uuid_to_entity.get(&uuid).cloned()
    }

    /// Gets a shared reference to the entity with the given [`EntityId`].
    ///
    /// If the ID is invalid, `None` is returned.
    pub fn get(&self, entity: EntityId) -> Option<&Entity<C>> {
        self.slab.get(entity.0)
    }

    /// Gets an exclusive reference to the entity with the given [`EntityId`].
    ///
    /// If the ID is invalid, `None` is returned.
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut Entity<C>> {
        self.slab.get_mut(entity.0)
    }

    pub fn delete(&mut self, entity: EntityId) -> bool {
        if let Some(entity) = self.get_mut(entity) {
            entity.set_deleted(true);
            true
        } else {
            false
        }
    }

    pub fn get_with_raw_id(&self, raw_id: i32) -> Option<(EntityId, &Entity<C>)> {
        let version = NonZeroU32::new(raw_id as u32)?;
        let index = *self.raw_id_to_entity.get(&version)?;

        let id = EntityId(Key::new(index, version));
        let entity = self.get(id)?;
        Some((id, entity))
    }

    pub fn get_with_raw_id_mut(&mut self, raw_id: i32) -> Option<(EntityId, &mut Entity<C>)> {
        let version = NonZeroU32::new(raw_id as u32)?;
        let index = *self.raw_id_to_entity.get(&version)?;

        let id = EntityId(Key::new(index, version));
        let entity = self.get_mut(id)?;
        Some((id, entity))
    }

    /// Returns an iterator over all entities on the server in an unspecified
    /// order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (EntityId, &Entity<C>)> + FusedIterator + Clone + '_ {
        self.slab.iter().map(|(k, v)| (EntityId(k), v))
    }

    /// Returns a mutable iterator over all entities on the server in an
    /// unspecified order.
    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (EntityId, &mut Entity<C>)> + FusedIterator + '_ {
        self.slab.iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    /// Returns a parallel iterator over all entities on the server in an
    /// unspecified order.
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (EntityId, &Entity<C>)> + Clone + '_ {
        self.slab.par_iter().map(|(k, v)| (EntityId(k), v))
    }

    /// Returns a parallel mutable iterator over all clients on the server in an
    /// unspecified order.
    pub fn par_iter_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = (EntityId, &mut Entity<C>)> + '_ {
        self.slab.par_iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    pub(crate) fn update(&mut self) {
        self.slab.retain(|k, entity| {
            if entity.deleted() {
                self.uuid_to_entity
                    .remove(&entity.uuid)
                    .expect("UUID should have been in UUID map");

                self.raw_id_to_entity
                    .remove(&k.version())
                    .expect("raw ID should have been in the raw ID map");

                false
            } else {
                entity.old_position = entity.position;
                entity.old_world = entity.world;
                entity.variants.clear_modifications();
                entity.events.clear();

                entity.bits.set_yaw_or_pitch_modified(false);
                entity.bits.set_head_yaw_modified(false);
                entity.bits.set_velocity_modified(false);

                true
            }
        });
    }
}

impl<C: Config> Index<EntityId> for Entities<C> {
    type Output = Entity<C>;

    fn index(&self, index: EntityId) -> &Self::Output {
        self.get(index).expect("invalid entity ID")
    }
}

impl<C: Config> IndexMut<EntityId> for Entities<C> {
    fn index_mut(&mut self, index: EntityId) -> &mut Self::Output {
        self.get_mut(index).expect("invalid entity ID")
    }
}

/// An identifier for an [`Entity`] on the server.
///
/// Entity IDs are either _valid_ or _invalid_. Valid entity IDs point to
/// entities that have not been deleted, while invalid IDs point to those that
/// have. Once an ID becomes invalid, it will never become valid again.
///
/// The [`Ord`] instance on this type is correct but otherwise unspecified. This
/// is useful for storing IDs in containers such as
/// [`BTreeMap`](std::collections::BTreeMap).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct EntityId(Key);

impl EntityId {
    /// The value of the default entity ID which is always invalid.
    pub const NULL: Self = Self(Key::NULL);

    pub fn to_raw(self) -> i32 {
        self.0.version().get() as i32
    }
}

/// Represents an entity on the server.
///
/// An entity is mostly anything in a world that isn't a block or client.
/// Entities include paintings, falling blocks, zombies, fireballs, and more.
///
/// Every entity has common state which is accessible directly from
/// this struct. This includes position, rotation, velocity, UUID, and hitbox.
/// To access data that is not common to every kind of entity, see
/// [`Self::data`].
pub struct Entity<C: Config> {
    /// Custom data.
    pub state: C::EntityState,
    variants: TrackedData,
    bits: EntityBits,
    /// The range of bytes in the partition cell containing this entity's update
    /// packets.
    pub(crate) self_update_range: Range<usize>,
    events: Vec<EntityEvent>, // TODO: store this info in bits?
    world: WorldId,
    old_world: WorldId,
    position: Vec3<f64>,
    old_position: Vec3<f64>,
    yaw: f32,
    pitch: f32,
    head_yaw: f32,
    velocity: Vec3<f32>,
    uuid: Uuid,
}

#[bitfield(u8)]
pub(crate) struct EntityBits {
    pub yaw_or_pitch_modified: bool,
    pub head_yaw_modified: bool,
    pub velocity_modified: bool,
    pub on_ground: bool,
    pub deleted: bool,
    #[bits(3)]
    _pad: u8,
}

impl<C: Config> Deref for Entity<C> {
    type Target = C::EntityState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<C: Config> DerefMut for Entity<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<C: Config> Entity<C> {
    /// Returns a shared reference to this entity's tracked data.
    pub fn data(&self) -> &TrackedData {
        &self.variants
    }

    /// Returns an exclusive reference to this entity's tracked data.
    pub fn data_mut(&mut self) -> &mut TrackedData {
        &mut self.variants
    }

    /// Gets the [`EntityKind`] of this entity.
    pub fn kind(&self) -> EntityKind {
        self.variants.kind()
    }

    /// Triggers an entity event for this entity.
    pub fn push_event(&mut self, event: EntityEvent) {
        self.events.push(event);
    }

    /// Gets the [`WorldId`](crate::world::WorldId) of the world this entity is
    /// located in.
    ///
    /// By default, entities are located in
    /// [`WorldId::NULL`](crate::world::WorldId::NULL).
    pub fn world(&self) -> WorldId {
        self.world
    }

    /// Sets the world this entity is located in.
    pub fn set_world(&mut self, world: WorldId) {
        self.world = world;
    }

    pub(crate) fn old_world(&self) -> WorldId {
        self.old_world
    }

    /// Gets the position of this entity in the world it inhabits.
    ///
    /// The position of an entity is located on the botton of its
    /// hitbox and not the center.
    pub fn position(&self) -> Vec3<f64> {
        self.position
    }

    /// Sets the position of this entity in the world it inhabits.
    ///
    /// The position of an entity is located on the botton of its
    /// hitbox and not the center.
    pub fn set_position(&mut self, pos: impl Into<Vec3<f64>>) {
        self.position = pos.into();
    }

    /// Returns the position of this entity as it existed at the end of the
    /// previous tick.
    pub(crate) fn old_position(&self) -> Vec3<f64> {
        self.old_position
    }

    /// Gets the yaw of this entity in degrees.
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Sets the yaw of this entity in degrees.
    pub fn set_yaw(&mut self, yaw: f32) {
        if self.yaw != yaw {
            self.yaw = yaw;
            self.bits.set_yaw_or_pitch_modified(true);
        }
    }

    /// Gets the pitch of this entity in degrees.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Sets the pitch of this entity in degrees.
    pub fn set_pitch(&mut self, pitch: f32) {
        if self.pitch != pitch {
            self.pitch = pitch;
            self.bits.set_yaw_or_pitch_modified(true);
        }
    }

    /// Gets the head yaw of this entity in degrees.
    pub fn head_yaw(&self) -> f32 {
        self.head_yaw
    }

    /// Sets the head yaw of this entity in degrees.
    pub fn set_head_yaw(&mut self, head_yaw: f32) {
        if self.head_yaw != head_yaw {
            self.head_yaw = head_yaw;
            self.bits.set_head_yaw_modified(true);
        }
    }

    /// Gets the velocity of this entity in meters per second.
    pub fn velocity(&self) -> Vec3<f32> {
        self.velocity
    }

    /// Sets the velocity of this entity in meters per second.
    pub fn set_velocity(&mut self, velocity: impl Into<Vec3<f32>>) {
        let new_vel = velocity.into();

        if self.velocity != new_vel {
            self.velocity = new_vel;
            self.bits.set_velocity_modified(true);
        }
    }

    /// Gets the value of the "on ground" flag.
    pub fn on_ground(&self) -> bool {
        self.bits.on_ground()
    }

    /// Sets the value of the "on ground" flag.
    pub fn set_on_ground(&mut self, on_ground: bool) {
        self.bits.set_on_ground(on_ground);
    }

    /// Gets the UUID of this entity.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Returns the hitbox of this entity.
    ///
    /// The hitbox describes the space that an entity occupies. Clients interact
    /// with this space to create an [interact event].
    ///
    /// The hitbox of an entity is determined by its position, entity type, and
    /// other state specific to that type.
    ///
    /// [interact event]: crate::client::ClientEvent::InteractWithEntity
    pub fn hitbox(&self) -> Aabb<f64> {
        fn baby(is_baby: bool, adult_hitbox: [f64; 3]) -> [f64; 3] {
            if is_baby {
                adult_hitbox.map(|a| a / 2.0)
            } else {
                adult_hitbox
            }
        }

        fn item_frame(pos: Vec3<f64>, rotation: i32) -> Aabb<f64> {
            let mut center_pos = pos + 0.5;

            match rotation {
                0 => center_pos.y += 0.46875,
                1 => center_pos.y -= 0.46875,
                2 => center_pos.z += 0.46875,
                3 => center_pos.z -= 0.46875,
                4 => center_pos.x += 0.46875,
                5 => center_pos.x -= 0.46875,
                _ => center_pos.y -= 0.46875,
            };

            let bounds = Vec3::from(match rotation {
                0 | 1 => [0.75, 0.0625, 0.75],
                2 | 3 => [0.75, 0.75, 0.0625],
                4 | 5 => [0.0625, 0.75, 0.75],
                _ => [0.75, 0.0625, 0.75],
            });

            Aabb {
                min: center_pos - bounds / 2.0,
                max: center_pos + bounds / 2.0,
            }
        }

        let dimensions = match &self.variants {
            TrackedData::Allay(_) => [0.6, 0.35, 0.6],
            TrackedData::ChestBoat(_) => [1.375, 0.5625, 1.375],
            TrackedData::Frog(_) => [0.5, 0.5, 0.5],
            TrackedData::Tadpole(_) => [0.4, 0.3, 0.4],
            TrackedData::Warden(e) => match e.get_pose() {
                Pose::Emerging | Pose::Digging => [0.9, 1.0, 0.9],
                _ => [0.9, 2.9, 0.9],
            },
            TrackedData::AreaEffectCloud(e) => [
                e.get_radius() as f64 * 2.0,
                0.5,
                e.get_radius() as f64 * 2.0,
            ],
            TrackedData::ArmorStand(e) => {
                if e.get_marker() {
                    [0.0, 0.0, 0.0]
                } else if e.get_small() {
                    [0.5, 0.9875, 0.5]
                } else {
                    [0.5, 1.975, 0.5]
                }
            }
            TrackedData::Arrow(_) => [0.5, 0.5, 0.5],
            TrackedData::Axolotl(_) => [1.3, 0.6, 1.3],
            TrackedData::Bat(_) => [0.5, 0.9, 0.5],
            TrackedData::Bee(e) => baby(e.get_child(), [0.7, 0.6, 0.7]),
            TrackedData::Blaze(_) => [0.6, 1.8, 0.6],
            TrackedData::Boat(_) => [1.375, 0.5625, 1.375],
            TrackedData::Camel(e) => baby(e.get_child(), [1.7, 2.375, 1.7]),
            TrackedData::Cat(_) => [0.6, 0.7, 0.6],
            TrackedData::CaveSpider(_) => [0.7, 0.5, 0.7],
            TrackedData::Chicken(e) => baby(e.get_child(), [0.4, 0.7, 0.4]),
            TrackedData::Cod(_) => [0.5, 0.3, 0.5],
            TrackedData::Cow(e) => baby(e.get_child(), [0.9, 1.4, 0.9]),
            TrackedData::Creeper(_) => [0.6, 1.7, 0.6],
            TrackedData::Dolphin(_) => [0.9, 0.6, 0.9],
            TrackedData::Donkey(e) => baby(e.get_child(), [1.5, 1.39648, 1.5]),
            TrackedData::DragonFireball(_) => [1.0, 1.0, 1.0],
            TrackedData::Drowned(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
            TrackedData::ElderGuardian(_) => [1.9975, 1.9975, 1.9975],
            TrackedData::EndCrystal(_) => [2.0, 2.0, 2.0],
            TrackedData::EnderDragon(_) => [16.0, 8.0, 16.0],
            TrackedData::Enderman(_) => [0.6, 2.9, 0.6],
            TrackedData::Endermite(_) => [0.4, 0.3, 0.4],
            TrackedData::Evoker(_) => [0.6, 1.95, 0.6],
            TrackedData::EvokerFangs(_) => [0.5, 0.8, 0.5],
            TrackedData::ExperienceOrb(_) => [0.5, 0.5, 0.5],
            TrackedData::EyeOfEnder(_) => [0.25, 0.25, 0.25],
            TrackedData::FallingBlock(_) => [0.98, 0.98, 0.98],
            TrackedData::FireworkRocket(_) => [0.25, 0.25, 0.25],
            TrackedData::Fox(e) => baby(e.get_child(), [0.6, 0.7, 0.6]),
            TrackedData::Ghast(_) => [4.0, 4.0, 4.0],
            TrackedData::Giant(_) => [3.6, 12.0, 3.6],
            TrackedData::GlowItemFrame(e) => return item_frame(self.position, e.get_rotation()),
            TrackedData::GlowSquid(_) => [0.8, 0.8, 0.8],
            TrackedData::Goat(e) => {
                if e.get_pose() == Pose::LongJumping {
                    baby(e.get_child(), [0.63, 0.91, 0.63])
                } else {
                    baby(e.get_child(), [0.9, 1.3, 0.9])
                }
            }
            TrackedData::Guardian(_) => [0.85, 0.85, 0.85],
            TrackedData::Hoglin(e) => baby(e.get_child(), [1.39648, 1.4, 1.39648]),
            TrackedData::Horse(e) => baby(e.get_child(), [1.39648, 1.6, 1.39648]),
            TrackedData::Husk(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
            TrackedData::Illusioner(_) => [0.6, 1.95, 0.6],
            TrackedData::IronGolem(_) => [1.4, 2.7, 1.4],
            TrackedData::Item(_) => [0.25, 0.25, 0.25],
            TrackedData::ItemFrame(e) => return item_frame(self.position, e.get_rotation()),
            TrackedData::Fireball(_) => [1.0, 1.0, 1.0],
            TrackedData::LeashKnot(_) => [0.375, 0.5, 0.375],
            TrackedData::Lightning(_) => [0.0, 0.0, 0.0],
            TrackedData::Llama(e) => baby(e.get_child(), [0.9, 1.87, 0.9]),
            TrackedData::LlamaSpit(_) => [0.25, 0.25, 0.25],
            TrackedData::MagmaCube(e) => {
                let s = 0.5202 * e.get_slime_size() as f64;
                [s, s, s]
            }
            TrackedData::Marker(_) => [0.0, 0.0, 0.0],
            TrackedData::Minecart(_) => [0.98, 0.7, 0.98],
            TrackedData::ChestMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::CommandBlockMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::FurnaceMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::HopperMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::SpawnerMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::TntMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::Mule(e) => baby(e.get_child(), [1.39648, 1.6, 1.39648]),
            TrackedData::Mooshroom(e) => baby(e.get_child(), [0.9, 1.4, 0.9]),
            TrackedData::Ocelot(e) => baby(e.get_child(), [0.6, 0.7, 0.6]),
            TrackedData::Painting(e) => {
                let bounds: Vec3<u32> = match e.get_variant() {
                    PaintingKind::Kebab => [1, 1, 1],
                    PaintingKind::Aztec => [1, 1, 1],
                    PaintingKind::Alban => [1, 1, 1],
                    PaintingKind::Aztec2 => [1, 1, 1],
                    PaintingKind::Bomb => [1, 1, 1],
                    PaintingKind::Plant => [1, 1, 1],
                    PaintingKind::Wasteland => [1, 1, 1],
                    PaintingKind::Pool => [2, 1, 2],
                    PaintingKind::Courbet => [2, 1, 2],
                    PaintingKind::Sea => [2, 1, 2],
                    PaintingKind::Sunset => [2, 1, 2],
                    PaintingKind::Creebet => [2, 1, 2],
                    PaintingKind::Wanderer => [1, 2, 1],
                    PaintingKind::Graham => [1, 2, 1],
                    PaintingKind::Match => [2, 2, 2],
                    PaintingKind::Bust => [2, 2, 2],
                    PaintingKind::Stage => [2, 2, 2],
                    PaintingKind::Void => [2, 2, 2],
                    PaintingKind::SkullAndRoses => [2, 2, 2],
                    PaintingKind::Wither => [2, 2, 2],
                    PaintingKind::Fighters => [4, 2, 4],
                    PaintingKind::Pointer => [4, 4, 4],
                    PaintingKind::Pigscene => [4, 4, 4],
                    PaintingKind::BurningSkull => [4, 4, 4],
                    PaintingKind::Skeleton => [4, 3, 4],
                    PaintingKind::Earth => [2, 2, 2],
                    PaintingKind::Wind => [2, 2, 2],
                    PaintingKind::Water => [2, 2, 2],
                    PaintingKind::Fire => [2, 2, 2],
                    PaintingKind::DonkeyKong => [4, 3, 4],
                }
                .into();

                let mut center_pos = self.position + 0.5;

                let (facing_x, facing_z, cc_facing_x, cc_facing_z) =
                    match ((self.yaw + 45.0).rem_euclid(360.0) / 90.0) as u8 {
                        0 => (0, 1, 1, 0),   // South
                        1 => (-1, 0, 0, 1),  // West
                        2 => (0, -1, -1, 0), // North
                        _ => (1, 0, 0, -1),  // East
                    };

                center_pos.x -= facing_x as f64 * 0.46875;
                center_pos.z -= facing_z as f64 * 0.46875;

                center_pos.x += cc_facing_x as f64 * if bounds.x % 2 == 0 { 0.5 } else { 0.0 };
                center_pos.y += if bounds.y % 2 == 0 { 0.5 } else { 0.0 };
                center_pos.z += cc_facing_z as f64 * if bounds.z % 2 == 0 { 0.5 } else { 0.0 };

                let bounds = match (facing_x, facing_z) {
                    (1, 0) | (-1, 0) => bounds.as_().with_x(0.0625),
                    _ => bounds.as_().with_z(0.0625),
                };

                return Aabb {
                    min: center_pos - bounds / 2.0,
                    max: center_pos + bounds / 2.0,
                };
            }
            TrackedData::Panda(e) => baby(e.get_child(), [1.3, 1.25, 1.3]),
            TrackedData::Parrot(_) => [0.5, 0.9, 0.5],
            TrackedData::Phantom(_) => [0.9, 0.5, 0.9],
            TrackedData::Pig(e) => baby(e.get_child(), [0.9, 0.9, 0.9]),
            TrackedData::Piglin(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
            TrackedData::PiglinBrute(_) => [0.6, 1.95, 0.6],
            TrackedData::Pillager(_) => [0.6, 1.95, 0.6],
            TrackedData::PolarBear(e) => baby(e.get_child(), [1.4, 1.4, 1.4]),
            TrackedData::Tnt(_) => [0.98, 0.98, 0.98],
            TrackedData::Pufferfish(_) => [0.7, 0.7, 0.7],
            TrackedData::Rabbit(e) => baby(e.get_child(), [0.4, 0.5, 0.4]),
            TrackedData::Ravager(_) => [1.95, 2.2, 1.95],
            TrackedData::Salmon(_) => [0.7, 0.4, 0.7],
            TrackedData::Sheep(e) => baby(e.get_child(), [0.9, 1.3, 0.9]),
            TrackedData::Shulker(e) => {
                const PI: f64 = std::f64::consts::PI;

                let pos = self.position + 0.5;
                let mut min = pos - 0.5;
                let mut max = pos + 0.5;

                let peek = 0.5 - f64::cos(e.get_peek_amount() as f64 * 0.01 * PI) * 0.5;

                match e.get_attached_face() {
                    Facing::Down => max.y += peek,
                    Facing::Up => min.y -= peek,
                    Facing::North => max.z += peek,
                    Facing::South => min.z -= peek,
                    Facing::West => max.x += peek,
                    Facing::East => min.x -= peek,
                }

                return Aabb { min, max };
            }
            TrackedData::ShulkerBullet(_) => [0.3125, 0.3125, 0.3125],
            TrackedData::Silverfish(_) => [0.4, 0.3, 0.4],
            TrackedData::Skeleton(_) => [0.6, 1.99, 0.6],
            TrackedData::SkeletonHorse(e) => baby(e.get_child(), [1.39648, 1.6, 1.39648]),
            TrackedData::Slime(e) => {
                let s = 0.5202 * e.get_slime_size() as f64;
                [s, s, s]
            }
            TrackedData::SmallFireball(_) => [0.3125, 0.3125, 0.3125],
            TrackedData::SnowGolem(_) => [0.7, 1.9, 0.7],
            TrackedData::Snowball(_) => [0.25, 0.25, 0.25],
            TrackedData::SpectralArrow(_) => [0.5, 0.5, 0.5],
            TrackedData::Spider(_) => [1.4, 0.9, 1.4],
            TrackedData::Squid(_) => [0.8, 0.8, 0.8],
            TrackedData::Stray(_) => [0.6, 1.99, 0.6],
            TrackedData::Strider(e) => baby(e.get_child(), [0.9, 1.7, 0.9]),
            TrackedData::Egg(_) => [0.25, 0.25, 0.25],
            TrackedData::EnderPearl(_) => [0.25, 0.25, 0.25],
            TrackedData::ExperienceBottle(_) => [0.25, 0.25, 0.25],
            TrackedData::Potion(_) => [0.25, 0.25, 0.25],
            TrackedData::Trident(_) => [0.5, 0.5, 0.5],
            TrackedData::TraderLlama(_) => [0.9, 1.87, 0.9],
            TrackedData::TropicalFish(_) => [0.5, 0.4, 0.5],
            TrackedData::Turtle(e) => {
                if e.get_child() {
                    [0.36, 0.12, 0.36]
                } else {
                    [1.2, 0.4, 1.2]
                }
            }
            TrackedData::Vex(_) => [0.4, 0.8, 0.4],
            TrackedData::Villager(e) => baby(e.get_child(), [0.6, 1.95, 0.6]),
            TrackedData::Vindicator(_) => [0.6, 1.95, 0.6],
            TrackedData::WanderingTrader(_) => [0.6, 1.95, 0.6],
            TrackedData::Witch(_) => [0.6, 1.95, 0.6],
            TrackedData::Wither(_) => [0.9, 3.5, 0.9],
            TrackedData::WitherSkeleton(_) => [0.7, 2.4, 0.7],
            TrackedData::WitherSkull(_) => [0.3125, 0.3125, 0.3125],
            TrackedData::Wolf(e) => baby(e.get_child(), [0.6, 0.85, 0.6]),
            TrackedData::Zoglin(e) => baby(e.get_baby(), [1.39648, 1.4, 1.39648]),
            TrackedData::Zombie(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
            TrackedData::ZombieHorse(e) => baby(e.get_child(), [1.39648, 1.6, 1.39648]),
            TrackedData::ZombieVillager(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
            TrackedData::ZombifiedPiglin(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
            TrackedData::Player(e) => match e.get_pose() {
                Pose::Standing => [0.6, 1.8, 0.6],
                Pose::Sleeping => [0.2, 0.2, 0.2],
                Pose::FallFlying => [0.6, 0.6, 0.6],
                Pose::Swimming => [0.6, 0.6, 0.6],
                Pose::SpinAttack => [0.6, 0.6, 0.6],
                Pose::Sneaking => [0.6, 1.5, 0.6],
                Pose::Dying => [0.2, 0.2, 0.2],
                _ => [0.6, 1.8, 0.6],
            },
            TrackedData::FishingBobber(_) => [0.25, 0.25, 0.25],
        };

        aabb_from_bottom_and_size(self.position, dimensions.into())
    }

    pub fn deleted(&self) -> bool {
        self.bits.deleted()
    }

    pub fn set_deleted(&mut self, deleted: bool) {
        self.bits.set_deleted(deleted)
    }

    /// Sends the appropriate packets to initialize the entity. This will spawn
    /// the entity and initialize tracked data.
    pub(crate) fn send_init_packets(
        &self,
        send: &mut PlayPacketSender,
        position: Vec3<f64>,
        this_id: EntityId,
        scratch: &mut Vec<u8>,
    ) -> anyhow::Result<()> {
        let with_object_data = |data| SpawnEntity {
            entity_id: VarInt(this_id.to_raw()),
            object_uuid: self.uuid,
            kind: VarInt(self.kind() as i32),
            position: position.into_array(),
            pitch: ByteAngle::from_degrees(self.pitch),
            yaw: ByteAngle::from_degrees(self.yaw),
            head_yaw: ByteAngle::from_degrees(self.head_yaw),
            data: VarInt(data),
            velocity: velocity_to_packet_units(self.velocity).into_array(),
        };

        match &self.variants {
            TrackedData::Marker(_) => {}
            TrackedData::ExperienceOrb(_) => send.append_packet(&SpawnExperienceOrb {
                entity_id: VarInt(this_id.to_raw()),
                position: position.into_array(),
                count: 0, // TODO
            })?,
            TrackedData::Player(_) => {
                send.append_packet(&SpawnPlayer {
                    entity_id: VarInt(this_id.to_raw()),
                    player_uuid: self.uuid,
                    position: position.into_array(),
                    yaw: ByteAngle::from_degrees(self.yaw),
                    pitch: ByteAngle::from_degrees(self.pitch),
                })?;

                // Player spawn packet doesn't include head yaw for some reason.
                send.append_packet(&SetHeadRotation {
                    entity_id: VarInt(this_id.to_raw()),
                    head_yaw: ByteAngle::from_degrees(self.head_yaw),
                })?;
            }
            TrackedData::ItemFrame(e) => send.append_packet(&with_object_data(e.get_rotation()))?,
            TrackedData::GlowItemFrame(e) => {
                send.append_packet(&with_object_data(e.get_rotation()))?
            }

            TrackedData::Painting(_) => send.append_packet(&with_object_data(
                match ((self.yaw + 45.0).rem_euclid(360.0) / 90.0) as u8 {
                    0 => 3,
                    1 => 4,
                    2 => 2,
                    _ => 5,
                },
            ))?,
            // TODO: set block state ID for falling block.
            TrackedData::FallingBlock(_) => send.append_packet(&with_object_data(1))?,
            TrackedData::FishingBobber(e) => {
                send.append_packet(&with_object_data(e.get_hook_entity_id()))?
            }
            TrackedData::Warden(e) => {
                send.append_packet(&with_object_data((e.get_pose() == Pose::Emerging).into()))?
            }
            _ => send.append_packet(&with_object_data(0))?,
        }

        scratch.clear();
        self.variants.write_initial_tracked_data(scratch);
        if !scratch.is_empty() {
            send.append_packet(&SetEntityMetadata {
                entity_id: VarInt(this_id.to_raw()),
                metadata: RawBytes(scratch),
            })?;
        }

        Ok(())
    }

    /// Writes the appropriate packets to update the entity (Position, tracked
    /// data, and event packets).
    pub(crate) fn write_update_packets(
        &self,
        mut writer: impl WritePacket,
        this_id: EntityId,
        scratch: &mut Vec<u8>,
    ) -> anyhow::Result<()> {
        let entity_id = VarInt(this_id.to_raw());

        let position_delta = self.position - self.old_position;
        let needs_teleport = position_delta.map(f64::abs).reduce_partial_max() >= 8.0;
        let changed_position = self.position != self.old_position;

        if changed_position && !needs_teleport && self.bits.yaw_or_pitch_modified() {
            writer.write_packet(&UpdateEntityPositionAndRotation {
                entity_id,
                delta: (position_delta * 4096.0).as_::<i16>().into_array(),
                yaw: ByteAngle::from_degrees(self.yaw),
                pitch: ByteAngle::from_degrees(self.pitch),
                on_ground: self.bits.on_ground(),
            })?;
        } else {
            if changed_position && !needs_teleport {
                writer.write_packet(&UpdateEntityPosition {
                    entity_id,
                    delta: (position_delta * 4096.0).as_::<i16>().into_array(),
                    on_ground: self.bits.on_ground(),
                })?;
            }

            if self.bits.yaw_or_pitch_modified() {
                writer.write_packet(&UpdateEntityRotation {
                    entity_id,
                    yaw: ByteAngle::from_degrees(self.yaw),
                    pitch: ByteAngle::from_degrees(self.pitch),
                    on_ground: self.bits.on_ground(),
                })?;
            }
        }

        if needs_teleport {
            writer.write_packet(&TeleportEntity {
                entity_id,
                position: self.position.into_array(),
                yaw: ByteAngle::from_degrees(self.yaw),
                pitch: ByteAngle::from_degrees(self.pitch),
                on_ground: self.bits.on_ground(),
            })?;
        }

        if self.bits.velocity_modified() {
            writer.write_packet(&SetEntityVelocity {
                entity_id,
                velocity: velocity_to_packet_units(self.velocity).into_array(),
            })?;
        }

        if self.bits.head_yaw_modified() {
            writer.write_packet(&SetHeadRotation {
                entity_id,
                head_yaw: ByteAngle::from_degrees(self.head_yaw),
            })?;
        }

        scratch.clear();
        self.variants.write_updated_tracked_data(scratch);
        if !scratch.is_empty() {
            writer.write_packet(&SetEntityMetadata {
                entity_id,
                metadata: RawBytes(scratch),
            })?;
        }

        for &event in &self.events {
            match event.status_or_animation() {
                StatusOrAnimation::Status(code) => writer.write_packet(&EntityEventPacket {
                    entity_id: entity_id.0,
                    entity_status: code,
                })?,
                StatusOrAnimation::Animation(code) => writer.write_packet(&EntityAnimationS2c {
                    entity_id,
                    animation: code,
                })?,
            }
        }

        Ok(())
    }
}

pub(crate) fn velocity_to_packet_units(vel: Vec3<f32>) -> Vec3<i16> {
    // The saturating cast to i16 is desirable.
    (8000.0 / STANDARD_TPS as f32 * vel).as_()
}

#[cfg(test)]
mod tests {
    use super::*;

    type MockConfig = crate::config::MockConfig<(), (), u8>;

    #[test]
    fn entities_has_valid_new_state() {
        let mut entities: Entities<MockConfig> = Entities::new();
        let raw_id: i32 = 8675309;
        let entity_id = EntityId(Key::new(
            202298,
            NonZeroU32::new(raw_id as u32).expect("value given should never be zero!"),
        ));
        let uuid = Uuid::from_bytes([2; 16]);
        assert!(entities.is_empty());
        assert!(entities.get(entity_id).is_none());
        assert!(entities.get_mut(entity_id).is_none());
        assert!(entities.get_with_uuid(uuid).is_none());
        assert!(entities.get_with_raw_id(raw_id).is_none());
    }

    #[test]
    fn entities_can_be_set_and_get() {
        let mut entities: Entities<MockConfig> = Entities::new();
        assert!(entities.is_empty());
        let (player_id, player_entity) = entities.insert(EntityKind::Player, 1);
        assert_eq!(player_entity.state, 1);
        assert_eq!(entities.get(player_id).unwrap().state, 1);
        let mut_player_entity = entities
            .get_mut(player_id)
            .expect("failed to get mutable reference");
        mut_player_entity.state = 100;
        assert_eq!(entities.get(player_id).unwrap().state, 100);
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn entities_can_be_set_and_get_with_uuid() {
        let mut entities: Entities<MockConfig> = Entities::new();
        let uuid = Uuid::from_bytes([2; 16]);
        assert!(entities.is_empty());
        let (zombie_id, zombie_entity) = entities
            .insert_with_uuid(EntityKind::Zombie, uuid, 1)
            .expect("unexpected Uuid collision when inserting to an empty collection");
        assert_eq!(zombie_entity.state, 1);
        let maybe_zombie = entities
            .get_with_uuid(uuid)
            .expect("UUID lookup failed on item already added to this collection");
        assert_eq!(zombie_id, maybe_zombie);
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn entities_can_be_set_and_get_with_raw_id() {
        let mut entities: Entities<MockConfig> = Entities::new();
        assert!(entities.is_empty());
        let (boat_id, boat_entity) = entities.insert(EntityKind::Boat, 12);
        assert_eq!(boat_entity.state, 12);
        let (cat_id, cat_entity) = entities.insert(EntityKind::Cat, 75);
        assert_eq!(cat_entity.state, 75);
        let maybe_boat_id = entities
            .get_with_raw_id(boat_id.0.version.get() as i32)
            .expect("raw id lookup failed on item already added to this collection")
            .0;
        let maybe_boat = entities
            .get(maybe_boat_id)
            .expect("failed to look up item already added to collection");
        assert_eq!(maybe_boat.state, 12);
        let maybe_cat_id = entities
            .get_with_raw_id(cat_id.0.version.get() as i32)
            .expect("raw id lookup failed on item already added to this collection")
            .0;
        let maybe_cat = entities
            .get(maybe_cat_id)
            .expect("failed to look up item already added to collection");
        assert_eq!(maybe_cat.state, 75);
        assert_eq!(entities.len(), 2);
    }
}
