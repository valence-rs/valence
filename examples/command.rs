#![allow(clippy::type_complexity)]

use std::ops::DerefMut;

use command::graph::CommandGraphBuilder;
use command::handler::CommandResultEvent;
use command::parsers::entity_selector::{EntitySelector, EntitySelectors};
use command::parsers::{CommandArg, GreedyString, QuotableString};
use command::scopes::CommandScopes;
use command::{parsers, Command, CommandApp, CommandScopeRegistry, ModifierValue};
use command_macros::Command;
use parsers::{Vec2 as Vec2Parser, Vec3 as Vec3Parser};
use rand::prelude::IteratorRandom;
use valence::entity::living::LivingEntity;
use valence::prelude::*;
use valence::*;
use valence_server::op_level::OpLevel;

const SPAWN_Y: i32 = 64;

#[derive(Command, Debug, Clone)]
#[paths("teleport", "tp")]
#[scopes("valence.command.teleport")]
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
#[scopes("valence.command.gamemode")]
enum Gamemode {
    #[paths("survival {target?}", "{/} gms {target?}")]
    Survival { target: Option<EntitySelector> },
    #[paths("creative {target?}", "{/} gmc {target?}")]
    Creative { target: Option<EntitySelector> },
    #[paths("adventure {target?}", "{/} gma {target?}")]
    Adventure { target: Option<EntitySelector> },
    #[paths("spectator {target?}", "{/} gmspec {target?}")]
    Spectator { target: Option<EntitySelector> },
}

