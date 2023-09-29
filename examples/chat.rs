#![allow(clippy::type_complexity)]

use tracing::warn;
use valence::chat::message::{ChatMessageEvent, CommandExecutionEvent, SendMessage};
use valence::chat::ChatState;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                handle_command_events,
                handle_message_events,
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

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut Position,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut client,
        mut pos,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        pos.0 = [0.0, SPAWN_Y as f64 + 1.0, 0.0].into();
        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        *game_mode = GameMode::Creative;

        client.send_game_message("Welcome to Valence! Say something.".italic());
    }
}

fn handle_message_events(
    mut clients: Query<(&mut Client, &mut ChatState)>,
    names: Query<&Username>,
    mut messages: EventReader<ChatMessageEvent>,
) {
    for message in messages.iter() {
        let sender_name = names.get(message.client).expect("Error getting username");
        // Need to find better way. Username is sender, while client and chat state are
        // recievers. Maybe try to add a chat feature to Client.
        for (mut client, mut state) in clients.iter_mut() {
            state
                .as_mut()
                .send_chat_message(client.as_mut(), sender_name, message)
                .expect("Error sending message");
        }
    }
}

fn handle_command_events(
    mut clients: Query<&mut Client>,
    mut commands: EventReader<CommandExecutionEvent>,
) {
    for command in commands.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(command.client) else {
            warn!("Unable to find client for message: {:?}", command);
            continue;
        };

        let message = command.command.to_string();

        let formatted =
            "You sent the command ".into_text() + ("/".into_text() + (message).into_text()).bold();

        client.send_game_message(formatted);
    }
}
