pub mod meta;
pub mod types;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::iter::FusedIterator;
use std::ops::Deref;

use bitfield_struct::bitfield;
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use vek::{Aabb, Vec3};

use crate::byte_angle::ByteAngle;
use crate::packets::play::{
    ClientPlayPacket, EntityMetadata, SpawnEntity, SpawnExperienceOrb, SpawnLivingEntity,
    SpawnPainting, SpawnPlayer,
};
use crate::protocol::RawBytes;
use crate::slotmap::{Key, SlotMap};
use crate::util::aabb_from_bottom_and_size;
use crate::var_int::VarInt;

pub struct Entities {
    sm: SlotMap<Entity>,
    uuid_to_entity: HashMap<Uuid, EntityId>,
}

pub struct EntitiesMut<'a>(&'a mut Entities);

impl<'a> Deref for EntitiesMut<'a> {
    type Target = Entities;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Entities {
    pub(crate) fn new() -> Self {
        Self {
            sm: SlotMap::new(),
            uuid_to_entity: HashMap::new(),
        }
    }

    /// Returns the number of live entities.
    pub fn count(&self) -> usize {
        self.sm.count()
    }

    /// Gets the [`EntityId`] of the entity with the given UUID in an efficient
    /// manner.
    ///
    /// Returns `None` if there is no entity with the provided UUID. Returns
    /// `Some` otherwise.
    pub fn get_with_uuid(&self, uuid: Uuid) -> Option<EntityId> {
        self.uuid_to_entity.get(&uuid).cloned()
    }

    pub fn get(&self, entity: EntityId) -> Option<&Entity> {
        self.sm.get(entity.0)
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (EntityId, &Entity)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (EntityId(k), v))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (EntityId, &Entity)> + Clone + '_ {
        self.sm.par_iter().map(|(k, v)| (EntityId(k), v))
    }
}

impl<'a> EntitiesMut<'a> {
    pub(crate) fn new(entities: &'a mut Entities) -> Self {
        Self(entities)
    }

    /// Spawns a new entity with the default data. The new entity's [`EntityId`]
    /// is returned.
    ///
    /// To actually see the new entity, set its position to somewhere nearby and
    /// [set its type](EntityData::set_type) to something visible.
    pub fn create(&mut self) -> EntityId {
        loop {
            let uuid = Uuid::from_bytes(rand::random());
            if let Some(entity) = self.create_with_uuid(uuid) {
                return entity;
            }
        }
    }

    /// Like [`create`](Entities::create), but requires specifying the new
    /// entity's UUID. This is useful for deserialization.
    ///
    /// The provided UUID must not conflict with an existing entity UUID in this
    /// world. If it does, `None` is returned and the entity is not spawned.
    pub fn create_with_uuid(&mut self, uuid: Uuid) -> Option<EntityId> {
        match self.0.uuid_to_entity.entry(uuid) {
            Entry::Occupied(_) => None,
            Entry::Vacant(ve) => {
                let entity = EntityId(self.0.sm.insert(Entity {
                    flags: EntityFlags(0),
                    meta: EntityMeta::new(EntityType::Marker),
                    new_position: Vec3::default(),
                    old_position: Vec3::default(),
                    yaw: 0.0,
                    pitch: 0.0,
                    head_yaw: 0.0,
                    head_pitch: 0.0,
                    velocity: Vec3::default(),
                    uuid,
                }));

                ve.insert(entity);

                // TODO: insert into partition.

                Some(entity)
            }
        }
    }

    pub fn delete(&mut self, entity: EntityId) -> bool {
        if let Some(e) = self.0.sm.remove(entity.0) {
            self.0
                .uuid_to_entity
                .remove(&e.uuid)
                .expect("UUID should have been in UUID map");

            // TODO: remove entity from partition.
            true
        } else {
            false
        }
    }

    pub fn retain(&mut self, mut f: impl FnMut(EntityId, EntityMut) -> bool) {
        // TODO
        self.0.sm.retain(|k, v| f(EntityId(k), EntityMut(v)))
    }

