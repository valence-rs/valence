#![allow(clippy::type_complexity)]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use derive_more::Deref;
use valence_math::{Aabb, UVec3, Vec3Swizzles};
use valence_protocol::Direction;

use crate::*;

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct HitboxShapeUpdateSet;

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct HitboxComponentsAddSet;

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct HitboxUpdateSet;

pub struct HitboxPlugin;

#[derive(Resource)]
/// Settings for hitbox plugin
pub struct EntityHitboxSettings {
    /// Controls if a plugin should add hitbox component on each created entity.
    /// Otherwise you should add hitbox component by yourself in order to use
    /// it.
    pub add_hitbox_component: bool,
}

impl Default for EntityHitboxSettings {
    fn default() -> Self {
        Self {
            add_hitbox_component: true,
        }
    }
}

impl Plugin for HitboxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityHitboxSettings>()
            .configure_sets(PreUpdate, HitboxShapeUpdateSet)
            .add_systems(
                PreUpdate,
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
                ),
            )
            .configure_sets(PostUpdate, HitboxComponentsAddSet)
            .add_systems(
                PostUpdate,
                add_hitbox_component.in_set(HitboxComponentsAddSet),
            )
            .configure_sets(PreUpdate, HitboxUpdateSet.after(HitboxShapeUpdateSet))
            .add_systems(PreUpdate, update_hitbox.in_set(HitboxUpdateSet));
    }
}

/// Size of hitbox. The only way to manipulate it without losing it on the next
/// tick is using a marker entity. Marker entity's hitbox is never updated.
#[derive(Component, Debug, PartialEq, Deref)]
pub struct HitboxShape(pub Aabb);

/// Hitbox, aabb of which is calculated each tick using its position and
/// [`Hitbox`]. In order to change size of this hitbox you need to change
/// [`Hitbox`].
#[derive(Component, Debug, Deref)]
pub struct Hitbox(Aabb);

impl HitboxShape {
    pub const ZERO: HitboxShape = HitboxShape(Aabb::ZERO);

    pub fn get(&self) -> Aabb {
        self.0
    }

    pub(crate) fn centered(&mut self, size: DVec3) {
        self.0 = Aabb::from_bottom_size(DVec3::ZERO, size);
    }

    pub(crate) fn in_world(&self, pos: DVec3) -> Aabb {
        self.0 + pos
    }
}

impl Hitbox {
    pub fn get(&self) -> Aabb {
        self.0
    }
}

fn add_hitbox_component(
    settings: Res<EntityHitboxSettings>,
    mut commands: Commands,
    query: Query<(Entity, &Position), Added<entity::Entity>>,
    alt_query: Query<(Entity, &Position, &HitboxShape), Added<HitboxShape>>,
) {
    if settings.add_hitbox_component {
        for (entity, pos) in query.iter() {
            commands
                .entity(entity)
                .insert(HitboxShape::ZERO)
                .insert(Hitbox(HitboxShape::ZERO.in_world(pos.0)));
        }
    } else {
        for (entity, pos, hitbox) in alt_query.iter() {
            commands
                .entity(entity)
                .insert(Hitbox(hitbox.in_world(pos.0)));
        }
    }
}

fn update_hitbox(
    mut hitbox_query: Query<
        (&mut Hitbox, &HitboxShape, &Position),
        Or<(Changed<HitboxShape>, Changed<Position>)>,
    >,
) {
    for (mut in_world, hitbox, pos) in &mut hitbox_query {
        in_world.0 = hitbox.in_world(pos.0);
    }
}

fn update_constant_hitbox(
    mut hitbox_query: Query<
        (&mut HitboxShape, &EntityKind),
        Or<(Changed<EntityKind>, Added<HitboxShape>)>,
    >,
) {
    for (mut hitbox, entity_kind) in &mut hitbox_query {
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
            EntityKind::LIGHTNING /* | EntityKind::MARKER - marker hitbox */ => [0.0; 3],
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
        hitbox.centered(size);
    }
}

