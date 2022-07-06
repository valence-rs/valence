pub mod data;
pub mod meta;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::iter::FusedIterator;
use std::num::NonZeroU32;

use bitfield_struct::bitfield;
pub use data::{EntityData, EntityKind};
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use vek::{Aabb, Vec3};

use crate::protocol::packets::play::s2c::{
    AddEntity, AddExperienceOrb, AddPlayer, S2cPlayPacket, SetEntityMetadata,
};
use crate::protocol::{ByteAngle, RawBytes, VarInt};
use crate::slotmap::{Key, SlotMap};
use crate::util::aabb_from_bottom_and_size;
use crate::WorldId;

pub struct Entities {
    sm: SlotMap<Entity>,
    uuid_to_entity: HashMap<Uuid, EntityId>,
    network_id_to_entity: HashMap<NonZeroU32, u32>,
}

impl Entities {
    pub(crate) fn new() -> Self {
        Self {
            sm: SlotMap::new(),
            uuid_to_entity: HashMap::new(),
            network_id_to_entity: HashMap::new(),
        }
    }

    /// Spawns a new entity with the default data. The new entity's [`EntityId`]
    /// is returned.
    pub fn create(&mut self, kind: EntityKind) -> (EntityId, &mut Entity) {
        self.create_with_uuid(kind, Uuid::from_bytes(rand::random()))
            .expect("UUID collision")
    }

    /// Like [`create`](Entities::create), but requires specifying the new
    /// entity's UUID.
    ///
    /// The provided UUID must not conflict with an existing entity UUID in this
    /// world. If it does, `None` is returned and the entity is not spawned.
    pub fn create_with_uuid(
        &mut self,
        kind: EntityKind,
        uuid: Uuid,
    ) -> Option<(EntityId, &mut Entity)> {
        match self.uuid_to_entity.entry(uuid) {
            Entry::Occupied(_) => None,
            Entry::Vacant(ve) => {
                let (k, e) = self.sm.insert(Entity {
                    flags: EntityFlags(0),
                    data: EntityData::new(kind),
                    world: None,
                    new_position: Vec3::default(),
                    old_position: Vec3::default(),
                    yaw: 0.0,
                    pitch: 0.0,
                    head_yaw: 0.0,
                    velocity: Vec3::default(),
                    uuid,
                });

                // TODO check for overflowing version?
                self.network_id_to_entity.insert(k.version(), k.index());

                ve.insert(EntityId(k));

                Some((EntityId(k), e))
            }
        }
    }

    pub fn delete(&mut self, entity: EntityId) -> bool {
        if let Some(e) = self.sm.remove(entity.0) {
            self.uuid_to_entity
                .remove(&e.uuid)
                .expect("UUID should have been in UUID map");

            self.network_id_to_entity
                .remove(&entity.0.version())
                .expect("network ID should have been in the network ID map");

            true
        } else {
            false
        }
    }

    pub fn retain(&mut self, mut f: impl FnMut(EntityId, &mut Entity) -> bool) {
        self.sm.retain(|k, v| {
            if f(EntityId(k), v) {
                true
            } else {
                self.uuid_to_entity
                    .remove(&v.uuid)
                    .expect("UUID should have been in UUID map");

                self.network_id_to_entity
                    .remove(&k.version())
                    .expect("network ID should have been in the network ID map");

                false
            }
        });
    }

