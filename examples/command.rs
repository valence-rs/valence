#![allow(clippy::type_complexity)]

use clap::Parser;
use valence::prelude::*;
use valence_client::message::SendMessage;
use valence_command::command::{Command, CommandExecutionEvent, RegisterCommand};
use valence_command::node::{Nodes, PrimaryNodes};
use valence_command::parse::{parse_error_message, CommandExecutor, Parse};
use valence_command::reader::StrReader;
use valence_core::__private::VarInt;
use valence_core::protocol::packet::command::{Node, NodeData, Parser as NParser, Suggestion};

const SPAWN_Y: i32 = 64;

pub fn main() {
    App::new()
        // for packet inspector to work
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .register_command::<CreativeCommand>()
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    commands.spawn((Nodes::default(), PrimaryNodes));

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
    mut clients: Query<(&mut Location, &mut Position, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut game_mode) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
        *game_mode = GameMode::Creative;
    }
}

pub struct CreativeCommand;

impl Command for CreativeCommand {
    fn name() -> String {
        "creative".into()
    }

    fn build(app: &mut App) {
        app.add_systems(Update, (command_listen, creative_node));
    }
}

fn creative_node(mut query: Query<&mut Nodes, Added<Nodes>>) {
    for mut nodes in query.iter_mut() {
        let start = nodes.count() as i32;
        nodes
            .insert_command_nodes(
                [
                    Node {
                        children: vec![VarInt(start + 1)].into(),
                        data: NodeData::Literal { name: "creative" },
                        executable: false,
                        redirect_node: None,
                    },
                    Node {
                        children: vec![].into(),
                        data: NodeData::Argument {
                            name: "enable",
                            parser: NParser::Bool,
                            suggestion: None, // default suggestions
                        },
                        executable: true,
                        redirect_node: None,
                    },
                ]
                .into_iter(),
            )
            .unwrap();
    }
}

fn command_listen(
    mut event: EventReader<CommandExecutionEvent>,
    mut client_query: Query<&mut Client>,
) {
    for exec in event.iter() {
        let mut reader = exec.reader();
        if reader.read_unquoted_str() == "creative" {
            if let CommandExecutor::Entity(client) = exec.executor {
                let mut client = client_query.get_mut(client).unwrap();
                if !reader.skip_char(' ') {
                    continue;
                }
                if let Err(err) = bool::parse(&(), &mut Default::default(), &(), &mut reader) {
                    client.send_chat_message(parse_error_message(&reader, err));
                }
            }
        }
    }
}
