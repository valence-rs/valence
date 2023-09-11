#![allow(clippy::type_complexity)]

use std::ops::DerefMut;
use parsers::vec2::Vec2 as Vec2Parser;
use parsers::vec3::Vec3 as Vec3Parser;
use valence::prelude::*;
use valence_command::graph::CommandGraphBuilder;
use valence_command::handler::CommandResultEvent;
use valence_command::parsers::entity_selector::{EntitySelector, EntitySelectors};
use valence_command::parsers::strings::{GreedyString, QuotableString};
use valence_command::parsers::CommandArg;
use valence_command::scopes::CommandScopes;
use valence_command::{parsers, Command, CommandApp, CommandScopeRegistry, ModifierValue};
use valence_command_derive::Command;
use valence_server::op_level::OpLevel;

const SPAWN_Y: i32 = 64;

#[derive(Command, Debug, Clone)]
#[paths("teleport", "tp")]
#[scopes("valence:command:teleport")]
enum Teleport {
    #[paths = "{location}"]
    ExecutorToLocation { location: Vec3Parser },
    #[paths = "{target}"]
    ExecutorToTarget { target: EntitySelector },
    #[paths = "{from} {to}"]
    TargetToTarget {
        from: EntitySelector,
        to: EntitySelector,
    },
    #[paths = "{target} {location}"]
    TargetToLocation {
        target: EntitySelector,
        location: Vec3Parser,
    },
}

#[derive(Command, Debug, Clone)]
#[paths("gamemode", "gm")]
#[scopes("valence:command:gamemode")]
enum Gamemode {
    #[paths("survival", "{/} gms")]
    Survival,
    #[paths("creative", "{/} gmc")]
    Creative,
    #[paths("adventure", "{/} gma")]
    Adventure,
    #[paths("spectator", "{/} gmsp")]
    Spectator,
}

#[derive(Command, Debug, Clone)]
#[paths("test ", "t ")]
#[scopes("valence:command:test")]
#[allow(dead_code)]
enum Test {
    // 3 literals with an arg each
    #[paths("a {a} b {b} c {c}", "{a} {b} {c}")]
    A { a: String, b: i32, c: f32 },
    // 2 literals with an arg last being optional (Because of the greedy string before the end
    // this is technically unreachable)
    #[paths = "a {a} {b} b {c?}"]
    B {
        a: parsers::dimension::Dimension,
        b: GreedyString,
        c: Option<String>,
    },
    // greedy string optional arg
    #[paths = "a {a} b {b?}"]
    C { a: String, b: Option<GreedyString> },
    // greedy string required arg
    #[paths = "a {a} b {b}"]
    D { a: String, b: GreedyString },
    // five optional args and an ending greedyString
    #[paths("options {a?} {b?} {c?} {d?} {e?}", "options {b?} {a?} {d?} {c?} {e?}")]
    E {
        a: Option<i32>,
        b: Option<QuotableString>,
        c: Option<Vec2Parser>,
        d: Option<Vec3Parser>,
        e: Option<GreedyString>,
    },
}

#[derive(Debug, Clone)]
enum ComplexRedirection {
    A(parsers::dimension::Dimension),
    B,
    C(Vec2Parser),
    D,
    E(Vec3Parser),
}

impl Command for ComplexRedirection {
    fn assemble_graph(graph: &mut CommandGraphBuilder<Self>)
    where
        Self: Sized,
    {
        let root = graph.root().id();

        let command_root = graph
            .literal("complex")
            .with_scopes(vec!["valence:command:complex"])
            .id();
        let a = graph.literal("a").id();

        graph
            .at(a)
            .argument("a")
            .with_parser::<parsers::dimension::Dimension>()
            .with_executable(|input| ComplexRedirection::A(parsers::dimension::Dimension::parse_arg(input).unwrap()));

        let b = graph.literal("b").id();

        graph.at(b).with_executable(|_| ComplexRedirection::B);
        graph.at(b).redirect_to(root);

        let c = graph.literal("c").id();

        graph
            .at(c)
            .argument("c")
            .with_parser::<Vec2Parser>()
            .with_executable(|input| ComplexRedirection::C(Vec2Parser::parse_arg(input).unwrap()));

        let d = graph
            .at(command_root)
            .literal("d")
            .with_modifier(|_, modifiers| {
                let entry = modifiers.entry("d_pass_count".into()).or_insert(0.into());
                if let ModifierValue::I32(i) = entry {
                    *i += 1;
                }
            })
            .id();

        graph.at(d).with_executable(|_| ComplexRedirection::D);
        graph.at(d).redirect_to(command_root);

        let e = graph.literal("e").id();

        graph
            .at(e)
            .argument("e")
            .with_parser::<Vec3Parser>()
            .with_executable(|input| ComplexRedirection::E(Vec3Parser::parse_arg(input).unwrap()));
    }
}