    /// Returns the number of live entities.
    pub fn count(&self) -> usize {
        self.sm.len()
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

    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut Entity> {
        self.sm.get_mut(entity.0)
    }

    pub(crate) fn get_with_network_id(&self, network_id: i32) -> Option<EntityId> {
        let version = NonZeroU32::new(network_id as u32)?;
        let index = *self.network_id_to_entity.get(&version)?;
        Some(EntityId(Key::new(index, version)))
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (EntityId, &Entity)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (EntityId(k), v))
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (EntityId, &mut Entity)> + '_ {
        self.sm.iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (EntityId, &Entity)> + Clone + '_ {
        self.sm.par_iter().map(|(k, v)| (EntityId(k), v))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (EntityId, &mut Entity)> + '_ {
        self.sm.par_iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    pub(crate) fn update(&mut self) {
        for (_, e) in self.iter_mut() {
            e.old_position = e.new_position;
            e.data.clear_modifications();

            e.flags.set_yaw_or_pitch_modified(false);
            e.flags.set_head_yaw_modified(false);
            e.flags.set_velocity_modified(false);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct EntityId(Key);

impl EntityId {
    pub const NULL: Self = Self(Key::NULL);

    pub(crate) fn to_network_id(self) -> i32 {
        self.0.version().get() as i32
    }
}

pub struct Entity {
    flags: EntityFlags,
    data: EntityData,
    world: Option<WorldId>,
    new_position: Vec3<f64>,
    old_position: Vec3<f64>,
    yaw: f32,
    pitch: f32,
    head_yaw: f32,
    velocity: Vec3<f32>,
    uuid: Uuid,
}

/// Contains a bit for certain fields in [`Entity`] to track if they have been
/// modified.
#[bitfield(u8)]
pub(crate) struct EntityFlags {
    pub yaw_or_pitch_modified: bool,
    pub head_yaw_modified: bool,
    pub velocity_modified: bool,
    pub on_ground: bool,
    #[bits(4)]
    _pad: u8,
}

impl Entity {
    pub(crate) fn flags(&self) -> EntityFlags {
        self.flags
    }

    /// Returns a reference to this entity's [`EntityData`].
    pub fn data(&self) -> &EntityData {
        &self.data
    }

    /// Returns a mutable reference to this entity's [`EntityData`].
    pub fn data_mut(&mut self) -> &mut EntityData {
        &mut self.data
    }

    /// Returns the [`EntityKind`] of this entity.
    pub fn kind(&self) -> EntityKind {
        self.data.kind()
    }

    pub fn world(&self) -> Option<WorldId> {
        self.world
    }

    pub fn set_world(&mut self, world: impl Into<Option<WorldId>>) {
        self.world = world.into();
    }

    /// Returns the position of this entity in the world it inhabits.
    pub fn position(&self) -> Vec3<f64> {
        self.new_position
    }

    /// Sets the position of this entity in the world it inhabits.
    pub fn set_position(&mut self, pos: impl Into<Vec3<f64>>) {
        self.new_position = pos.into();
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

    /// Sets the yaw of this entity (in degrees).
    pub fn set_yaw(&mut self, yaw: f32) {
        if self.yaw != yaw {
            self.yaw = yaw;
            self.flags.set_yaw_or_pitch_modified(true);
        }
    }

    /// Gets the pitch of this entity (in degrees).
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Sets the pitch of this entity (in degrees).
    pub fn set_pitch(&mut self, pitch: f32) {
        if self.pitch != pitch {
            self.pitch = pitch;
            self.flags.set_yaw_or_pitch_modified(true);
        }
    }

    /// Gets the head yaw of this entity (in degrees).
    pub fn head_yaw(&self) -> f32 {
        self.head_yaw
    }

    /// Sets the head yaw of this entity (in degrees).
    pub fn set_head_yaw(&mut self, head_yaw: f32) {
        if self.head_yaw != head_yaw {
            self.head_yaw = head_yaw;
            self.flags.set_head_yaw_modified(true);
        }
    }

    /// Gets the velocity of this entity in meters per second.
    pub fn velocity(&self) -> Vec3<f32> {
        self.velocity
    }

    pub fn set_velocity(&mut self, velocity: impl Into<Vec3<f32>>) {
        let new_vel = velocity.into();

        if self.velocity != new_vel {
            self.velocity = new_vel;
            self.flags.set_velocity_modified(true);
        }
    }

    pub fn on_ground(&self) -> bool {
        self.flags.on_ground()
    }

    pub fn set_on_ground(&mut self, on_ground: bool) {
        self.flags.set_on_ground(on_ground);
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn hitbox(&self) -> Aabb<f64> {
        let dims = match &self.data {
            EntityData::Allay(_) => [0.6, 0.35, 0.6],
            EntityData::ChestBoat(_) => [1.375, 0.5625, 1.375],
            EntityData::Frog(_) => [0.5, 0.5, 0.5],
            EntityData::Tadpole(_) => [0.4, 0.3, 0.4],
            EntityData::Warden(_) => [0.9, 2.9, 0.9],
            EntityData::AreaEffectCloud(e) => [
                e.get_radius() as f64 * 2.0,
                0.5,
                e.get_radius() as f64 * 2.0,
            ],
            EntityData::ArmorStand(e) => {
                if e.get_marker() {
                    [0.0, 0.0, 0.0]
                } else if e.get_small() {
                    [0.5, 0.9875, 0.5]
                } else {
                    [0.5, 1.975, 0.5]
                }
            }
            EntityData::Arrow(_) => [0.5, 0.5, 0.5],
            EntityData::Axolotl(_) => [1.3, 0.6, 1.3],
            EntityData::Bat(_) => [0.5, 0.9, 0.5],
            EntityData::Bee(_) => [0.7, 0.6, 0.7], // TODO: baby size?
            EntityData::Blaze(_) => [0.6, 1.8, 0.6],
            EntityData::Boat(_) => [1.375, 0.5625, 1.375],
            EntityData::Cat(_) => [0.6, 0.7, 0.6],
            EntityData::CaveSpider(_) => [0.7, 0.5, 0.7],
            EntityData::Chicken(_) => [0.4, 0.7, 0.4], // TODO: baby size?
            EntityData::Cod(_) => [0.5, 0.3, 0.5],
            EntityData::Cow(_) => [0.9, 1.4, 0.9], // TODO: baby size?
            EntityData::Creeper(_) => [0.6, 1.7, 0.6],
            EntityData::Dolphin(_) => [0.9, 0.6, 0.9],
            EntityData::Donkey(_) => [1.5, 1.39648, 1.5], // TODO: baby size?
            EntityData::DragonFireball(_) => [1.0, 1.0, 1.0],
            EntityData::Drowned(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityData::ElderGuardian(_) => [1.9975, 1.9975, 1.9975],
            EntityData::EndCrystal(_) => [2.0, 2.0, 2.0],
            EntityData::EnderDragon(_) => [16.0, 8.0, 16.0],
            EntityData::Enderman(_) => [0.6, 2.9, 0.6],
            EntityData::Endermite(_) => [0.4, 0.3, 0.4],
            EntityData::Evoker(_) => [0.6, 1.95, 0.6],
            EntityData::EvokerFangs(_) => [0.5, 0.8, 0.5],
            EntityData::ExperienceOrb(_) => [0.5, 0.5, 0.5],
            EntityData::EyeOfEnder(_) => [0.25, 0.25, 0.25],
            EntityData::FallingBlock(_) => [0.98, 0.98, 0.98],
            EntityData::FireworkRocket(_) => [0.25, 0.25, 0.25],
            EntityData::Fox(_) => [0.6, 0.7, 0.6], // TODO: baby size?
            EntityData::Ghast(_) => [4.0, 4.0, 4.0],
            EntityData::Giant(_) => [3.6, 12.0, 3.6],
            EntityData::GlowItemFrame(_) => todo!("account for rotation"),
            EntityData::GlowSquid(_) => [0.8, 0.8, 0.8],
            EntityData::Goat(_) => [1.3, 0.9, 1.3], // TODO: baby size?
            EntityData::Guardian(_) => [0.85, 0.85, 0.85],
            EntityData::Hoglin(_) => [1.39648, 1.4, 1.39648], // TODO: baby size?
            EntityData::Horse(_) => [1.39648, 1.6, 1.39648],  // TODO: baby size?
            EntityData::Husk(_) => [0.6, 1.95, 0.6],          // TODO: baby size?
            EntityData::Illusioner(_) => [0.6, 1.95, 0.6],
            EntityData::IronGolem(_) => [1.4, 2.7, 1.4],
            EntityData::Item(_) => [0.25, 0.25, 0.25],
            EntityData::ItemFrame(_) => todo!("account for rotation"),
            EntityData::Fireball(_) => [1.0, 1.0, 1.0],
            EntityData::LeashKnot(_) => [0.375, 0.5, 0.375],
            EntityData::LightningBolt(_) => [0.0, 0.0, 0.0],
            EntityData::Llama(_) => [0.9, 1.87, 0.9], // TODO: baby size?
            EntityData::LlamaSpit(_) => [0.25, 0.25, 0.25],
            EntityData::MagmaCube(e) => {
                let s = e.get_size() as f64 * 0.51000005;
                [s, s, s]
            }
            EntityData::Marker(_) => [0.0, 0.0, 0.0],
            EntityData::Minecart(_) => [0.98, 0.7, 0.98],
            EntityData::ChestMinecart(_) => [0.98, 0.7, 0.98],
            EntityData::CommandBlockMinecart(_) => [0.98, 0.7, 0.98],
            EntityData::FurnaceMinecart(_) => [0.98, 0.7, 0.98],
            EntityData::HopperMinecart(_) => [0.98, 0.7, 0.98],
            EntityData::SpawnerMinecart(_) => [0.98, 0.7, 0.98],
            EntityData::TntMinecart(_) => [0.98, 0.7, 0.98],
            EntityData::Mule(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            EntityData::Mooshroom(_) => [0.9, 1.4, 0.9],    // TODO: baby size?
            EntityData::Ocelot(_) => [0.6, 0.7, 0.6],       // TODO: baby size?
            EntityData::Painting(_) => todo!("account for rotation and type"),
            EntityData::Panda(_) => [0.6, 0.7, 0.6], // TODO: baby size?
            EntityData::Parrot(_) => [0.5, 0.9, 0.5],
            EntityData::Phantom(_) => [0.9, 0.5, 0.9],
            EntityData::Pig(_) => [0.9, 0.9, 0.9], // TODO: baby size?
            EntityData::Piglin(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityData::PiglinBrute(_) => [0.6, 1.95, 0.6],
            EntityData::Pillager(_) => [0.6, 1.95, 0.6],
            EntityData::PolarBear(_) => [1.4, 1.4, 1.4], // TODO: baby size?
            EntityData::Tnt(_) => [0.98, 0.98, 0.98],
            EntityData::Pufferfish(_) => [0.7, 0.7, 0.7],
            EntityData::Rabbit(_) => [0.4, 0.5, 0.4], // TODO: baby size?
            EntityData::Ravager(_) => [1.95, 2.2, 1.95],
            EntityData::Salmon(_) => [0.7, 0.4, 0.7],
            EntityData::Sheep(_) => [0.9, 1.3, 0.9], // TODO: baby size?
            EntityData::Shulker(_) => [1.0, 1.0, 1.0], // TODO: how is height calculated?
            EntityData::ShulkerBullet(_) => [0.3125, 0.3125, 0.3125],
            EntityData::Silverfish(_) => [0.4, 0.3, 0.4],
            EntityData::Skeleton(_) => [0.6, 1.99, 0.6],
            EntityData::SkeletonHorse(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            EntityData::Slime(e) => {
                let s = 0.51000005 * e.get_size() as f64;
                [s, s, s]
            }
            EntityData::SmallFireball(_) => [0.3125, 0.3125, 0.3125],
            EntityData::SnowGolem(_) => [0.7, 1.9, 0.7],
            EntityData::Snowball(_) => [0.25, 0.25, 0.25],
            EntityData::SpectralArrow(_) => [0.5, 0.5, 0.5],
            EntityData::Spider(_) => [1.4, 0.9, 1.4],
            EntityData::Squid(_) => [0.8, 0.8, 0.8],
            EntityData::Stray(_) => [0.6, 1.99, 0.6],
            EntityData::Strider(_) => [0.9, 1.7, 0.9], // TODO: baby size?
            EntityData::Egg(_) => [0.25, 0.25, 0.25],
            EntityData::EnderPearl(_) => [0.25, 0.25, 0.25],
            EntityData::ExperienceBottle(_) => [0.25, 0.25, 0.25],
            EntityData::Potion(_) => [0.25, 0.25, 0.25],
            EntityData::Trident(_) => [0.5, 0.5, 0.5],
            EntityData::TraderLlama(_) => [0.9, 1.87, 0.9],
            EntityData::TropicalFish(_) => [0.5, 0.4, 0.5],
            EntityData::Turtle(_) => [1.2, 0.4, 1.2], // TODO: baby size?
            EntityData::Vex(_) => [0.4, 0.8, 0.4],
            EntityData::Villager(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityData::Vindicator(_) => [0.6, 1.95, 0.6],
            EntityData::WanderingTrader(_) => [0.6, 1.95, 0.6],
            EntityData::Witch(_) => [0.6, 1.95, 0.6],
            EntityData::Wither(_) => [0.9, 3.5, 0.9],
            EntityData::WitherSkeleton(_) => [0.7, 2.4, 0.7],
            EntityData::WitherSkull(_) => [0.3125, 0.3125, 0.3125],
            EntityData::Wolf(_) => [0.6, 0.85, 0.6], // TODO: baby size?
            EntityData::Zoglin(_) => [1.39648, 1.4, 1.39648], // TODO: baby size?
            EntityData::Zombie(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityData::ZombieHorse(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            EntityData::ZombieVillager(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityData::ZombifiedPiglin(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityData::Player(_) => [0.6, 1.8, 0.6], // TODO: changes depending on the pose.
            EntityData::FishingBobber(_) => [0.25, 0.25, 0.25],
        };

        aabb_from_bottom_and_size(self.new_position, dims.into())
    }

    /// Gets the metadata packet to send to clients after this entity has been
    /// spawned.
    ///
    /// Is `None` if there is no initial metadata.
    pub(crate) fn initial_metadata_packet(&self, this_id: EntityId) -> Option<SetEntityMetadata> {
        self.data.initial_metadata().map(|meta| SetEntityMetadata {
            entity_id: VarInt(this_id.to_network_id()),
            metadata: RawBytes(meta),
        })
    }

    /// Gets the metadata packet to send to clients when the entity is modified.
    ///
    /// Is `None` if this entity's metadata has not been modified.
    pub(crate) fn updated_metadata_packet(&self, this_id: EntityId) -> Option<SetEntityMetadata> {
        self.data.updated_metadata().map(|meta| SetEntityMetadata {
            entity_id: VarInt(this_id.to_network_id()),
            metadata: RawBytes(meta),
        })
    }

    pub(crate) fn spawn_packet(&self, this_id: EntityId) -> Option<EntitySpawnPacket> {
        match &self.data {
            EntityData::Marker(_) => None,
            EntityData::ExperienceOrb(_) => {
                Some(EntitySpawnPacket::SpawnExperienceOrb(AddExperienceOrb {
                    entity_id: VarInt(this_id.to_network_id()),
                    position: self.new_position,
                    count: 0, // TODO
                }))
            }
            EntityData::Player(_) => Some(EntitySpawnPacket::SpawnPlayer(AddPlayer {
                entity_id: VarInt(this_id.to_network_id()),
                player_uuid: self.uuid,
                position: self.new_position,
                yaw: ByteAngle::from_degrees(self.yaw),
                pitch: ByteAngle::from_degrees(self.pitch),
            })),
            _ => Some(EntitySpawnPacket::SpawnEntity(AddEntity {
                entity_id: VarInt(this_id.to_network_id()),
                object_uuid: self.uuid,
                kind: VarInt(self.kind() as i32),
                position: self.new_position,
                pitch: ByteAngle::from_degrees(self.pitch),
                yaw: ByteAngle::from_degrees(self.yaw),
                head_yaw: ByteAngle::from_degrees(self.head_yaw),
                data: VarInt(1), // TODO
                velocity: velocity_to_packet_units(self.velocity),
            })),
        }
    }
}

pub(crate) fn velocity_to_packet_units(vel: Vec3<f32>) -> Vec3<i16> {
    // The saturating cast to i16 is desirable.
    (vel * 400.0).as_()
}

pub(crate) enum EntitySpawnPacket {
    SpawnEntity(AddEntity),
    SpawnExperienceOrb(AddExperienceOrb),
    SpawnPlayer(AddPlayer),
}

impl From<EntitySpawnPacket> for S2cPlayPacket {
    fn from(pkt: EntitySpawnPacket) -> Self {
        match pkt {
            EntitySpawnPacket::SpawnEntity(pkt) => pkt.into(),
            EntitySpawnPacket::SpawnExperienceOrb(pkt) => pkt.into(),
            EntitySpawnPacket::SpawnPlayer(pkt) => pkt.into(),
        }
    }
}