    pub fn get_mut(&mut self, entity: EntityId) -> Option<EntityMut> {
        self.0.sm.get_mut(entity.0).map(EntityMut)
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (EntityId, EntityMut)> + '_ {
        self.0
            .sm
            .iter_mut()
            .map(|(k, v)| (EntityId(k), EntityMut(v)))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (EntityId, EntityMut)> + '_ {
        self.0
            .sm
            .par_iter_mut()
            .map(|(k, v)| (EntityId(k), EntityMut(v)))
    }

    pub(crate) fn update(&mut self) {
        for (_, e) in self.iter_mut() {
            e.0.old_position = e.new_position;
            e.0.meta.clear_modifications();

            let on_ground = e.0.flags.on_ground();
            e.0.flags = EntityFlags(0);
            e.0.flags.set_on_ground(on_ground);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EntityId(Key);

impl EntityId {
    pub(crate) fn to_network_id(self) -> i32 {
        // ID 0 is reserved for clients.
        self.0.index() as i32 + 1
    }
}

pub struct Entity {
    flags: EntityFlags,
    meta: EntityMeta,
    new_position: Vec3<f64>,
    old_position: Vec3<f64>,
    yaw: f32,
    pitch: f32,
    head_yaw: f32,
    head_pitch: f32,
    velocity: Vec3<f32>,
    uuid: Uuid,
}

pub struct EntityMut<'a>(&'a mut Entity);

impl<'a> Deref for EntityMut<'a> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Contains a bit for certain fields in [`Entity`] to track if they have been
/// modified.
#[bitfield(u8)]
pub(crate) struct EntityFlags {
    /// When the type of this entity changes.
    pub type_modified: bool,
    pub yaw_or_pitch_modified: bool,
    pub head_yaw_modified: bool,
    pub velocity_modified: bool,
    pub on_ground: bool,
    #[bits(3)]
    _pad: u8,
}

impl Entity {
    pub(crate) fn flags(&self) -> EntityFlags {
        self.flags
    }

    /// Returns a reference to this entity's [`EntityMeta`].
    pub fn meta(&self) -> &EntityMeta {
        &self.meta
    }

    /// Returns the [`EntityType`] of this entity.
    pub fn typ(&self) -> EntityType {
        self.meta.typ()
    }

    /// Returns the position of this entity in the world it inhabits.
    pub fn position(&self) -> Vec3<f64> {
        self.new_position
    }

    /// Returns the position of this entity as it existed at the end of the
    /// previous tick.
    pub fn old_position(&self) -> Vec3<f64> {
        self.old_position
    }

    /// Gets the yaw of this entity (in degrees).
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Gets the pitch of this entity (in degrees).
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Gets the head yaw of this entity (in degrees).
    pub fn head_yaw(&self) -> f32 {
        self.head_yaw
    }

    /// Gets the velocity of this entity in meters per second.
    pub fn velocity(&self) -> Vec3<f32> {
        self.velocity
    }

    pub fn on_ground(&self) -> bool {
        self.flags.on_ground()
    }

    /// Gets the metadata packet to send to clients after this entity has been
    /// spawned.
    ///
    /// Is `None` if there is no initial metadata.
    pub(crate) fn initial_metadata_packet(&self, this_id: EntityId) -> Option<EntityMetadata> {
        self.meta.initial_metadata().map(|meta| EntityMetadata {
            entity_id: VarInt(this_id.to_network_id()),
            metadata: RawBytes(meta),
        })
    }

    /// Gets the metadata packet to send to clients when the entity is modified.
    ///
    /// Is `None` if this entity's metadata has not been modified.
    pub(crate) fn updated_metadata_packet(&self, this_id: EntityId) -> Option<EntityMetadata> {
        self.meta.updated_metadata().map(|meta| EntityMetadata {
            entity_id: VarInt(this_id.to_network_id()),
            metadata: RawBytes(meta),
        })
    }

    pub(crate) fn spawn_packet(&self, this_id: EntityId) -> Option<EntitySpawnPacket> {
        use EntityMeta::*;
        match &self.meta {
            Marker(_) => None,
            ExperienceOrb(_) => Some(EntitySpawnPacket::SpawnExperienceOrb(SpawnExperienceOrb {
                entity_id: VarInt(this_id.to_network_id()),
                position: self.new_position,
                count: 0, // TODO
            })),
            Painting(_) => todo!(),
            Player(_) => todo!(),
            AreaEffectCloud(_)
            | Arrow(_)
            | Boat(_)
            | DragonFireball(_)
            | EndCrystal(_)
            | EvokerFangs(_)
            | EyeOfEnder(_)
            | FallingBlock(_)
            | FireworkRocket(_)
            | GlowItemFrame(_)
            | Item(_)
            | ItemFrame(_)
            | Fireball(_)
            | LeashKnot(_)
            | LightningBolt(_)
            | LlamaSpit(_)
            | Minecart(_)
            | ChestMinecart(_)
            | CommandBlockMinecart(_)
            | FurnaceMinecart(_)
            | HopperMinecart(_)
            | SpawnerMinecart(_)
            | TntMinecart(_)
            | Tnt(_)
            | ShulkerBullet(_)
            | SmallFireball(_)
            | Snowball(_)
            | SpectralArrow(_)
            | Egg(_)
            | EnderPearl(_)
            | ExperienceBottle(_)
            | Potion(_)
            | Trident(_)
            | WitherSkull(_)
            | FishingBobber(_) => Some(EntitySpawnPacket::SpawnEntity(SpawnEntity {
                entity_id: VarInt(this_id.to_network_id()),
                object_uuid: self.uuid,
                typ: VarInt(self.typ() as i32),
                position: self.new_position,
                pitch: ByteAngle::from_degrees(self.pitch),
                yaw: ByteAngle::from_degrees(self.yaw),
                data: 1, // TODO
                velocity: velocity_to_packet_units(self.velocity),
            })),

            ArmorStand(_) | Axolotl(_) | Bat(_) | Bee(_) | Blaze(_) | Cat(_) | CaveSpider(_)
            | Chicken(_) | Cod(_) | Cow(_) | Creeper(_) | Dolphin(_) | Donkey(_) | Drowned(_)
            | ElderGuardian(_) | EnderDragon(_) | Enderman(_) | Endermite(_) | Evoker(_)
            | Fox(_) | Ghast(_) | Giant(_) | GlowSquid(_) | Goat(_) | Guardian(_) | Hoglin(_)
            | Horse(_) | Husk(_) | Illusioner(_) | IronGolem(_) | Llama(_) | MagmaCube(_)
            | Mule(_) | Mooshroom(_) | Ocelot(_) | Panda(_) | Parrot(_) | Phantom(_) | Pig(_)
            | Piglin(_) | PiglinBrute(_) | Pillager(_) | PolarBear(_) | Pufferfish(_)
            | Rabbit(_) | Ravager(_) | Salmon(_) | Sheep(_) | Shulker(_) | Silverfish(_)
            | Skeleton(_) | SkeletonHorse(_) | Slime(_) | SnowGolem(_) | Spider(_) | Squid(_)
            | Stray(_) | Strider(_) | TraderLlama(_) | TropicalFish(_) | Turtle(_) | Vex(_)
            | Villager(_) | Vindicator(_) | WanderingTrader(_) | Witch(_) | Wither(_)
            | WitherSkeleton(_) | Wolf(_) | Zoglin(_) | Zombie(_) | ZombieHorse(_)
            | ZombieVillager(_) | ZombifiedPiglin(_) => {
                Some(EntitySpawnPacket::SpawnLivingEntity(SpawnLivingEntity {
                    entity_id: VarInt(this_id.to_network_id()),
                    entity_uuid: self.uuid,
                    typ: VarInt(self.typ() as i32),
                    position: self.new_position,
                    yaw: ByteAngle::from_degrees(self.yaw),
                    pitch: ByteAngle::from_degrees(self.pitch),
                    head_yaw: ByteAngle::from_degrees(self.head_yaw),
                    velocity: velocity_to_packet_units(self.velocity),
                }))
            }
        }
    }

    pub fn hitbox(&self) -> Aabb<f64> {
        let dims = match &self.meta {
            EntityMeta::AreaEffectCloud(e) => [
                e.get_radius() as f64 * 2.0,
                0.5,
                e.get_radius() as f64 * 2.0,
            ],
            EntityMeta::ArmorStand(e) => {
                if e.get_marker() {
                    [0.0, 0.0, 0.0]
                } else if e.get_small() {
                    [0.5, 0.9875, 0.5]
                } else {
                    [0.5, 1.975, 0.5]
                }
            }
            EntityMeta::Arrow(_) => [0.5, 0.5, 0.5],
            EntityMeta::Axolotl(_) => [1.3, 0.6, 1.3],
            EntityMeta::Bat(_) => [0.5, 0.9, 0.5],
            EntityMeta::Bee(_) => [0.7, 0.6, 0.7], // TODO: baby size?
            EntityMeta::Blaze(_) => [0.6, 1.8, 0.6],
            EntityMeta::Boat(_) => [1.375, 0.5625, 1.375],
            EntityMeta::Cat(_) => [0.6, 0.7, 0.6],
            EntityMeta::CaveSpider(_) => [0.7, 0.5, 0.7],
            EntityMeta::Chicken(_) => [0.4, 0.7, 0.4], // TODO: baby size?
            EntityMeta::Cod(_) => [0.5, 0.3, 0.5],
            EntityMeta::Cow(_) => [0.9, 1.4, 0.9], // TODO: baby size?
            EntityMeta::Creeper(_) => [0.6, 1.7, 0.6],
            EntityMeta::Dolphin(_) => [0.9, 0.6, 0.9],
            EntityMeta::Donkey(_) => [1.5, 1.39648, 1.5], // TODO: baby size?
            EntityMeta::DragonFireball(_) => [1.0, 1.0, 1.0],
            EntityMeta::Drowned(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityMeta::ElderGuardian(_) => [1.9975, 1.9975, 1.9975],
            EntityMeta::EndCrystal(_) => [2.0, 2.0, 2.0],
            EntityMeta::EnderDragon(_) => [16.0, 8.0, 16.0],
            EntityMeta::Enderman(_) => [0.6, 2.9, 0.6],
            EntityMeta::Endermite(_) => [0.4, 0.3, 0.4],
            EntityMeta::Evoker(_) => [0.6, 1.95, 0.6],
            EntityMeta::EvokerFangs(_) => [0.5, 0.8, 0.5],
            EntityMeta::ExperienceOrb(_) => [0.5, 0.5, 0.5],
            EntityMeta::EyeOfEnder(_) => [0.25, 0.25, 0.25],
            EntityMeta::FallingBlock(_) => [0.98, 0.98, 0.98],
            EntityMeta::FireworkRocket(_) => [0.25, 0.25, 0.25],
            EntityMeta::Fox(_) => [0.6, 0.7, 0.6], // TODO: baby size?
            EntityMeta::Ghast(_) => [4.0, 4.0, 4.0],
            EntityMeta::Giant(_) => [3.6, 12.0, 3.6],
            EntityMeta::GlowItemFrame(_) => todo!("account for rotation"),
            EntityMeta::GlowSquid(_) => [0.8, 0.8, 0.8],
            EntityMeta::Goat(e) => [1.3, 0.9, 1.3], // TODO: baby size?
            EntityMeta::Guardian(_) => [0.85, 0.85, 0.85],
            EntityMeta::Hoglin(_) => [1.39648, 1.4, 1.39648], // TODO: baby size?
            EntityMeta::Horse(_) => [1.39648, 1.6, 1.39648],  // TODO: baby size?
            EntityMeta::Husk(_) => [0.6, 1.95, 0.6],          // TODO: baby size?
            EntityMeta::Illusioner(_) => [0.6, 1.95, 0.6],
            EntityMeta::IronGolem(_) => [1.4, 2.7, 1.4],
            EntityMeta::Item(_) => [0.25, 0.25, 0.25],
            EntityMeta::ItemFrame(_) => todo!("account for rotation"),
            EntityMeta::Fireball(_) => [1.0, 1.0, 1.0],
            EntityMeta::LeashKnot(_) => [0.375, 0.5, 0.375],
            EntityMeta::LightningBolt(_) => [0.0, 0.0, 0.0],
            EntityMeta::Llama(_) => [0.9, 1.87, 0.9], // TODO: baby size?
            EntityMeta::LlamaSpit(_) => [0.25, 0.25, 0.25],
            EntityMeta::MagmaCube(e) => {
                let s = e.get_size() as f64 * 0.51000005;
                [s, s, s]
            }
            EntityMeta::Marker(_) => [0.0, 0.0, 0.0],
            EntityMeta::Minecart(_) => [0.98, 0.7, 0.98],
            EntityMeta::ChestMinecart(_) => [0.98, 0.7, 0.98],
            EntityMeta::CommandBlockMinecart(_) => [0.98, 0.7, 0.98],
            EntityMeta::FurnaceMinecart(_) => [0.98, 0.7, 0.98],
            EntityMeta::HopperMinecart(_) => [0.98, 0.7, 0.98],
            EntityMeta::SpawnerMinecart(_) => [0.98, 0.7, 0.98],
            EntityMeta::TntMinecart(_) => [0.98, 0.7, 0.98],
            EntityMeta::Mule(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            EntityMeta::Mooshroom(_) => [0.9, 1.4, 0.9],    // TODO: baby size?
            EntityMeta::Ocelot(_) => [0.6, 0.7, 0.6],       // TODO: baby size?
            EntityMeta::Painting(_) => todo!("account for rotation and type"),
            EntityMeta::Panda(_) => [0.6, 0.7, 0.6], // TODO: baby size?
            EntityMeta::Parrot(_) => [0.5, 0.9, 0.5],
            EntityMeta::Phantom(_) => [0.9, 0.5, 0.9],
            EntityMeta::Pig(_) => [0.9, 0.9, 0.9], // TODO: baby size?
            EntityMeta::Piglin(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityMeta::PiglinBrute(_) => [0.6, 1.95, 0.6],
            EntityMeta::Pillager(_) => [0.6, 1.95, 0.6],
            EntityMeta::PolarBear(_) => [1.4, 1.4, 1.4], // TODO: baby size?
            EntityMeta::Tnt(_) => [0.98, 0.98, 0.98],
            EntityMeta::Pufferfish(_) => [0.7, 0.7, 0.7],
            EntityMeta::Rabbit(_) => [0.4, 0.5, 0.4], // TODO: baby size?
            EntityMeta::Ravager(_) => [1.95, 2.2, 1.95],
            EntityMeta::Salmon(_) => [0.7, 0.4, 0.7],
            EntityMeta::Sheep(_) => [0.9, 1.3, 0.9], // TODO: baby size?
            EntityMeta::Shulker(_) => [1.0, 1.0, 1.0], // TODO: how is height calculated?
            EntityMeta::ShulkerBullet(_) => [0.3125, 0.3125, 0.3125],
            EntityMeta::Silverfish(_) => [0.4, 0.3, 0.4],
            EntityMeta::Skeleton(_) => [0.6, 1.99, 0.6],
            EntityMeta::SkeletonHorse(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            EntityMeta::Slime(e) => {
                let s = 0.51000005 * e.get_size() as f64;
                [s, s, s]
            }
            EntityMeta::SmallFireball(_) => [0.3125, 0.3125, 0.3125],
            EntityMeta::SnowGolem(_) => [0.7, 1.9, 0.7],
            EntityMeta::Snowball(_) => [0.25, 0.25, 0.25],
            EntityMeta::SpectralArrow(_) => [0.5, 0.5, 0.5],
            EntityMeta::Spider(_) => [1.4, 0.9, 1.4],
            EntityMeta::Squid(_) => [0.8, 0.8, 0.8],
            EntityMeta::Stray(_) => [0.6, 1.99, 0.6],
            EntityMeta::Strider(_) => [0.9, 1.7, 0.9], // TODO: baby size?
            EntityMeta::Egg(_) => [0.25, 0.25, 0.25],
            EntityMeta::EnderPearl(_) => [0.25, 0.25, 0.25],
            EntityMeta::ExperienceBottle(_) => [0.25, 0.25, 0.25],
            EntityMeta::Potion(_) => [0.25, 0.25, 0.25],
            EntityMeta::Trident(_) => [0.5, 0.5, 0.5],
            EntityMeta::TraderLlama(_) => [0.9, 1.87, 0.9],
            EntityMeta::TropicalFish(_) => [0.5, 0.4, 0.5],
            EntityMeta::Turtle(_) => [1.2, 0.4, 1.2], // TODO: baby size?
            EntityMeta::Vex(_) => [0.4, 0.8, 0.4],
            EntityMeta::Villager(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityMeta::Vindicator(_) => [0.6, 1.95, 0.6],
            EntityMeta::WanderingTrader(_) => [0.6, 1.95, 0.6],
            EntityMeta::Witch(_) => [0.6, 1.95, 0.6],
            EntityMeta::Wither(_) => [0.9, 3.5, 0.9],
            EntityMeta::WitherSkeleton(_) => [0.7, 2.4, 0.7],
            EntityMeta::WitherSkull(_) => [0.3125, 0.3125, 0.3125],
            EntityMeta::Wolf(_) => [0.6, 0.85, 0.6], // TODO: baby size?
            EntityMeta::Zoglin(_) => [1.39648, 1.4, 1.39648], // TODO: baby size?
            EntityMeta::Zombie(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityMeta::ZombieHorse(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            EntityMeta::ZombieVillager(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityMeta::ZombifiedPiglin(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityMeta::Player(_) => [0.6, 1.8, 0.6], // TODO: changes depending on the pose.
            EntityMeta::FishingBobber(_) => [0.25, 0.25, 0.25],
        };

        aabb_from_bottom_and_size(self.new_position, dims.into())
    }
}

pub(crate) fn velocity_to_packet_units(vel: Vec3<f32>) -> Vec3<i16> {
    // The saturating cast to i16 is desirable.
    (vel * 400.0).as_()
}

impl<'a> EntityMut<'a> {
    // TODO: exposing &mut EntityMeta is unsound?
    /// Returns a mutable reference to this entity's [`EntityMeta`].
    ///
    /// **NOTE:** Never call [`std::mem::swap`] on the returned reference or any
    /// part of `EntityMeta` as this would break invariants within the
    /// library.
    pub fn meta_mut(&mut self) -> &mut EntityMeta {
        &mut self.0.meta
    }

    /// Changes the [`EntityType`] of this entity to the provided type.
    ///
    /// All metadata of this entity is reset to the default values.
    pub fn set_type(&mut self, typ: EntityType) {
        self.0.meta = EntityMeta::new(typ);
        // All metadata is lost so we must mark it as modified unconditionally.
        self.0.flags.set_type_modified(true);
    }

    /// Sets the position of this entity in the world it inhabits.
    pub fn set_position(&mut self, pos: impl Into<Vec3<f64>>) {
        self.0.new_position = pos.into();
    }

    /// Sets the yaw of this entity (in degrees).
    pub fn set_yaw(&mut self, yaw: f32) {
        if self.0.yaw != yaw {
            self.0.yaw = yaw;
            self.0.flags.set_yaw_or_pitch_modified(true);
        }
    }

    /// Sets the pitch of this entity (in degrees).
    pub fn set_pitch(&mut self, pitch: f32) {
        if self.0.pitch != pitch {
            self.0.pitch = pitch;
            self.0.flags.set_yaw_or_pitch_modified(true);
        }
    }

    /// Sets the head yaw of this entity (in degrees).
    pub fn set_head_yaw(&mut self, head_yaw: f32) {
        if self.0.head_yaw != head_yaw {
            self.0.head_yaw = head_yaw;
            self.0.flags.set_head_yaw_modified(true);
        }
    }

    pub fn set_velocity(&mut self, velocity: impl Into<Vec3<f32>>) {
        let new_vel = velocity.into();

        if self.0.velocity != new_vel {
            self.0.velocity = new_vel;
            self.0.flags.set_velocity_modified(true);
        }
    }

    pub fn set_on_ground(&mut self, on_ground: bool) {
        self.0.flags.set_on_ground(on_ground);
    }
}

pub(crate) enum EntitySpawnPacket {
    SpawnEntity(SpawnEntity),
    SpawnExperienceOrb(SpawnExperienceOrb),
    SpawnLivingEntity(SpawnLivingEntity),
    SpawnPainting(SpawnPainting),
    SpawnPlayer(SpawnPlayer),
}

impl From<EntitySpawnPacket> for ClientPlayPacket {
    fn from(pkt: EntitySpawnPacket) -> Self {
        match pkt {
            EntitySpawnPacket::SpawnEntity(pkt) => pkt.into(),
            EntitySpawnPacket::SpawnExperienceOrb(pkt) => pkt.into(),
            EntitySpawnPacket::SpawnLivingEntity(pkt) => pkt.into(),
            EntitySpawnPacket::SpawnPainting(pkt) => pkt.into(),
            EntitySpawnPacket::SpawnPlayer(pkt) => pkt.into(),
        }
    }
}

pub use types::{EntityMeta, EntityType};
