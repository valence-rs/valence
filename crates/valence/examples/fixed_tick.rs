#![allow(clippy::type_complexity)]

use valence::prelude::*;
use valence_client::message::SendMessage;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(TickSystem)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(run_every_20_ticks.in_schedule(CoreSchedule::FixedUpdate))
        .add_system(manual_tick_interval)
        .insert_resource(FixedTick::new(20))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Position, &mut Location, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut pos, mut loc, mut game_mode) in &mut clients {
        pos.0 = [0.0, SPAWN_Y as f64 + 1.0, 0.0].into();
        loc.0 = instances.single();
        *game_mode = GameMode::Creative;
    }
}

fn run_every_20_ticks(mut clients: Query<&mut Client>, tick: Res<Tick>) {
    for mut client in &mut clients {
        client.send_chat_message(format!("20 ticks have passed, tick: {}", tick.elapsed()));
    }
}

fn manual_tick_interval(
    mut clients: Query<&mut Client>,
    mut last_time: Local<usize>,
    tick: Res<Tick>,
) {
    if tick.elapsed() - *last_time >= 40 {
        *last_time = tick.elapsed();
        for mut client in &mut clients {
            client.send_chat_message(format!("40 ticks have passed, tick: {}", tick.elapsed()));
        }
    }
}
