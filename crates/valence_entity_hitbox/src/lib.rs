#![doc = include_str!("../README.md")]
#![allow(clippy::type_complexity)]

use bevy_app::{App, CoreSet, Plugin};
use bevy_ecs::prelude::{Component, Entity, SystemSet};
use bevy_ecs::query::{Added, Changed, Or, With, WorldQuery};
use bevy_ecs::schedule::{IntoSystemConfig, IntoSystemConfigs, IntoSystemSetConfig};
use bevy_ecs::system::{Commands, Query};
use glam::{DVec3, UVec3, Vec3Swizzles};
use valence_core::aabb::Aabb;
use valence_core::direction::Direction;
use valence_entity::*;

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct HitboxUpdateSet;

#[cfg(feature = "hitbox_add")]
#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct HitboxComponentAddSet;

pub struct HitboxPlugin;

impl Plugin for HitboxPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(HitboxUpdateSet.in_base_set(CoreSet::PreUpdate))
            .add_systems(
                (
                    update_constant_hitbox,
                    update_warden_hitbox,
                    update_area_effect_cloud_hitbox,
                    update_armor_stand_hitbox,
                    update_passive_child_hitbox,
                    update_zombie_hitbox,
                    update_piglin_hitbox,
                    update_zoglin_hitbox,
                    update_player_hitbox,
                    update_item_frame_hitbox,
                    update_slime_hitbox,
                    update_painting_hitbox,
                    update_shulker_hitbox,
                )
                    .in_set(HitboxUpdateSet),
            );

        #[cfg(feature = "hitbox_add")]
        {
            app.configure_set(HitboxComponentAddSet.in_base_set(CoreSet::PostUpdate))
                .add_system(add_hitbox_component.in_set(HitboxComponentAddSet));
        }
    }
}

#[cfg(not(feature = "hitbox_add"))]
type HitboxFilter<T> = Or<(T, Added<Hitbox>)>; 

#[cfg(not(feature = "hitbox_add"))]
type HitboxFilterSingle<T> = Or<T, Added<Hitbox>>;

#[cfg(feature = "hitbox_add")]
type HitboxFilter<T> = Or<T>;

#[cfg(feature = "hitbox_add")]
type HitboxFilterSingle<T> = T;

#[derive(Component, Debug)]
pub struct Hitbox(Aabb);

impl Hitbox {
    pub fn get(&self) -> Aabb {
        self.0
    }

    pub(crate) fn centered(&mut self, size: DVec3) {
        self.0 = Aabb::from_bottom_size(DVec3::ZERO, size);
    }

    pub fn in_world(&self, pos: DVec3) -> Aabb {
        Aabb {
            min: self.0.min + pos,
            max: self.0.max + pos,
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub(crate) struct HitboxQueryParam {
    hitbox: &'static mut Hitbox,
}

impl<'a> HitboxQueryParamItem<'a> {
    pub(crate) fn get(self) -> &'a mut Hitbox {
        self.hitbox.into_inner()
    }
}

#[cfg(feature = "hitbox_add")]
fn add_hitbox_component(mut commands: Commands, query: Query<Entity, Added<entity::Entity>>) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert(Hitbox(Aabb::new(DVec3::ZERO, DVec3::ZERO)));
    }
}