pub fn main() {
    App::new()
        .add_plugins((DefaultPlugins,))
        .add_command::<Test>()
        .add_command::<Teleport>()
        .add_command::<Gamemode>()
        .add_command::<ComplexRedirection>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                // Command handlers
                handle_test_command,
                handle_teleport_command,
                handle_complex_command,
                handle_gamemode_command,
            ),
        )
        .run();
}

fn handle_teleport_command(
    mut events: EventReader<CommandResultEvent<Teleport>>,
    mut clients: Query<(&mut Client, &mut Position)>,
    usernames: Query<(Entity, &Username)>,
) {
    for event in events.iter() {
        match &event.result {
            Teleport::ExecutorToLocation { location } => {
                let (client, pos) = &mut clients.get_mut(event.executor).unwrap();
                pos.0.x = location.x.get(pos.0.x as f32) as f64;
                pos.0.y = location.y.get(pos.0.y as f32) as f64;
                pos.0.z = location.z.get(pos.0.z as f32) as f64;

                client.send_chat_message(format!(
                    "Teleport command executor -> location executed with data:\n {:#?}",
                    &event.result
                ));
            }
            Teleport::ExecutorToTarget { target } => {
                let raw_target = match target {
                    EntitySelector::SimpleSelector(EntitySelectors::SinglePlayer(x)) => x,
                    _ => "not implemented",
                };
                let target = usernames.iter().find(|(_, name)| name.0 == *raw_target);

                match target {
                    None => {
                        let client = &mut clients.get_mut(event.executor).unwrap().0;
                        client.send_chat_message(format!("Could not find target: {}", raw_target));
                    }
                    Some(target_entity) => {
                        let target_pos = clients.get(target_entity.0).unwrap().1 .0;
                        let pos = &mut clients.get_mut(event.executor).unwrap().1 .0;
                        pos.x = target_pos.x;
                        pos.y = target_pos.y;
                        pos.z = target_pos.z;
                    }
                }

                let client = &mut clients.get_mut(event.executor).unwrap().0;
                client.send_chat_message(format!(
                    "Teleport command executor -> target executed with data:\n {:#?}",
                    &event.result
                ));
            }
            Teleport::TargetToTarget { from, to } => {
                let from_raw_target = match from {
                    EntitySelector::SimpleSelector(EntitySelectors::SinglePlayer(x)) => x,
                    _ => "not implemented",
                };
                let from_target = usernames
                    .iter()
                    .find(|(_, name)| name.0 == *from_raw_target);
                let to_raw_target = match to {
                    EntitySelector::SimpleSelector(EntitySelectors::SinglePlayer(x)) => x,
                    _ => "not implemented",
                };
                let to_target = usernames.iter().find(|(_, name)| name.0 == *to_raw_target);

                let client = &mut clients.get_mut(event.executor).unwrap().0;
                client.send_chat_message(format!(
                    "Teleport command target -> location with data:\n {:#?}",
                    &event.result
                ));
                match from_target {
                    None => {
                        client.send_chat_message(format!(
                            "Could not find target: {}",
                            from_raw_target
                        ));
                    }
                    Some(from_target_entity) => match to_target {
                        None => {
                            client.send_chat_message(format!(
                                "Could not find target: {}",
                                to_raw_target
                            ));
                        }
                        Some(to_target_entity) => {
                            let target_pos = *clients.get(to_target_entity.0).unwrap().1;
                            let (from_client, from_pos) =
                                &mut clients.get_mut(from_target_entity.0).unwrap();
                            from_pos.0 = target_pos.0;

                            from_client.send_chat_message(format!(
                                "You have been teleported to {}",
                                to_target_entity.1
                            ));

                            let to_client = &mut clients.get_mut(to_target_entity.0).unwrap().0;
                            to_client.send_chat_message(format!(
                                "{} has been teleported to your location",
                                from_target_entity.1
                            ));
                        }
                    },
                }
            }
            Teleport::TargetToLocation { target, location } => {
                let raw_target = match target {
                    EntitySelector::SimpleSelector(EntitySelectors::SinglePlayer(x)) => x,
                    _ => "not implemented",
                };
                let target = usernames.iter().find(|(_, name)| name.0 == *raw_target);

                let client = &mut clients.get_mut(event.executor).unwrap().0;
                client.send_chat_message(format!(
                    "Teleport command target -> location with data:\n {:#?}",
                    &event.result
                ));
                match target {
                    None => {
                        client.send_chat_message(format!("Could not find target: {}", raw_target));
                    }
                    Some(target_entity) => {
                        let (client, pos) = &mut clients.get_mut(target_entity.0).unwrap();
                        pos.0.x = location.x.get(pos.0.x as f32) as f64;
                        pos.0.y = location.y.get(pos.0.y as f32) as f64;
                        pos.0.z = location.z.get(pos.0.z as f32) as f64;

                        client.send_chat_message(format!(
                            "Teleport command executor -> location executed with data:\n {:#?}",
                            &event.result
                        ));
                    }
                }
            }
        }
    }
}

