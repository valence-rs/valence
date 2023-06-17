use valence::client::despawn_disconnected_clients;
use valence::client::hand_swing::HandSwingEvent;
use valence::client::message::SendMessage;
use valence::inventory::HeldItem;
use valence::prelude::*;
use valence::world_time::{ChangeTrackingTimeBroadcast, LinearTimeTicking, WorldTime};

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(handle_adjust_speed)
        .add_system(handle_display_time)
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    biomes: Res<BiomeRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
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

    commands.spawn(instance).insert((
        WorldTime {
            time_of_day: -1,
            ..Default::default()
        },
        ChangeTrackingTimeBroadcast,
        LinearTimeTicking { speed: 1 },
    ));
}

fn init_clients(
    mut clients: Query<(&mut Client, &mut Location, &mut Position, &mut Inventory), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, mut loc, mut pos, mut inv) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, SPAWN_Y as f64 + 1.0, 0.5]);

        client.send_chat_message("Let's control time!");
        client.send_chat_message("- Left click the clock to make time run faster");
        client.send_chat_message("- Left click the trident to make time run slower");

        inv.set_slot(36, Some(ItemStack::new(ItemKind::Trident, 1, None)));
        inv.set_slot(37, Some(ItemStack::new(ItemKind::Clock, 1, None)));
    }
}

fn handle_adjust_speed(
    mut instances: Query<&mut LinearTimeTicking, With<Instance>>,
    clients: Query<(&Inventory, &HeldItem)>,
    mut events: EventReader<HandSwingEvent>,
) {
    for e in events.iter() {
        let Ok((inv, inv_state)) = clients.get(e.client) else {
            continue;
        };

        let mut ltt = instances.single_mut();

        let slot = inv_state.slot();
        let Some(is) = inv.slot(slot) else {
            continue;
        };

        match is.item {
            ItemKind::Clock => ltt.speed += 1,
            ItemKind::Trident => ltt.speed -= 1,
            _ => (),
        }
    }
}

fn handle_display_time(
    mut clients: Query<&mut Client>,
    instances: Query<(&WorldTime, &LinearTimeTicking), With<Instance>>,
) {
    for mut client in clients.iter_mut() {
        let (time, ltt) = instances.single();

        client.send_action_bar_message(format!(
            "Time: {} / {} per tick",
            time.time_of_day, ltt.speed
        ));
    }
}

// Add more systems here!