fn update_constant_hitbox(
    mut hitbox_query: Query<
        (HitboxQueryParam, &EntityKind),
        HitboxFilterSingle<Changed<EntityKind>>,
    >,
) {
    for (hitbox, entity_kind) in hitbox_query.iter_mut() {
        let size = match *entity_kind {
            EntityKind::ALLAY => [0.6, 0.35, 0.6],
            EntityKind::CHEST_BOAT | EntityKind::BOAT => [1.375, 0.5625, 1.375],
            EntityKind::FROG => [0.5, 0.5, 0.5],
            EntityKind::TADPOLE => [0.4, 0.3, 0.4],
            EntityKind::SPECTRAL_ARROW | EntityKind::ARROW => [0.5, 0.5, 0.5],
            EntityKind::AXOLOTL => [1.3, 0.6, 1.3],
            EntityKind::BAT => [0.5, 0.9, 0.5],
            EntityKind::BLAZE => [0.6, 1.8, 0.6],
            EntityKind::CAT => [0.6, 0.7, 0.6],
            EntityKind::CAVE_SPIDER => [0.7, 0.5, 0.7],
            EntityKind::COD => [0.5, 0.3, 0.5],
            EntityKind::CREEPER => [0.6, 1.7, 0.6],
            EntityKind::DOLPHIN => [0.9, 0.6, 0.9],
            EntityKind::DRAGON_FIREBALL => [1.0, 1.0, 1.0],
            EntityKind::ELDER_GUARDIAN => [1.9975, 1.9975, 1.9975],
            EntityKind::END_CRYSTAL => [2.0, 2.0, 2.0],
            EntityKind::ENDER_DRAGON => [16.0, 8.0, 16.0],
            EntityKind::ENDERMAN => [0.6, 2.9, 0.6],
            EntityKind::ENDERMITE => [0.4, 0.3, 0.4],
            EntityKind::EVOKER => [0.6, 1.95, 0.6],
            EntityKind::EVOKER_FANGS => [0.5, 0.8, 0.5],
            EntityKind::EXPERIENCE_ORB => [0.5, 0.5, 0.5],
            EntityKind::EYE_OF_ENDER => [0.25, 0.25, 0.25],
            EntityKind::FALLING_BLOCK => [0.98, 0.98, 0.98],
            EntityKind::FIREWORK_ROCKET => [0.25, 0.25, 0.25],
            EntityKind::GHAST => [4.0, 4.0, 4.0],
            EntityKind::GIANT => [3.6, 12.0, 3.6],
            EntityKind::GLOW_SQUID | EntityKind::SQUID => [0.8, 0.8, 0.8],
            EntityKind::GUARDIAN => [0.85, 0.85, 0.85],
            EntityKind::ILLUSIONER => [0.6, 1.95, 0.6],
            EntityKind::IRON_GOLEM => [1.4, 2.7, 1.4],
            EntityKind::ITEM => [0.25, 0.25, 0.25],
            EntityKind::FIREBALL => [1.0, 1.0, 1.0],
            EntityKind::LEASH_KNOT => [0.375, 0.5, 0.375],
            EntityKind::LIGHTNING | EntityKind::MARKER => [0.0; 3],
            EntityKind::LLAMA_SPIT => [0.25, 0.25, 0.25],
            EntityKind::MINECART
            | EntityKind::CHEST_MINECART
            | EntityKind::TNT_MINECART
            | EntityKind::HOPPER_MINECART
            | EntityKind::FURNACE_MINECART
            | EntityKind::SPAWNER_MINECART
            | EntityKind::COMMAND_BLOCK_MINECART => [0.98, 0.7, 0.98],
            EntityKind::PARROT => [0.5, 0.9, 0.5],
            EntityKind::PHANTOM => [0.9, 0.5, 0.9],
            EntityKind::PIGLIN_BRUTE => [0.6, 1.95, 0.6],
            EntityKind::PILLAGER => [0.6, 1.95, 0.6],
            EntityKind::TNT => [0.98, 0.98, 0.98],
            EntityKind::PUFFERFISH => [0.7, 0.7, 0.7],
            EntityKind::RAVAGER => [1.95, 2.2, 1.95],
            EntityKind::SALMON => [0.7, 0.4, 0.7],
            EntityKind::SHULKER_BULLET => [0.3125, 0.3125, 0.3125],
            EntityKind::SILVERFISH => [0.4, 0.3, 0.4],
            EntityKind::SMALL_FIREBALL => [0.3125, 0.3125, 0.3125],
            EntityKind::SNOW_GOLEM => [0.7, 1.9, 0.7],
            EntityKind::SPIDER => [1.4, 0.9, 1.4],
            EntityKind::STRAY => [0.6, 1.99, 0.6],
            EntityKind::EGG => [0.25, 0.25, 0.25],
            EntityKind::ENDER_PEARL => [0.25, 0.25, 0.25],
            EntityKind::EXPERIENCE_BOTTLE => [0.25, 0.25, 0.25],
            EntityKind::POTION => [0.25, 0.25, 0.25],
            EntityKind::TRIDENT => [0.5, 0.5, 0.5],
            EntityKind::TRADER_LLAMA => [0.9, 1.87, 0.9],
            EntityKind::TROPICAL_FISH => [0.5, 0.4, 0.5],
            EntityKind::VEX => [0.4, 0.8, 0.4],
            EntityKind::VINDICATOR => [0.6, 1.95, 0.6],
            EntityKind::WITHER => [0.9, 3.5, 0.9],
            EntityKind::WITHER_SKELETON => [0.7, 2.4, 0.7],
            EntityKind::WITHER_SKULL => [0.3125, 0.3125, 0.3125],
            EntityKind::FISHING_BOBBER => [0.25, 0.25, 0.25],
            _ => {
                continue;
            }
        }
        .into();
        hitbox.get().centered(size);
    }
}

