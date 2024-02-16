#![allow(clippy::type_complexity)]

use std::f64::consts::TAU;

use valence::abilities::{PlayerStartFlyingEvent, PlayerStopFlyingEvent};
use valence::math::{DQuat, EulerRot};
use valence::message::SendMessage;
use valence::prelude::*;
use valence_text::color::NamedColor;

type SpherePartBundle = valence::entity::cow::CowEntityBundle;

const SPHERE_CENTER: DVec3 = DVec3::new(0.5, SPAWN_POS.y as f64 + 2.0, 0.5);
const SPHERE_AMOUNT: usize = 200;
const SPHERE_MIN_RADIUS: f64 = 6.0;
const SPHERE_MAX_RADIUS: f64 = 12.0;
const SPHERE_FREQ: f64 = 0.5;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -16);

/// Marker component for entities that are part of the sphere.
#[derive(Component)]
struct SpherePart;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                update_sphere,
                despawn_disconnected_clients,
                display_is_flying,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    layer.chunk.set_block(SPAWN_POS, BlockState::BEDROCK);

    let layer_id = commands.spawn(layer).id();

    commands.spawn_batch([0; SPHERE_AMOUNT].map(|_| {
        (
            SpherePartBundle {
                layer: EntityLayerId(layer_id),
                ..Default::default()
            },
            SpherePart,
        )
    }));
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([
            SPAWN_POS.x as f64 + 0.5,
            SPAWN_POS.y as f64 + 1.0,
            SPAWN_POS.z as f64 + 0.5,
        ]);
        *game_mode = GameMode::Creative;
    }
}

fn update_sphere(
    server: Res<Server>,
    mut parts: Query<(&mut Position, &mut Look, &mut HeadYaw), With<SpherePart>>,
) {
    let time = server.current_tick() as f64 / server.tick_rate().get() as f64;

    let rot_angles = DVec3::new(0.2, 0.4, 0.6) * SPHERE_FREQ * time * TAU % TAU;
    let rot = DQuat::from_euler(EulerRot::XYZ, rot_angles.x, rot_angles.y, rot_angles.z);

    let radius = lerp(
        SPHERE_MIN_RADIUS,
        SPHERE_MAX_RADIUS,
        ((time * SPHERE_FREQ * TAU).sin() + 1.0) / 2.0,
    );

    for ((mut pos, mut look, mut head_yaw), p) in
        parts.iter_mut().zip(fibonacci_spiral(SPHERE_AMOUNT))
    {
        debug_assert!(p.is_normalized());

        let dir = rot * p;

        pos.0 = SPHERE_CENTER + dir * radius;
        look.set_vec(dir.as_vec3());
        head_yaw.0 = look.yaw;
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

// Send an actionbar message to all clients when their flying state changes.
fn display_is_flying(
    mut player_start_flying_events: EventReader<PlayerStartFlyingEvent>,
    mut player_stop_flying_events: EventReader<PlayerStopFlyingEvent>,
    mut clients: Query<&mut Client>,
) {
    for event in player_start_flying_events.read() {
        if let Ok(mut client) = clients.get_mut(event.client) {
            client.send_action_bar_message("You are flying!".color(NamedColor::Green));
        }
    }

    for event in player_stop_flying_events.read() {
        if let Ok(mut client) = clients.get_mut(event.client) {
            client.send_action_bar_message("You are no longer flying!".color(NamedColor::Red));
        }
    }
}
