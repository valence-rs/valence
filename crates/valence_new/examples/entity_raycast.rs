use glam::Vec3;
use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::default_event_handler;
use valence_new::math::from_yaw_and_pitch;
use valence_new::prelude::*;
use valence_protocol::entity_meta::{Facing, PaintingKind, Pose};
use valence_spatial_index::bvh::Bvh;
use valence_spatial_index::{RaycastHit, SpatialIndex, WithAabb};

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -5);
const PLAYER_EYE_HEIGHT: f64 = 1.62;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(rebuild_bvh)
        .add_system(raycast)
        .add_system(manage_cursors)
        .run();
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    instance.set_block_state(SPAWN_POS, BlockState::BEDROCK);

    let raycaster = Raycaster::new();
    let instance_ent = world.spawn((instance, raycaster)).id();

    // ==== Item Frames ==== //
    let mut e = McEntity::new(EntityKind::ItemFrame, instance_ent);
    if let TrackedData::ItemFrame(i) = e.data_mut() {
        i.set_rotation(Facing::North as i32);
    }
    e.set_position([2.0, 102.0, 0.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::ItemFrame, instance_ent);
    if let TrackedData::ItemFrame(i) = e.data_mut() {
        i.set_rotation(Facing::East as i32);
    }
    e.set_position([3.0, 102.0, 0.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::GlowItemFrame, instance_ent);
    if let TrackedData::GlowItemFrame(i) = e.data_mut() {
        i.set_rotation(Facing::South as i32);
    }
    e.set_position([4.0, 102.0, 0.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::GlowItemFrame, instance_ent);
    if let TrackedData::GlowItemFrame(i) = e.data_mut() {
        i.set_rotation(Facing::West as i32);
    }
    e.set_position([5.0, 102.0, 0.0]);
    world.spawn(e);

    // ==== Paintings ==== //
    let mut e = McEntity::new(EntityKind::Painting, instance_ent);
    if let TrackedData::Painting(p) = e.data_mut() {
        p.set_variant(PaintingKind::Pigscene);
    }
    e.set_yaw(180.0);
    e.set_position([0.0, 102.0, 0.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Painting, instance_ent);
    if let TrackedData::Painting(p) = e.data_mut() {
        p.set_variant(PaintingKind::DonkeyKong);
    }
    e.set_yaw(90.0);
    e.set_position([-4.0, 102.0, 0.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Painting, instance_ent);
    if let TrackedData::Painting(p) = e.data_mut() {
        p.set_variant(PaintingKind::Void);
    }
    e.set_position([-6.0, 102.0, 0.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Painting, instance_ent);
    if let TrackedData::Painting(p) = e.data_mut() {
        p.set_variant(PaintingKind::Aztec);
    }
    e.set_yaw(270.0);
    e.set_position([-7.0, 102.0, 0.0]);
    world.spawn(e);

    // ==== Shulkers ==== //
    let mut e = McEntity::new(EntityKind::Shulker, instance_ent);
    if let TrackedData::Shulker(s) = e.data_mut() {
        s.set_peek_amount(100);
        s.set_attached_face(Facing::West);
    }
    e.set_position([-4.0, 102.0, -8.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Shulker, instance_ent);
    if let TrackedData::Shulker(s) = e.data_mut() {
        s.set_peek_amount(75);
        s.set_attached_face(Facing::Up);
    }
    e.set_position([-1.0, 102.0, -8.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Shulker, instance_ent);
    if let TrackedData::Shulker(s) = e.data_mut() {
        s.set_peek_amount(50);
        s.set_attached_face(Facing::Down);
    }
    e.set_position([2.0, 102.0, -8.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Shulker, instance_ent);
    if let TrackedData::Shulker(s) = e.data_mut() {
        s.set_peek_amount(25);
        s.set_attached_face(Facing::East);
    }
    e.set_position([5.0, 102.0, -8.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Shulker, instance_ent);
    if let TrackedData::Shulker(s) = e.data_mut() {
        s.set_peek_amount(0);
        s.set_attached_face(Facing::North);
    }
    e.set_position([8.0, 102.0, -8.0]);
    world.spawn(e);

    // ==== Slimes ==== //
    let mut e = McEntity::new(EntityKind::Slime, instance_ent);
    if let TrackedData::Slime(s) = e.data_mut() {
        s.set_slime_size(30);
    }
    e.set_yaw(180.0);
    e.set_head_yaw(180.0);
    e.set_position([12.0, 102.0, 10.0]);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::MagmaCube, instance_ent);
    if let TrackedData::MagmaCube(m) = e.data_mut() {
        m.set_slime_size(30);
    }
    e.set_yaw(180.0);
    e.set_head_yaw(180.0);
    e.set_position([-12.0, 102.0, 10.0]);
    world.spawn(e);

    // ==== Sheep ==== //
    let mut e = McEntity::new(EntityKind::Sheep, instance_ent);
    if let TrackedData::Sheep(s) = e.data_mut() {
        s.set_color(6);
        s.set_child(true);
    }
    e.set_position([-5.0, 101.0, -4.5]);
    e.set_yaw(270.0);
    e.set_head_yaw(270.0);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Sheep, instance_ent);
    if let TrackedData::Sheep(s) = e.data_mut() {
        s.set_color(6);
    }
    e.set_position([5.0, 101.0, -4.5]);
    e.set_yaw(90.0);
    e.set_head_yaw(90.0);
    world.spawn(e);

    // ==== Players ==== //
    let player_poses = [
        Pose::Standing,
        Pose::Sneaking,
        Pose::FallFlying,
        Pose::Sleeping,
        Pose::Swimming,
        Pose::SpinAttack,
        Pose::Dying,
    ];

    for (i, pose) in player_poses.into_iter().enumerate() {
        let mut e = McEntity::new(EntityKind::Player, instance_ent);
        if let TrackedData::Player(p) = e.data_mut() {
            p.set_pose(pose);
        }
        e.set_position([-3.0 + i as f64 * 2.0, 104.0, -9.0]);
        world.spawn(e);
    }

    // ==== Warden ==== //
    let mut e = McEntity::new(EntityKind::Warden, instance_ent);
    e.set_position([-7.0, 102.0, -4.5]);
    e.set_yaw(270.0);
    e.set_head_yaw(270.0);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Warden, instance_ent);
    if let TrackedData::Warden(w) = e.data_mut() {
        w.set_pose(Pose::Emerging);
    }
    e.set_position([-7.0, 102.0, -6.5]);
    e.set_yaw(270.0);
    e.set_head_yaw(270.0);
    world.spawn(e);

    // ==== Goat ==== //
    let mut e = McEntity::new(EntityKind::Goat, instance_ent);
    e.set_position([5.0, 103.0, -4.5]);
    e.set_yaw(270.0);
    e.set_head_yaw(90.0);
    world.spawn(e);

    let mut e = McEntity::new(EntityKind::Goat, instance_ent);
    if let TrackedData::Goat(g) = e.data_mut() {
        g.set_pose(Pose::LongJumping);
    }
    e.set_position([5.0, 103.0, -3.5]);
    e.set_yaw(270.0);
    e.set_head_yaw(90.0);
    world.spawn(e);

    // ==== Giant ==== //
    let mut e = McEntity::new(EntityKind::Giant, instance_ent);
    e.set_position([20.0, 101.0, -5.0]);
    e.set_yaw(270.0);
    e.set_head_yaw(90.0);
    world.spawn(e);
}

fn init_clients(
    mut commands: Commands,
    mut clients: Query<(Entity, &mut Client), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for (ent, mut client) in &mut clients {
        client.set_position([
            SPAWN_POS.x as f64 + 0.5,
            SPAWN_POS.y as f64 + 1.0,
            SPAWN_POS.z as f64 + 0.5,
        ]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);

        let cursor = RaycastCursor {
            bullet: None,
            show: false,
            target_pos: DVec3::ZERO,
        };
        commands.entity(ent).insert(cursor);

        client.send_message(
            "Press ".italic()
                + "F3 + B".italic().color(Color::AQUA)
                + " to show hitboxes.".italic(),
        );
    }
}

#[derive(Component)]
struct Raycaster {
    bvh: Bvh<WithAabb<Entity>>,
}

#[derive(Component)]
struct RaycastCursor {
    bullet: Option<Entity>,
    show: bool,
    target_pos: DVec3,
}

fn rebuild_bvh(
    mut instances: Query<(Entity, &Instance, &mut Raycaster)>,
    entities: Query<(Entity, &McEntity)>,
) {
    for (instance_ent, instance, mut raycaster) in &mut instances.iter_mut() {
        raycaster.bvh.rebuild(
            entities
                .iter()
                .filter(|(ent, mc_ent)| mc_ent.instance() == instance_ent)
                .map(|(ent, mc_ent)| WithAabb::new(ent, mc_ent.hitbox())),
        );
    }
}

fn raycast(
    mut instances: Query<(Entity, &Instance, &mut Raycaster)>,
    mut clients: Query<(&mut Client, &mut RaycastCursor)>,
    mut mc_entities: Query<&mut McEntity, Without<Client>>,
) {
    for (instance_ent, instance, mut raycaster) in &mut instances.iter_mut() {
        for (mut client, cursor) in clients
            .iter_mut()
            .filter(|(c, _)| c.instance() == instance_ent)
        {
            let client_pos = client.position();

            let origin = DVec3::new(client_pos.x, client_pos.y + PLAYER_EYE_HEIGHT, client_pos.z);
            let direction = from_yaw_and_pitch(client.yaw(), client.pitch());
            let direction = DVec3::new(direction.x as f64, direction.y as f64, direction.z as f64);
            let not_self_or_bullet = |hit: RaycastHit<WithAabb<Entity>>| {
                let Ok(ent) = mc_entities.get_component::<McEntity>(hit.object.object) else {
                    return false;
                };
                if ent.kind() == EntityKind::ShulkerBullet {
                    return false;
                }
                return true;
            };

            if let Some(hit) =
                raycaster
                    .bvh
                    .raycast::<Entity>(origin, direction, not_self_or_bullet)
            {
                let hit_pos = origin + direction * hit.near;

                cursor.show = true;
                cursor.target_pos = hit_pos;

                client.set_action_bar("Intersection".color(Color::GREEN));
            } else {
                cursor.show = false;
                client.set_action_bar("No Intersection".color(Color::RED));
            }
        }
    }
}

fn manage_cursors(
    mut commands: Commands,
    mut clients: Query<(&mut Client, &mut RaycastCursor)>,
    mut mc_entities: Query<&mut McEntity, Without<Client>>,
) {
    for (mut client, cursor) in clients.iter_mut() {
        if cursor.show && cursor.bullet.is_none() {
            let bullet = McEntity::new(EntityKind::ShulkerBullet, client.instance());
            cursor.bullet = Some(commands.spawn(bullet).id());
        } else if !cursor.show && cursor.bullet.is_some() {
            commands.entity(cursor.bullet.unwrap()).despawn();
            cursor.bullet = None;
        }

        if let Some(bullet) = cursor.bullet {
            if let Ok(mut bullet) = mc_entities.get_component_mut::<McEntity>(bullet) {
                let mut target = cursor.target_pos;
                let hitbox = bullet.hitbox();
                target.y -= (hitbox.max.y - hitbox.min.y) / 2.0;
                bullet.set_position(target);
            }
        }
    }
}