#[derive(Command, Debug, Clone)]
#[paths("test", "t")]
#[scopes("valence.command.test")]
#[allow(dead_code)]
enum Test {
    // 3 literals with an arg each
    #[paths("a {a} b {b} c {c}", "{a} {b} {c}")]
    A { a: String, b: i32, c: f32 },
    // 2 literals with an arg last being optional (Because of the greedy string before the end
    // this is technically unreachable)
    #[paths = "a {a} {b} b {c?}"]
    B {
        a: Vec3Parser,
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
    A(Vec3Parser),
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
            .with_scopes(vec!["valence.command.complex"])
            .id();
        let a = graph.literal("a").id();

        graph
            .at(a)
            .argument("a")
            .with_parser::<Vec3Parser>()
            .with_executable(|input| ComplexRedirection::A(Vec3Parser::parse_arg(input).unwrap()));

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

enum TeleportTarget {
    Targets(Vec<Entity>),
}

#[derive(Debug)]
enum TeleportDestination {
    Location(Vec3Parser),
    Target(Option<Entity>),
}

fn handle_teleport_command(
    mut events: EventReader<CommandResultEvent<Teleport>>,
    living_entities: Query<Entity, With<LivingEntity>>,
    mut clients: Query<(Entity, &mut Client)>,
    entity_layers: Query<&EntityLayerId>,
    mut positions: Query<&mut Position>,
    usernames: Query<(Entity, &Username)>,
) {
    for event in events.iter() {
        let compiled_command = match &event.result {
            Teleport::ExecutorToLocation { location } => (
                TeleportTarget::Targets(vec![event.executor]),
                TeleportDestination::Location(*location),
            ),
            Teleport::ExecutorToTarget { target } => (
                TeleportTarget::Targets(vec![event.executor]),
                TeleportDestination::Target(
                    find_targets(
                        &living_entities,
                        &mut clients,
                        &positions,
                        &entity_layers,
                        &usernames,
                        &event,
                        target,
                    )
                    .first()
                    .copied(),
                ),
            ),
            Teleport::TargetToTarget { from, to } => (
                TeleportTarget::Targets(
                    find_targets(
                        &living_entities,
                        &mut clients,
                        &positions,
                        &entity_layers,
                        &usernames,
                        &event,
                        from,
                    )
                    .to_vec(),
                ),
                TeleportDestination::Target(
                    find_targets(
                        &living_entities,
                        &mut clients,
                        &positions,
                        &entity_layers,
                        &usernames,
                        &event,
                        to,
                    )
                    .first()
                    .copied(),
                ),
            ),
            Teleport::TargetToLocation { target, location } => (
                TeleportTarget::Targets(
                    find_targets(
                        &living_entities,
                        &mut clients,
                        &positions,
                        &entity_layers,
                        &usernames,
                        &event,
                        target,
                    )
                    .to_vec(),
                ),
                TeleportDestination::Location(*location),
            ),
        };

        let (TeleportTarget::Targets(targets), destination) = compiled_command;

        println!(
            "executing teleport command {:#?} -> {:#?}",
            targets, destination
        );
        match destination {
            TeleportDestination::Location(location) => {
                for target in targets {
                    let mut pos = positions.get_mut(target).unwrap();
                    pos.0.x = location.x.get(pos.0.x as f32) as f64;
                    pos.0.y = location.y.get(pos.0.y as f32) as f64;
                    pos.0.z = location.z.get(pos.0.z as f32) as f64;
                }
            }
            TeleportDestination::Target(target) => {
                let target = target.unwrap();
                let target_pos = **positions.get(target).unwrap();
                for target in targets {
                    let mut position = positions.get_mut(target).unwrap();
                    position.0 = target_pos;
                }
            }
        }
    }
}

fn find_targets(
    living_entities: &Query<Entity, With<LivingEntity>>,
    clients: &mut Query<(Entity, &mut Client)>,
    positions: &Query<&mut Position>,
    entity_layers: &Query<&EntityLayerId>,
    usernames: &Query<(Entity, &Username)>,
    event: &&CommandResultEvent<Teleport>,
    target: &EntitySelector,
) -> Vec<Entity> {
    match target {
        EntitySelector::SimpleSelector(selector) => match selector {
            EntitySelectors::AllEntities => {
                let executor_entity_layer = *entity_layers.get(event.executor).unwrap();
                living_entities
                    .iter()
                    .filter(|entity| {
                        let entity_layer = entity_layers.get(*entity).unwrap();
                        entity_layer.0 == executor_entity_layer.0
                    })
                    .collect()
            }
            EntitySelectors::SinglePlayer(name) => {
                let target = usernames.iter().find(|(_, username)| username.0 == *name);
                match target {
                    None => {
                        let client = &mut clients.get_mut(event.executor).unwrap().1;
                        client.send_chat_message(format!("Could not find target: {}", name));
                        vec![]
                    }
                    Some(target_entity) => {
                        vec![target_entity.0]
                    }
                }
            }
            EntitySelectors::AllPlayers => {
                let executor_entity_layer = *entity_layers.get(event.executor).unwrap();
                clients
                    .iter_mut()
                    .filter_map(|(entity, ..)| {
                        let entity_layer = entity_layers.get(entity).unwrap();
                        if entity_layer.0 == executor_entity_layer.0 {
                            Some(entity)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            EntitySelectors::SelfPlayer => {
                vec![event.executor]
            }
            EntitySelectors::NearestPlayer => {
                let executor_entity_layer = *entity_layers.get(event.executor).unwrap();
                let executor_pos = positions.get(event.executor).unwrap();
                let target = clients
                    .iter_mut()
                    .filter(|(entity, ..)| {
                        *entity_layers.get(*entity).unwrap() == executor_entity_layer
                    })
                    .filter(|(target, ..)| *target != event.executor)
                    .map(|(target, ..)| target)
                    .min_by(|target, target2| {
                        let target_pos = positions.get(*target).unwrap();
                        let target2_pos = positions.get(*target2).unwrap();
                        let target_dist = target_pos.distance(**executor_pos);
                        let target2_dist = target2_pos.distance(**executor_pos);
                        target_dist.partial_cmp(&target2_dist).unwrap()
                    });
                match target {
                    None => {
                        let mut client = clients.get_mut(event.executor).unwrap().1;
                        client.send_chat_message("Could not find target".to_string());
                        vec![]
                    }
                    Some(target_entity) => {
                        vec![target_entity]
                    }
                }
            }
            EntitySelectors::RandomPlayer => {
                let executor_entity_layer = *entity_layers.get(event.executor).unwrap();
                let target = clients
                    .iter_mut()
                    .filter(|(entity, ..)| {
                        *entity_layers.get(*entity).unwrap() == executor_entity_layer
                    })
                    .choose(&mut rand::thread_rng())
                    .map(|(target, ..)| target);
                match target {
                    None => {
                        let mut client = clients.get_mut(event.executor).unwrap().1;
                        client.send_chat_message("Could not find target".to_string());
                        vec![]
                    }
                    Some(target_entity) => {
                        vec![target_entity]
                    }
                }
            }
        },
        EntitySelector::ComplexSelector(_, _) => {
            let mut client = clients.get_mut(event.executor).unwrap().1;
            client.send_chat_message("complex selector not implemented".to_string());
            vec![]
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
    mut clients: Query<(&mut Client, &mut GameMode, &Username, Entity)>,
    positions: Query<&Position>,
) {
    for event in events.iter() {
        let game_mode_to_set = match &event.result {
            Gamemode::Survival { .. } => GameMode::Survival,
            Gamemode::Creative { .. } => GameMode::Creative,
            Gamemode::Adventure { .. } => GameMode::Adventure,
            Gamemode::Spectator { .. } => GameMode::Spectator,
        };

        let selector = match &event.result {
            Gamemode::Survival { target } => target.clone(),
            Gamemode::Creative { target } => target.clone(),
            Gamemode::Adventure { target } => target.clone(),
            Gamemode::Spectator { target } => target.clone(),
        };

        match selector {
            None => {
                let (mut client, mut game_mode, ..) = clients.get_mut(event.executor).unwrap();
                *game_mode = game_mode_to_set;
                client.send_chat_message(format!(
                    "Gamemode command executor -> self executed with data:\n {:#?}",
                    &event.result
                ));
            }
            Some(selector) => match selector {
                EntitySelector::SimpleSelector(selector) => match selector {
                    EntitySelectors::AllEntities => {
                        for (mut client, mut game_mode, ..) in &mut clients.iter_mut() {
                            *game_mode = game_mode_to_set;
                            client.send_chat_message(format!(
                                "Gamemode command executor -> all entities executed with data:\n \
                                 {:#?}",
                                &event.result
                            ));
                        }
                    }
                    EntitySelectors::SinglePlayer(name) => {
                        let target = clients
                            .iter_mut()
                            .find(|(.., username, _)| username.0 == *name)
                            .map(|(.., target)| target);

                        match target {
                            None => {
                                let client = &mut clients.get_mut(event.executor).unwrap().0;
                                client
                                    .send_chat_message(format!("Could not find target: {}", name));
                            }
                            Some(target) => {
                                let mut game_mode = clients.get_mut(target).unwrap().1;
                                *game_mode = game_mode_to_set;

                                let client = &mut clients.get_mut(event.executor).unwrap().0;
                                client.send_chat_message(format!(
                                    "Gamemode command executor -> single player executed with \
                                     data:\n {:#?}",
                                    &event.result
                                ));
                            }
                        }
                    }
                    EntitySelectors::AllPlayers => {
                        for (mut client, mut game_mode, ..) in &mut clients.iter_mut() {
                            *game_mode = game_mode_to_set;
                            client.send_chat_message(format!(
                                "Gamemode command executor -> all entities executed with data:\n \
                                 {:#?}",
                                &event.result
                            ));
                        }
                    }
                    EntitySelectors::SelfPlayer => {
                        let (mut client, mut game_mode, ..) =
                            clients.get_mut(event.executor).unwrap();
                        *game_mode = game_mode_to_set;
                        client.send_chat_message(format!(
                            "Gamemode command executor -> self executed with data:\n {:#?}",
                            &event.result
                        ));
                    }
                    EntitySelectors::NearestPlayer => {
                        let executor_pos = positions.get(event.executor).unwrap();
                        let target = clients
                            .iter_mut()
                            .filter(|(.., target)| *target != event.executor)
                            .min_by(|(.., target), (.., target2)| {
                                let target_pos = positions.get(*target).unwrap();
                                let target2_pos = positions.get(*target2).unwrap();
                                let target_dist = target_pos.distance(**executor_pos);
                                let target2_dist = target2_pos.distance(**executor_pos);
                                target_dist.partial_cmp(&target2_dist).unwrap()
                            })
                            .map(|(.., target)| target);

                        match target {
                            None => {
                                let client = &mut clients.get_mut(event.executor).unwrap().0;
                                client.send_chat_message("Could not find target".to_string());
                            }
                            Some(target) => {
                                let mut game_mode = clients.get_mut(target).unwrap().1;
                                *game_mode = game_mode_to_set;

                                let client = &mut clients.get_mut(event.executor).unwrap().0;
                                client.send_chat_message(format!(
                                    "Gamemode command executor -> single player executed with \
                                     data:\n {:#?}",
                                    &event.result
                                ));
                            }
                        }
                    }
                    EntitySelectors::RandomPlayer => {
                        let target = clients
                            .iter_mut()
                            .choose(&mut rand::thread_rng())
                            .map(|(.., target)| target);

                        match target {
                            None => {
                                let client = &mut clients.get_mut(event.executor).unwrap().0;
                                client.send_chat_message("Could not find target".to_string());
                            }
                            Some(target) => {
                                let mut game_mode = clients.get_mut(target).unwrap().1;
                                *game_mode = game_mode_to_set;

                                let client = &mut clients.get_mut(event.executor).unwrap().0;
                                client.send_chat_message(format!(
                                    "Gamemode command executor -> single player executed with \
                                     data:\n {:#?}",
                                    &event.result
                                ));
                            }
                        }
                    }
                },
                EntitySelector::ComplexSelector(_, _) => {
                    let client = &mut clients.get_mut(event.executor).unwrap().0;
                    client
                        .send_chat_message("Complex selectors are not implemented yet".to_string());
                }
            },
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
    dimensions
        .deref_mut()
        .insert(Ident::new("pooland").unwrap(), DimensionType::default());

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

    command_scopes.link("valence.admin", "valence.command");

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

        permissions.add("valence.admin");
    }
}