fn update_warden_hitbox(
    mut query: Query<
        (HitboxQueryParam, &entity::Pose),
        (
            HitboxFilterSingle<Changed<entity::Pose>>,
            With<warden::WardenEntity>,
        ),
    >,
) {
    for (hitbox, entity_pose) in query.iter_mut() {
        hitbox.get().centered(
            match entity_pose.0 {
                Pose::Emerging | Pose::Digging => [0.9, 1.0, 0.9],
                _ => [0.9, 2.9, 0.9],
            }
            .into(),
        );
    }
}

fn update_area_effect_cloud_hitbox(
    mut query: Query<
        (HitboxQueryParam, &area_effect_cloud::Radius),
        HitboxFilterSingle<Changed<area_effect_cloud::Radius>>,
    >,
) {
    for (hitbox, cloud_radius) in query.iter_mut() {
        let diameter = cloud_radius.0 as f64 * 2.0;
        hitbox.get().centered([diameter, 0.5, diameter].into());
    }
}

fn update_armor_stand_hitbox(
    mut query: Query<
        (HitboxQueryParam, &armor_stand::ArmorStandFlags),
        HitboxFilterSingle<Changed<armor_stand::ArmorStandFlags>>,
    >,
) {
    for (hitbox, stand_flags) in query.iter_mut() {
        hitbox.get().centered(
            if stand_flags.0 & 16 != 0 {
                // Marker armor stand
                [0.0; 3]
            } else if stand_flags.0 & 1 != 0 {
                // Small armor stand
                [0.5, 0.9875, 0.5]
            } else {
                [0.5, 1.975, 0.5]
            }
            .into(),
        );
    }
}

fn child_hitbox(child: bool, v: DVec3) -> DVec3 {
    if child {
        v / 2.0
    } else {
        v
    }
}

fn update_passive_child_hitbox(
    mut query: Query<
        (Entity, HitboxQueryParam, &EntityKind, &passive::Child),
        HitboxFilterSingle<Changed<passive::Child>>,
    >,
    pose_query: Query<&entity::Pose>,
) {
    for (entity, hitbox, entity_kind, child) in query.iter_mut() {
        let big_s = match *entity_kind {
            EntityKind::BEE => [0.7, 0.6, 0.7],
            EntityKind::CAMEL => [1.7, 2.375, 1.7],
            EntityKind::CHICKEN => [0.4, 0.7, 0.4],
            EntityKind::DONKEY => [1.5, 1.39648, 1.5],
            EntityKind::FOX => [0.6, 0.7, 0.6],
            EntityKind::GOAT => {
                if pose_query
                    .get(entity)
                    .ok()
                    .map(|v| v.0 == Pose::LongJumping)
                    .unwrap_or(false)
                {
                    [0.63, 0.91, 0.63]
                } else {
                    [0.9, 1.3, 0.9]
                }
            }
            EntityKind::HOGLIN => [1.39648, 1.4, 1.39648],
            EntityKind::HORSE | EntityKind::SKELETON_HORSE | EntityKind::ZOMBIE_HORSE => {
                [1.39648, 1.6, 1.39648]
            }
            EntityKind::LLAMA => [0.9, 1.87, 0.9],
            EntityKind::MULE => [1.39648, 1.6, 1.39648],
            EntityKind::MOOSHROOM => [0.9, 1.4, 0.9],
            EntityKind::OCELOT => [0.6, 0.7, 0.6],
            EntityKind::PANDA => [1.3, 1.25, 1.3],
            EntityKind::PIG => [0.9, 0.9, 0.9],
            EntityKind::POLAR_BEAR => [1.4, 1.4, 1.4],
            EntityKind::RABBIT => [0.4, 0.5, 0.4],
            EntityKind::SHEEP => [0.9, 1.3, 0.9],
            EntityKind::TURTLE => {
                hitbox.get().centered(
                    if child.0 {
                        [0.36, 0.12, 0.36]
                    } else {
                        [1.2, 0.4, 1.2]
                    }
                    .into(),
                );
                continue;
            }
            EntityKind::VILLAGER => [0.6, 1.95, 0.6],
            EntityKind::WOLF => [0.6, 0.85, 0.6],
            _ => {
                continue;
            }
        };
        hitbox.get().centered(child_hitbox(child.0, big_s.into()));
    }
}

fn update_zombie_hitbox(
    mut query: Query<(HitboxQueryParam, &zombie::Baby), HitboxFilterSingle<Changed<zombie::Baby>>>,
) {
    for (hitbox, baby) in query.iter_mut() {
        hitbox
            .get()
            .centered(child_hitbox(baby.0, [0.6, 1.95, 0.6].into()));
    }
}

