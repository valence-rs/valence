use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::{
    default_event_handler, ClickContainer, SetCreativeModeSlot, StartSneaking,
};
use valence_new::prelude::*;
use valence_protocol::packets::s2c::sound_id::SoundId;
use valence_protocol::types::SoundCategory;

const SPAWN_Y: i32 = 64;

const SLOT_MIN: i16 = 36;
const SLOT_MAX: i16 = 43;
const PITCH_MIN: f32 = 0.5;
const PITCH_MAX: f32 = 1.0;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, toggle_gamemode_on_sneak)
        .add_system_to_stage(EventLoop, click_to_play_notes)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
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

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block_state([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);
        client.send_message(
            "Welcome to Valence! Open your inventory, and click on your hotbar to play the piano."
                .italic(),
        );
        client.send_message(
            "Click the rightmost hotbar slot to toggle between creative and survival.".italic(),
        );
    }
}

fn toggle_gamemode_on_sneak(
    mut clients: Query<&mut Client>,
    mut events: EventReader<StartSneaking>,
) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(event.client) else {
            continue;
        };
        let mode = client.game_mode();
        client.set_game_mode(match mode {
            GameMode::Survival => GameMode::Creative,
            GameMode::Creative => GameMode::Survival,
            _ => GameMode::Creative,
        });
    }
}

fn click_to_play_notes(
    mut clients: Query<&mut Client>,
    mut events_click: EventReader<ClickContainer>,
    mut events_slot: EventReader<SetCreativeModeSlot>,
) {
    for event in events_click.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(event.client) else {
            continue;
        };
        let position = client.position();
        let position = BlockPos::new(position.x as i32, position.y as i32, position.z as i32);
        play_note(&mut client, position, event.slot_id);
    }

    for event in events_slot.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(event.client) else {
            continue;
        };
        let position = client.position();
        let position = BlockPos::new(position.x as i32, position.y as i32, position.z as i32);
        play_note(&mut client, position, event.slot);
    }
}

fn play_note(client: &mut Client, position: BlockPos, clicked_slot: i16) {
    if (SLOT_MIN..=SLOT_MAX).contains(&clicked_slot) {
        let pitch = (clicked_slot - SLOT_MIN) as f32 * (PITCH_MAX - PITCH_MIN)
            / (SLOT_MAX - SLOT_MIN) as f32
            + PITCH_MIN;

        client.send_message(format!("playing note with pitch: {pitch}"));

        client.write_packet(&valence_protocol::packets::s2c::play::SoundEffect {
            id: SoundId::Reference { id: 767.into() },
            seed: 0,
            position: position.into(),
            category: SoundCategory::Master,
            volume: 1.0,
            pitch,
        });
    } else if clicked_slot == 44 {
        client.set_game_mode(match client.game_mode() {
            GameMode::Survival => GameMode::Creative,
            GameMode::Creative => GameMode::Survival,
            _ => GameMode::Creative,
        });
    }
}