fn handle_test_command(
    mut events: EventReader<CommandResultEvent<Test>>,
    mut clients: Query<&mut Client>,
) {
    for event in events.iter() {
        let client = &mut clients.get_mut(event.executor).unwrap();
        client.send_chat_message(format!(
            "Test command executed with data:\n {:#?}",
            &event.result
        ));
    }
}

fn handle_complex_command(
    mut events: EventReader<CommandResultEvent<ComplexRedirection>>,
    mut clients: Query<&mut Client>,
) {
    for event in events.iter() {
        let client = &mut clients.get_mut(event.executor).unwrap();
        client.send_chat_message(format!(
            "complex command executed with data:\n {:#?}\n and with the modifiers:\n {:#?}",
            &event.result, &event.modifiers
        ));
    }
}

fn handle_gamemode_command(
    mut events: EventReader<CommandResultEvent<Gamemode>>,
    mut clients: Query<(&mut Client, &mut GameMode)>,
) {
    for event in events.iter() {
        let (mut client, mut gamemode) = clients.get_mut(event.executor).unwrap();
        match event.result {
            Gamemode::Adventure => {
                *gamemode = GameMode::Adventure;
                client.send_chat_message("Gamemode set to adventure");
            }
            Gamemode::Creative => {
                *gamemode = GameMode::Creative;
                client.send_chat_message("Gamemode set to creative");
            }
            Gamemode::Spectator => {
                *gamemode = GameMode::Spectator;
                client.send_chat_message("Gamemode set to spectator");
            }
            Gamemode::Survival => {
                *gamemode = GameMode::Survival;
                client.send_chat_message("Gamemode set to survival");
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    mut dimensions: ResMut<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    mut command_scopes: ResMut<CommandScopeRegistry>,
) {
    dimensions.deref_mut().insert(Ident::new("pooland").unwrap(), DimensionType::default());

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

    command_scopes.add_scope("valence:command:teleport");
    command_scopes.add_scope("valence:command:gamemode");
    command_scopes.add_scope("valence:command:test");
    command_scopes.add_scope("valence:command:complex");
    command_scopes.add_scope("valence:admin");
    command_scopes.link("valence:admin", "valence:command");

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut CommandScopes,
            &mut Position,
            &mut GameMode,
            &mut OpLevel,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut permissions,
        mut pos,
        mut game_mode,
        mut op_level,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);

        pos.0 = [0.0, SPAWN_Y as f64 + 1.0, 0.0].into();
        *game_mode = GameMode::Creative;
        op_level.set(4);

        permissions.add("valence:admin");
    }
}