fn update_piglin_hitbox(
    mut query: Query<(HitboxQueryParam, &piglin::Baby), HitboxFilterSingle<Changed<piglin::Baby>>>,
) {
    for (hitbox, baby) in query.iter_mut() {
        hitbox
            .get()
            .centered(child_hitbox(baby.0, [0.6, 1.95, 0.6].into()));
    }
}

fn update_zoglin_hitbox(
    mut query: Query<(HitboxQueryParam, &zoglin::Baby), HitboxFilterSingle<Changed<zoglin::Baby>>>,
) {
    for (hitbox, baby) in query.iter_mut() {
        hitbox
            .get()
            .centered(child_hitbox(baby.0, [1.39648, 1.4, 1.39648].into()));
    }
}

fn update_player_hitbox(
    mut query: Query<
        (HitboxQueryParam, &entity::Pose),
        (
            Or<(Changed<entity::Pose>, Added<Hitbox>)>,
            With<player::PlayerEntity>,
        ),
    >,
) {
    for (hitbox, pose) in query.iter_mut() {
        hitbox.get().centered(
            match pose.0 {
                Pose::Sleeping | Pose::Dying => [0.2, 0.2, 0.2],
                Pose::FallFlying | Pose::Swimming | Pose::SpinAttack => [0.6, 0.6, 0.6],
                Pose::Sneaking => [0.6, 1.5, 0.6],
                _ => [0.6, 1.8, 0.6],
            }
            .into(),
        );
    }
}

fn update_item_frame_hitbox(
    mut query: Query<
        (HitboxQueryParam, &item_frame::Rotation),
        HitboxFilterSingle<Changed<item_frame::Rotation>>,
    >,
) {
    for (hitbox, rotation) in query.iter_mut() {
        let mut center_pos = DVec3::splat(0.5);

        const A: f64 = 0.46875;

        match rotation.0 {
            0 => center_pos.y += A,
            1 => center_pos.y -= A,
            2 => center_pos.z += A,
            3 => center_pos.z -= A,
            4 => center_pos.x += A,
            5 => center_pos.x -= A,
            _ => center_pos.y -= A,
        }

        const BOUNDS23: DVec3 = DVec3::new(0.375, 0.375, 0.03125);

        let bounds = match rotation.0 {
            2 | 3 => BOUNDS23,
            4 | 5 => BOUNDS23.zxy(),
            _ => BOUNDS23.zxy(),
        };

        hitbox.get().0 = Aabb {
            min: center_pos - bounds,
            max: center_pos + bounds,
        }
    }
}

fn update_slime_hitbox(
    mut query: Query<
        (HitboxQueryParam, &slime::SlimeSize),
        HitboxFilterSingle<Changed<slime::SlimeSize>>,
    >,
) {
    for (hitbox, slime_size) in query.iter_mut() {
        let s = 0.5202 * slime_size.0 as f64;
        hitbox.get().centered([s, s, s].into());
    }
}

fn update_painting_hitbox(
    mut query: Query<
        (HitboxQueryParam, &painting::Variant, &Look),
        HitboxFilter<(Changed<Look>, Changed<painting::Variant>)>,
    >,
) {
    for (hitbox, painting_variant, look) in query.iter_mut() {
        let bounds: UVec3 = match painting_variant.0 {
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

        let mut center_pos = DVec3::splat(0.5);

        let (facing_x, facing_z, cc_facing_x, cc_facing_z) =
            match ((look.yaw + 45.0).rem_euclid(360.0) / 90.0) as u8 {
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

        hitbox.get().0 = Aabb {
            min: center_pos - bounds / 2.0,
            max: center_pos + bounds / 2.0,
        };
    }
}

fn update_shulker_hitbox(
    mut query: Query<
        (
            HitboxQueryParam,
            &shulker::PeekAmount,
            &shulker::AttachedFace,
        ),
        HitboxFilter<(
            Changed<shulker::PeekAmount>,
            Changed<shulker::AttachedFace>,
        )>,
    >,
) {
    use std::f64::consts::PI;

    for (hitbox, peek_amount, attached_face) in query.iter_mut() {
        let pos = DVec3::splat(0.5);
        let mut min = pos - 0.5;
        let mut max = pos + 0.5;

        let peek = 0.5 - f64::cos(peek_amount.0 as f64 * 0.01 * PI) * 0.5;

        match attached_face.0 {
            Direction::Down => max.y += peek,
            Direction::Up => min.y -= peek,
            Direction::North => max.z += peek,
            Direction::South => min.z -= peek,
            Direction::West => max.x += peek,
            Direction::East => min.x -= peek,
        }

        hitbox.get().0 = Aabb { min, max };
    }
}
