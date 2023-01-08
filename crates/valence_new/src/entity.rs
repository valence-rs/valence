use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::Range;

use bevy_ecs::prelude::*;
pub use data::{EntityKind, TrackedData};
use glam::{DVec3, UVec3, Vec3};
use rustc_hash::FxHashMap;
use tracing::warn;
use uuid::Uuid;
use valence_protocol::entity_meta::{Facing, PaintingKind, Pose};
use valence_protocol::packets::s2c::play::{
    EntityAnimationS2c, SetEntityMetadata, SetEntityVelocity, SetHeadRotation, SpawnEntity,
    SpawnExperienceOrb, SpawnPlayer, TeleportEntity, UpdateEntityPosition,
    UpdateEntityPositionAndRotation, UpdateEntityRotation,
};
use valence_protocol::{ByteAngle, RawBytes, VarInt};

use crate::config::DEFAULT_TPS;
use crate::math::Aabb;
use crate::packet::WritePacket;
use crate::{Despawned, NULL_ENTITY};

pub mod data;

/// A [`Resource`] which maintains information about all the [`McEntity`]
/// components on the server.
#[derive(Resource)]
pub struct McEntityManager {
    protocol_id_to_entity: FxHashMap<i32, Entity>,
    next_protocol_id: i32,
}

impl McEntityManager {
    pub(crate) fn new() -> Self {
        Self {
            protocol_id_to_entity: HashMap::default(),
            next_protocol_id: 1,
        }
    }

    /// Gets the [`Entity`] of the [`McEntity`] with the given protocol ID.
    pub fn get_with_protocol_id(&self, id: i32) -> Option<Entity> {
        self.protocol_id_to_entity.get(&id).cloned()
    }
}

/// Sets the protocol ID of new entities.
pub(crate) fn init_entities(
    mut entities: Query<&mut McEntity, Added<McEntity>>,
    mut manager: ResMut<McEntityManager>,
) {
    for mut entity in &mut entities {
        if manager.next_protocol_id == 0 {
            warn!("entity protocol ID overflow");
            manager.next_protocol_id = 1;
        }

        entity.protocol_id = manager.next_protocol_id;
        manager.next_protocol_id = manager.next_protocol_id.wrapping_add(1);
    }
}

/// Removes despawned entities from the entity manager.
pub(crate) fn deinit_despawned_entities(
    entities: Query<&mut McEntity, With<Despawned>>,
    mut manager: ResMut<McEntityManager>,
) {
    for entity in &entities {
        manager.protocol_id_to_entity.remove(&entity.protocol_id);
    }
}

pub(crate) fn update_entities(mut entities: Query<&mut McEntity, Changed<McEntity>>) {
    for mut entity in &mut entities {
        entity.old_position = entity.position;
        entity.old_instance = entity.instance;
        entity.variants.clear_modifications();
        // TODO: clear event/animation flags.
        entity.yaw_or_pitch_modified = false;
        entity.head_yaw_modified = false;
        entity.velocity_modified = false;
    }
}

