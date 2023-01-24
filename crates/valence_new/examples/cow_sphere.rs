use std::f64::consts::TAU;

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use glam::{DQuat, DVec3, EulerRot};
use valence_new::client::event::default_event_handler;
use valence_new::client::{despawn_disconnected_clients, Client};
use valence_new::config::Config;
use valence_new::dimension::DimensionId;
use valence_new::entity::{EntityKind, McEntity};
use valence_new::instance::{Chunk, Instance};
use valence_new::math::to_yaw_and_pitch;
use valence_new::player_list::{
    add_new_clients_to_player_list, remove_disconnected_clients_from_player_list,
};
use valence_new::server::Server;
use valence_protocol::block::BlockState;
use valence_protocol::types::GameMode;
use valence_protocol::BlockPos;

const SPHERE_CENTER: DVec3 = DVec3::new(0.5, SPAWN_POS.y as f64 + 2.0, 0.5);
const SPHERE_AMOUNT: usize = 200;
const SPHERE_TYPE: EntityKind = EntityKind::Cow;
const SPHERE_MIN_RADIUS: f64 = 6.0;
const SPHERE_MAX_RADIUS: f64 = 12.0;
const SPHERE_FREQ: f64 = 0.5;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -20);

/// Marker component for entities that are part of the sphere.
#[derive(Component)]
struct SpherePart;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    valence_new::run_server(
        Config::default(),
        SystemStage::parallel()
            .with_system(setup.with_run_criteria(ShouldRun::once))
            .with_system(init_clients)
            .with_system(update_sphere)
            .with_system(default_event_handler())
            .with_system(despawn_disconnected_clients)
            .with_system(add_new_clients_to_player_list)
            .with_system(remove_disconnected_clients_from_player_list),
        (),
    )
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

    let instance_id = world.spawn(instance).id();

    world.spawn_batch(
        [0; SPHERE_AMOUNT].map(|_| (McEntity::new(SPHERE_TYPE, instance_id), SpherePart)),
    );
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([
            SPAWN_POS.x as f64 + 0.5,
            SPAWN_POS.y as f64 + 1.0,
            SPAWN_POS.z as f64 + 0.5,
        ]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);
    }
}

fn update_sphere(server: Res<Server>, mut parts: Query<&mut McEntity, With<SpherePart>>) {
    let time = server.current_tick() as f64 / server.tick_rate() as f64;

    let rot_angles = DVec3::new(0.2, 0.4, 0.6) * SPHERE_FREQ * time * TAU % TAU;
    let rot = DQuat::from_euler(EulerRot::XYZ, rot_angles.x, rot_angles.y, rot_angles.z);

    let radius = lerp(
        SPHERE_MIN_RADIUS,
        SPHERE_MAX_RADIUS,
        ((time * SPHERE_FREQ * TAU).sin() + 1.0) / 2.0,
    );

    for (mut entity, p) in parts.iter_mut().zip(fibonacci_spiral(SPHERE_AMOUNT)) {
        debug_assert!(p.is_normalized());

        let dir = rot * p;
        let (yaw, pitch) = to_yaw_and_pitch(dir.as_vec3());

        entity.set_position(SPHERE_CENTER + dir * radius);
        entity.set_yaw(yaw);
        entity.set_head_yaw(yaw);
        entity.set_pitch(pitch);
    }
}

/// Distributes N points on the surface of a unit sphere.
fn fibonacci_spiral(n: usize) -> impl Iterator<Item = DVec3> {
    let golden_ratio = (1.0 + 5_f64.sqrt()) / 2.0;

    (0..n).map(move |i| {
        // Map to unit square
        let x = i as f64 / golden_ratio % 1.0;
        let y = i as f64 / n as f64;

        // Map from unit square to unit sphere.
        let theta = x * TAU;
        let phi = (1.0 - 2.0 * y).acos();
        DVec3::new(theta.cos() * phi.sin(), theta.sin() * phi.sin(), phi.cos())
    })
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}