fn update_warden_hitbox(
    mut query: Query<
        (&mut HitboxShape, &entity::Pose),
        (
            Or<(Changed<entity::Pose>, Added<HitboxShape>)>,
            With<warden::WardenEntity>,
        ),
    >,
) {
    for (mut hitbox, entity_pose) in &mut query {
        hitbox.centered(
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
        (&mut HitboxShape, &area_effect_cloud::Radius),
        Or<(Changed<area_effect_cloud::Radius>, Added<HitboxShape>)>,
    >,
) {
    for (mut hitbox, cloud_radius) in &mut query {
        let diameter = f64::from(cloud_radius.0) * 2.0;
        hitbox.centered([diameter, 0.5, diameter].into());
    }
}

fn update_armor_stand_hitbox(
    mut query: Query<
        (&mut HitboxShape, &armor_stand::ArmorStandFlags),
        Or<(Changed<armor_stand::ArmorStandFlags>, Added<HitboxShape>)>,
    >,
) {
    for (mut hitbox, stand_flags) in &mut query {
        hitbox.centered(
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
        (Entity, &mut HitboxShape, &EntityKind, &passive::Child),
        Or<(Changed<passive::Child>, Added<HitboxShape>)>,
    >,
    pose_query: Query<&entity::Pose>,
) {
    for (entity, mut hitbox, entity_kind, child) in &mut query {
        let big_s = match *entity_kind {
            EntityKind::BEE => [0.7, 0.6, 0.7],
            EntityKind::CAMEL => [1.7, 2.375, 1.7],
            EntityKind::CHICKEN => [0.4, 0.7, 0.4],
            EntityKind::DONKEY => [1.5, 1.39648, 1.5],
            EntityKind::FOX => [0.6, 0.7, 0.6],
            EntityKind::GOAT => {
                if pose_query
                    .get(entity)
                    .is_ok_and(|v| v.0 == Pose::LongJumping)
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
                hitbox.centered(
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
        hitbox.centered(child_hitbox(child.0, big_s.into()));
    }
}

fn update_zombie_hitbox(
    mut query: Query<
        (&mut HitboxShape, &zombie::Baby),
        Or<(Changed<zombie::Baby>, Added<HitboxShape>)>,
    >,
) {
    for (mut hitbox, baby) in &mut query {
        hitbox.centered(child_hitbox(baby.0, [0.6, 1.95, 0.6].into()));
    }
}

fn update_piglin_hitbox(
    mut query: Query<
        (&mut HitboxShape, &piglin::Baby),
        Or<(Changed<piglin::Baby>, Added<HitboxShape>)>,
    >,
) {
    for (mut hitbox, baby) in &mut query {
        hitbox.centered(child_hitbox(baby.0, [0.6, 1.95, 0.6].into()));
    }
}

fn update_zoglin_hitbox(
    mut query: Query<
        (&mut HitboxShape, &zoglin::Baby),
        Or<(Changed<zoglin::Baby>, Added<HitboxShape>)>,
    >,
) {
    for (mut hitbox, baby) in &mut query {
        hitbox.centered(child_hitbox(baby.0, [1.39648, 1.4, 1.39648].into()));
    }
}

fn update_player_hitbox(
    mut query: Query<
        (&mut HitboxShape, &entity::Pose),
        (
            Or<(Changed<entity::Pose>, Added<HitboxShape>)>,
            With<player::PlayerEntity>,
        ),
    >,
) {
    for (mut hitbox, pose) in &mut query {
        hitbox.centered(
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
        (&mut HitboxShape, &item_frame::Rotation),
        Or<(Changed<item_frame::Rotation>, Added<HitboxShape>)>,
    >,
) {
    for (mut hitbox, rotation) in &mut query {
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

        hitbox.0 = Aabb::new(center_pos - bounds, center_pos + bounds);
    }
}

fn update_slime_hitbox(
    mut query: Query<
        (&mut HitboxShape, &slime::SlimeSize),
        Or<(Changed<slime::SlimeSize>, Added<HitboxShape>)>,
    >,
) {
    for (mut hitbox, slime_size) in &mut query {
        let s = 0.5202 * f64::from(slime_size.0);
        hitbox.centered([s, s, s].into());
    }
}

fn update_painting_hitbox(
    mut query: Query<
        (&mut HitboxShape, &painting::Variant, &Look),
        Or<(
            Changed<Look>,
            Changed<painting::Variant>,
            Added<HitboxShape>,
        )>,
    >,
) {
    for (mut hitbox, painting_variant, look) in &mut query {
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

        center_pos.x -= f64::from(facing_x) * 0.46875;
        center_pos.z -= f64::from(facing_z) * 0.46875;

        center_pos.x += f64::from(cc_facing_x) * if bounds.x % 2 == 0 { 0.5 } else { 0.0 };
        center_pos.y += if bounds.y % 2 == 0 { 0.5 } else { 0.0 };
        center_pos.z += f64::from(cc_facing_z) * if bounds.z % 2 == 0 { 0.5 } else { 0.0 };

        let bounds = match (facing_x, facing_z) {
            (1 | -1, 0) => DVec3::new(0.0625, f64::from(bounds.y), f64::from(bounds.z)),
            _ => DVec3::new(f64::from(bounds.x), f64::from(bounds.y), 0.0625),
        };

        hitbox.0 = Aabb::new(center_pos - bounds / 2.0, center_pos + bounds / 2.0);
    }
}

fn update_shulker_hitbox(
    mut query: Query<
        (
            &mut HitboxShape,
            &shulker::PeekAmount,
            &shulker::AttachedFace,
        ),
        Or<(
            Changed<shulker::PeekAmount>,
            Changed<shulker::AttachedFace>,
            Added<HitboxShape>,
        )>,
    >,
) {
    use std::f64::consts::PI;

    for (mut hitbox, peek_amount, attached_face) in &mut query {
        let pos = DVec3::splat(0.5);
        let mut min = pos - 0.5;
        let mut max = pos + 0.5;

        let peek = 0.5 - f64::cos(f64::from(peek_amount.0) * 0.01 * PI) * 0.5;

        match attached_face.0 {
            Direction::Down => max.y += peek,
            Direction::Up => min.y -= peek,
            Direction::North => max.z += peek,
            Direction::South => min.z -= peek,
            Direction::West => max.x += peek,
            Direction::East => min.x -= peek,
        }

        hitbox.0 = Aabb::new(min, max);
    }
}