pub(crate) fn check_entity_invariants(removed: RemovedComponents<McEntity>) {
    for entity in &removed {
        warn!(
            entity = ?entity,
            "A `McEntity` component was removed from the world directly. You must use the \
             `Despawned` marker component instead."
        );
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
    variants: TrackedData,
    protocol_id: i32,
    uuid: Uuid,
    /// The range of bytes in the partition cell containing this entity's update
    /// packets.
    pub(crate) self_update_range: Range<usize>,
    // events: Vec<EntityEvent>, // TODO: store this info in bits?
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

impl McEntity {
    /// Creates a new [`McEntity`] component with a random UUID.
    ///
    /// - `kind`: The type of Minecraft entity this should be.
    /// - `instance`: The [`Instance`] this entity will be located in.
    pub fn new(kind: EntityKind, instance: Entity) -> Self {
        Self::with_uuid(kind, instance, Uuid::from_u128(rand::random()))
    }

    /// Like [`Self::new`], but allows specifying the UUID of the entity.
    pub fn with_uuid(kind: EntityKind, instance: Entity, uuid: Uuid) -> Self {
        Self {
            variants: TrackedData::new(kind),
            self_update_range: 0..0,
            instance,
            old_instance: NULL_ENTITY,
            position: DVec3::ZERO,
            old_position: DVec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            yaw_or_pitch_modified: false,
            head_yaw: 0.0,
            head_yaw_modified: false,
            velocity: Vec3::ZERO,
            velocity_modified: false,
            protocol_id: 0,
            uuid,
            on_ground: false,
        }
    }

    /// Returns a reference to this entity's tracked data.
    pub fn data(&self) -> &TrackedData {
        &self.variants
    }

    /// Returns a mutable reference to this entity's tracked data.
    pub fn data_mut(&mut self) -> &mut TrackedData {
        &mut self.variants
    }

    /// Gets the [`EntityKind`] of this entity.
    pub fn kind(&self) -> EntityKind {
        self.variants.kind()
    }

    /*
    /// Triggers an entity event for this entity.
    pub fn push_event(&mut self, event: EntityEvent) {
        self.events.push(event);
    }*/

    /// Returns a handle to the [`Instance`] this entity is located in.
    pub fn instance(&self) -> Entity {
        self.instance
    }

    pub fn old_instance(&self) -> Entity {
        self.old_instance
    }

    /// Gets the UUID of this entity.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Returns the raw protocol ID of this entity. IDs for new entities are not
    /// initialized until the end of the tick.
    pub fn protocol_id(&self) -> i32 {
        self.protocol_id
    }

    /// Gets the position of this entity in the world it inhabits.
    ///
    /// The position of an entity is located on the bottom of its
    /// hitbox and not the center.
    pub fn position(&self) -> DVec3 {
        self.position
    }

    /// Sets the position of this entity in the world it inhabits.
    ///
    /// The position of an entity is located on the bottom of its
    /// hitbox and not the center.
    pub fn set_position(&mut self, pos: impl Into<DVec3>) {
        self.position = pos.into();
    }

    /// Returns the position of this entity as it existed at the end of the
    /// previous tick.
    pub(crate) fn old_position(&self) -> DVec3 {
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
            self.yaw_or_pitch_modified = true;
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
            self.yaw_or_pitch_modified = true;
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
            self.head_yaw_modified = true;
        }
    }

    /// Gets the velocity of this entity in meters per second.
    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    /// Sets the velocity of this entity in meters per second.
    pub fn set_velocity(&mut self, velocity: impl Into<Vec3>) {
        let new_vel = velocity.into();

        if self.velocity != new_vel {
            self.velocity = new_vel;
            self.velocity_modified = true;
        }
    }

    /// Gets the value of the "on ground" flag.
    pub fn on_ground(&self) -> bool {
        self.on_ground
    }

    /// Sets the value of the "on ground" flag.
    pub fn set_on_ground(&mut self, on_ground: bool) {
        self.on_ground = on_ground;
        // TODO: on ground modified flag?
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
    pub fn hitbox(&self) -> Aabb {
        fn baby(is_baby: bool, adult_hitbox: [f64; 3]) -> [f64; 3] {
            if is_baby {
                adult_hitbox.map(|a| a / 2.0)
            } else {
                adult_hitbox
            }
        }

        fn item_frame(pos: DVec3, rotation: i32) -> Aabb {
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

            let bounds = DVec3::from(match rotation {
                0 | 1 => [0.75, 0.0625, 0.75],
                2 | 3 => [0.75, 0.75, 0.0625],
                4 | 5 => [0.0625, 0.75, 0.75],
                _ => [0.75, 0.0625, 0.75],
            });

            Aabb::new_unchecked(center_pos - bounds / 2.0, center_pos + bounds / 2.0)
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
                let bounds: UVec3 = match e.get_variant() {
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
                    (1, 0) | (-1, 0) => DVec3::new(0.0625, bounds.y as f64, bounds.z as f64),
                    _ => DVec3::new(bounds.x as f64, bounds.y as f64, 0.0625),
                };

                return Aabb::new_unchecked(center_pos - bounds / 2.0, center_pos + bounds / 2.0);
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

                return Aabb::new_unchecked(min, max);
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

        Aabb::from_bottom_size(self.position, dimensions)
    }

    /// Sends the appropriate packets to initialize the entity. This will spawn
    /// the entity and initialize tracked data.
    pub(crate) fn write_init_packets(
        &self,
        send: &mut impl WritePacket,
        position: DVec3,
        scratch: &mut Vec<u8>,
    ) -> anyhow::Result<()> {
        let with_object_data = |data| SpawnEntity {
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

        match &self.variants {
            TrackedData::Marker(_) => {}
            TrackedData::ExperienceOrb(_) => send.write_packet(&SpawnExperienceOrb {
                entity_id: VarInt(self.protocol_id),
                position: position.to_array(),
                count: 0, // TODO
            })?,
            TrackedData::Player(_) => {
                send.write_packet(&SpawnPlayer {
                    entity_id: VarInt(self.protocol_id),
                    player_uuid: self.uuid,
                    position: position.to_array(),
                    yaw: ByteAngle::from_degrees(self.yaw),
                    pitch: ByteAngle::from_degrees(self.pitch),
                })?;

                // Player spawn packet doesn't include head yaw for some reason.
                send.write_packet(&SetHeadRotation {
                    entity_id: VarInt(self.protocol_id),
                    head_yaw: ByteAngle::from_degrees(self.head_yaw),
                })?;
            }
            TrackedData::ItemFrame(e) => send.write_packet(&with_object_data(e.get_rotation()))?,
            TrackedData::GlowItemFrame(e) => {
                send.write_packet(&with_object_data(e.get_rotation()))?
            }

            TrackedData::Painting(_) => send.write_packet(&with_object_data(
                match ((self.yaw + 45.0).rem_euclid(360.0) / 90.0) as u8 {
                    0 => 3,
                    1 => 4,
                    2 => 2,
                    _ => 5,
                },
            ))?,
            // TODO: set block state ID for falling block.
            TrackedData::FallingBlock(_) => send.write_packet(&with_object_data(1))?,
            TrackedData::FishingBobber(e) => {
                send.write_packet(&with_object_data(e.get_hook_entity_id()))?
            }
            TrackedData::Warden(e) => {
                send.write_packet(&with_object_data((e.get_pose() == Pose::Emerging).into()))?
            }
            _ => send.write_packet(&with_object_data(0))?,
        }

        scratch.clear();
        self.variants.write_initial_tracked_data(scratch);
        if !scratch.is_empty() {
            send.write_packet(&SetEntityMetadata {
                entity_id: VarInt(self.protocol_id),
                metadata: RawBytes(scratch),
            })?;
        }

        Ok(())
    }

    /// Writes the appropriate packets to update the entity (Position, tracked
    /// data, events, animations).
    pub(crate) fn write_update_packets(
        &self,
        mut writer: impl WritePacket,
        scratch: &mut Vec<u8>,
    ) -> anyhow::Result<()> {
        let entity_id = VarInt(self.protocol_id);

        let position_delta = self.position - self.old_position;
        let needs_teleport = position_delta.abs().max_element() >= 8.0;
        let changed_position = self.position != self.old_position;

        if changed_position && !needs_teleport && self.yaw_or_pitch_modified {
            writer.write_packet(&UpdateEntityPositionAndRotation {
                entity_id,
                delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                yaw: ByteAngle::from_degrees(self.yaw),
                pitch: ByteAngle::from_degrees(self.pitch),
                on_ground: self.on_ground,
            })?;
        } else {
            if changed_position && !needs_teleport {
                writer.write_packet(&UpdateEntityPosition {
                    entity_id,
                    delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                    on_ground: self.on_ground,
                })?;
            }

            if self.yaw_or_pitch_modified {
                writer.write_packet(&UpdateEntityRotation {
                    entity_id,
                    yaw: ByteAngle::from_degrees(self.yaw),
                    pitch: ByteAngle::from_degrees(self.pitch),
                    on_ground: self.on_ground,
                })?;
            }
        }

        if needs_teleport {
            writer.write_packet(&TeleportEntity {
                entity_id,
                position: self.position.to_array(),
                yaw: ByteAngle::from_degrees(self.yaw),
                pitch: ByteAngle::from_degrees(self.pitch),
                on_ground: self.on_ground,
            })?;
        }

        if self.velocity_modified {
            writer.write_packet(&SetEntityVelocity {
                entity_id,
                velocity: velocity_to_packet_units(self.velocity),
            })?;
        }

        if self.head_yaw_modified {
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

        // TODO
        /*
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
        }*/

        Ok(())
    }
}

#[inline]
pub(crate) fn velocity_to_packet_units(vel: Vec3) -> [i16; 3] {
    // The saturating casts to i16 are desirable.
    (8000.0 / DEFAULT_TPS as f32 * vel)
        .to_array()
        .map(|v| v as i16)
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
