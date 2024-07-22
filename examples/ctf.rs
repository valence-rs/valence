#![allow(clippy::type_complexity)]

use std::collections::HashMap;

use bevy_ecs::query::QueryData;
use valence::entity::cow::CowEntityBundle;
use valence::entity::entity::Flags;
use valence::entity::living::Health;
use valence::entity::pig::PigEntityBundle;
use valence::entity::player::PlayerEntityBundle;
use valence::entity::{EntityAnimations, EntityStatuses, OnGround, Velocity};
use valence::interact_block::InteractBlockEvent;
use valence::inventory::HeldItem;
use valence::log::debug;
use valence::math::Vec3Swizzles;
use valence::nbt::{compound, List};
use valence::prelude::*;
use valence::scoreboard::*;
use valence::status::RequestRespawnEvent;

const ARENA_Y: i32 = 64;
const ARENA_MID_WIDTH: i32 = 2;
const SPAWN_BOX: [i32; 3] = [0, ARENA_Y + 20, 0];
const SPAWN_POS: [f64; 3] = [
    SPAWN_BOX[0] as f64,
    SPAWN_BOX[1] as f64 + 1.0,
    SPAWN_BOX[2] as f64,
];
const SPAWN_BOX_WIDTH: i32 = 5;
const SPAWN_BOX_HEIGHT: i32 = 4;
const PLAYER_MAX_HEALTH: f32 = 20.0;

pub fn main() {
    App::new()
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(EventLoopUpdate, handle_combat_events)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                digging,
                place_blocks,
                do_team_selector_portals,
                update_flag_visuals,
                do_flag_capturing,
                // visualize_triggers,
                update_clones,
                teleport_oob_clients,
                necromancy,
                update_scoreboard,
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

    for z in -50..50 {
        for x in -50..50 {
            let block = match x {
                x if x < -ARENA_MID_WIDTH => BlockState::RED_CONCRETE,
                x if x > ARENA_MID_WIDTH => BlockState::BLUE_CONCRETE,
                _ => BlockState::WHITE_CONCRETE,
            };
            layer.chunk.set_block([x, ARENA_Y, z], block);
        }
    }

    let red_flag = build_flag(
        &mut layer,
        Team::Red,
        BlockPos {
            x: -48,
            y: ARENA_Y + 1,
            z: 0,
        },
    );
    let blue_flag = build_flag(
        &mut layer,
        Team::Blue,
        BlockPos {
            x: 48,
            y: ARENA_Y + 1,
            z: 0,
        },
    );

    build_spawn_box(&mut layer, SPAWN_BOX, &mut commands);

    commands.spawn(layer);

    let ctf_objective_layer = commands.spawn(EntityLayer::new(&server)).id();
    let ctf_objective = ObjectiveBundle {
        name: Objective::new("ctf-captures"),
        display: ObjectiveDisplay("Captures".into_text()),
        layer: EntityLayerId(ctf_objective_layer),
        ..Default::default()
    };
    commands.spawn(ctf_objective);

    let red_capture_trigger =
        TriggerArea::new(red_flag.offset(-5, -3, -5), red_flag.offset(5, 3, 5));
    let blue_capture_trigger =
        TriggerArea::new(blue_flag.offset(-5, -3, -5), blue_flag.offset(5, 3, 5));
    let mappos = CtfGlobals {
        scoreboard_layer: ctf_objective_layer,

        red_flag,
        blue_flag,

        red_capture_trigger,
        blue_capture_trigger,
    };

    commands.insert_resource(mappos);
    commands.insert_resource(FlagManager {
        red: None,
        blue: None,
    });

    let ctf_team_layers = CtfLayers::init(&mut commands, &server);

    // add some debug entities to the ctf entity layers
    let mut flags = Flags::default();
    flags.set_glowing(true);
    let mut pig = commands.spawn(PigEntityBundle {
        layer: EntityLayerId(ctf_team_layers.friendly_layers[&Team::Red]),
        position: Position([-30.0, 65.0, 2.0].into()),
        entity_flags: flags.clone(),
        ..Default::default()
    });
    pig.insert(Team::Red);

    let mut cow = commands.spawn(CowEntityBundle {
        layer: EntityLayerId(ctf_team_layers.friendly_layers[&Team::Blue]),
        position: Position([30.0, 65.0, 2.0].into()),
        entity_flags: flags,
        ..Default::default()
    });
    cow.insert(Team::Blue);

    commands.insert_resource(ctf_team_layers);
    commands.insert_resource(Score::default());
}

/// Build a flag at the given position. `pos` should be the position of the
/// bottom of the flag.
///
/// Returns the block position of the flag.
fn build_flag(layer: &mut LayerBundle, team: Team, pos: impl Into<BlockPos>) -> BlockPos {
    let mut pos = pos.into();

    // build the flag pole
    for _ in 0..3 {
        layer.chunk.set_block(pos, BlockState::OAK_FENCE);
        pos.y += 1;
    }
    let moving_east = pos.x < 0;
    layer.chunk.set_block(
        pos,
        BlockState::OAK_FENCE.set(
            if moving_east {
                PropName::East
            } else {
                PropName::West
            },
            PropValue::True,
        ),
    );
    pos.x += if pos.x < 0 { 1 } else { -1 };
    layer.chunk.set_block(
        pos,
        BlockState::OAK_FENCE
            .set(PropName::East, PropValue::True)
            .set(PropName::West, PropValue::True),
    );
    pos.x += if pos.x < 0 { 1 } else { -1 };
    layer.chunk.set_block(
        pos,
        BlockState::OAK_FENCE.set(
            if moving_east {
                PropName::West
            } else {
                PropName::East
            },
            PropValue::True,
        ),
    );
    pos.y -= 1;

    // build the flag
    layer.chunk.set_block(
        pos,
        match team {
            Team::Red => BlockState::RED_WOOL,
            Team::Blue => BlockState::BLUE_WOOL,
        },
    );

    pos
}

fn build_spawn_box(layer: &mut LayerBundle, pos: impl Into<BlockPos>, commands: &mut Commands) {
    let pos = pos.into();

    let spawn_box_block = BlockState::GLASS;

    // build floor and roof
    for z in -SPAWN_BOX_WIDTH..=SPAWN_BOX_WIDTH {
        for x in -SPAWN_BOX_WIDTH..=SPAWN_BOX_WIDTH {
            layer
                .chunk
                .set_block([pos.x + x, pos.y, pos.z + z], spawn_box_block);
            layer.chunk.set_block(
                [pos.x + x, pos.y + SPAWN_BOX_HEIGHT, pos.z + z],
                spawn_box_block,
            );
        }
    }

    // build walls
    for z in [-SPAWN_BOX_WIDTH, SPAWN_BOX_WIDTH] {
        for x in -SPAWN_BOX_WIDTH..=SPAWN_BOX_WIDTH {
            for y in pos.y..=pos.y + SPAWN_BOX_HEIGHT - 1 {
                layer
                    .chunk
                    .set_block([pos.x + x, y, pos.z + z], spawn_box_block);
            }
        }
    }

    for x in [-SPAWN_BOX_WIDTH, SPAWN_BOX_WIDTH] {
        for z in -SPAWN_BOX_WIDTH..=SPAWN_BOX_WIDTH {
            for y in pos.y..=pos.y + SPAWN_BOX_HEIGHT - 1 {
                layer
                    .chunk
                    .set_block([pos.x + x, y, pos.z + z], spawn_box_block);
            }
        }
    }

    // build team selector portals
    for (block, offset) in [
        (
            BlockState::RED_CONCRETE,
            BlockPos::new(-SPAWN_BOX_WIDTH, 0, SPAWN_BOX_WIDTH - 2),
        ),
        (
            BlockState::BLUE_CONCRETE,
            BlockPos::new(SPAWN_BOX_WIDTH - 2, 0, SPAWN_BOX_WIDTH - 2),
        ),
    ] {
        for z in 0..3 {
            for x in 0..3 {
                layer.chunk.set_block(
                    [pos.x + offset.x + x, pos.y + offset.y, pos.z + offset.z + z],
                    block,
                );
            }
        }
    }

    let red = [
        pos.x - SPAWN_BOX_WIDTH + 1,
        pos.y,
        pos.z + SPAWN_BOX_WIDTH - 1,
    ];
    let red_area = TriggerArea::new(red, red);
    let blue = [
        pos.x + SPAWN_BOX_WIDTH - 1,
        pos.y,
        pos.z + SPAWN_BOX_WIDTH - 1,
    ];
    let blue_area = TriggerArea::new(blue, blue);
    let portals = Portals {
        portals: HashMap::from_iter(vec![(Team::Red, red_area), (Team::Blue, blue_area)]),
    };

    for area in portals.portals.values() {
        for pos in area.iter_block_pos() {
            layer.chunk.set_block(pos, BlockState::AIR);
        }
        layer
            .chunk
            .set_block(area.a.offset(0, -1, 0), BlockState::BARRIER);
    }

    commands.insert_resource(portals);

    // build instruction signs

    let sign_pos = pos.offset(0, 2, SPAWN_BOX_WIDTH - 1);
    layer.chunk.set_block(
        sign_pos,
        Block {
            state: BlockState::OAK_WALL_SIGN.set(PropName::Rotation, PropValue::_3),
            nbt: Some(compound! {
                "front_text" => compound! {
                    "messages" => List::String(vec![
                        "Capture".color(Color::YELLOW).bold().to_string(),
                        "the".color(Color::YELLOW).bold().to_string(),
                        "Flag!".color(Color::YELLOW).bold().to_string(),
                        "Select a Team".color(Color::WHITE).italic().to_string(),
                    ])
                },
            }),
        },
    );

    layer.chunk.set_block(
        sign_pos.offset(-1, 0, 0),
        Block {
            state: BlockState::OAK_WALL_SIGN.set(PropName::Rotation, PropValue::_3),
            nbt: Some(compound! {
                "front_text" => compound! {
                    "messages" => List::String(vec![
                        "".into_text().to_string(),
                        ("Join ".bold().color(Color::WHITE) + Team::Red.team_text()).to_string(),
                        "=>".bold().color(Color::WHITE).to_string(),
                        "".into_text().to_string(),
                    ])
                },
            }),
        },
    );

    layer.chunk.set_block(
        sign_pos.offset(1, 0, 0),
        Block {
            state: BlockState::OAK_WALL_SIGN.set(PropName::Rotation, PropValue::_3),
            nbt: Some(compound! {
                "front_text" => compound! {
                    "messages" => List::String(vec![
                        "".into_text().to_string(),
                        ("Join ".bold().color(Color::WHITE) + Team::Blue.team_text()).to_string(),
                        "<=".bold().color(Color::WHITE).to_string(),
                        "".into_text().to_string(),
                    ])
                },
            }),
        },
    );
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
            &mut Health,
        ),
        Added<Client>,
    >,
    main_layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
    globals: Res<CtfGlobals>,
) {
    for (
        mut client,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
        mut health,
    ) in &mut clients
    {
        let layer = main_layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        visible_entity_layers.0.insert(globals.scoreboard_layer);
        pos.set(SPAWN_POS);
        *game_mode = GameMode::Adventure;
        health.0 = PLAYER_MAX_HEALTH;

        client.send_chat_message(
            "Welcome to Valence! Select a team by jumping in the team's portal.".italic(),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
enum Team {
    Red,
    Blue,
}

impl Team {
    fn spawn_pos(self) -> DVec3 {
        [
            match self {
                Team::Red => -40.0,
                Team::Blue => 40.0,
            },
            f64::from(ARENA_Y) + 1.0,
            0.0,
        ]
        .into()
    }

    fn team_text(self) -> Text {
        match self {
            Team::Red => "RED".color(Color::RED).bold(),
            Team::Blue => "BLUE".color(Color::BLUE).bold(),
        }
    }

    fn iter() -> impl Iterator<Item = Self> {
        [Team::Red, Team::Blue].iter().copied()
    }
}

fn digging(
    mut clients: Query<(&GameMode, &Team, Entity, &mut Client, &mut Inventory)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<DiggingEvent>,
    mut commands: Commands,
    globals: Res<CtfGlobals>,
    mut flag_manager: ResMut<FlagManager>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        let Ok((game_mode, team, ent, mut client, mut inv)) = clients.get_mut(event.client) else {
            continue;
        };

        if (*game_mode == GameMode::Creative && event.state == DiggingState::Start)
            || (*game_mode == GameMode::Survival && event.state == DiggingState::Stop)
        {
            let Some(block) = layer.block(event.position) else {
                continue;
            };
            let is_flag = event.position == globals.red_flag || event.position == globals.blue_flag;

            match (team, block.state) {
                (Team::Blue, BlockState::RED_WOOL) => {
                    if event.position == globals.red_flag {
                        commands.entity(event.client).insert(HasFlag(Team::Red));
                        client.send_chat_message("You have the flag!".italic());
                        flag_manager.red = Some(ent);
                        return;
                    }
                }
                (Team::Red, BlockState::BLUE_WOOL) => {
                    if event.position == globals.blue_flag {
                        commands.entity(event.client).insert(HasFlag(Team::Blue));
                        client.send_chat_message("You have the flag!".italic());
                        flag_manager.blue = Some(ent);
                        return;
                    }
                }
                _ => {}
            }

            if event.position.y <= ARENA_Y
                || block.state.to_kind() == BlockKind::OakFence
                || is_flag
            {
                continue;
            }

            let prev = layer.set_block(event.position, BlockState::AIR);

            if let Some(prev) = prev {
                let kind: ItemKind = prev.state.to_kind().to_item_kind();
                if let Some(slot) = inv.first_slot_with_item_in(kind, 64, 9..45) {
                    let count = inv.slot(slot).count;
                    inv.set_slot_amount(slot, count + 1);
                } else {
                    let stack = ItemStack::new(kind, 1, None);
                    if let Some(empty_slot) = inv.first_empty_slot_in(9..45) {
                        inv.set_slot(empty_slot, stack);
                    } else {
                        debug!("No empty slot to give item to player: {:?}", kind);
                    }
                }
            }
        }
    }
}

fn place_blocks(
    mut clients: Query<(&mut Inventory, &GameMode, &HeldItem)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        let Ok((mut inventory, game_mode, held)) = clients.get_mut(event.client) else {
            continue;
        };
        if event.hand != Hand::Main {
            continue;
        }

        // get the held item
        let slot_id = held.slot();
        let stack = inventory.slot(slot_id);
        if stack.is_empty() {
            continue;
        }

        let Some(block_kind) = BlockKind::from_item_kind(stack.item) else {
            // can't place this item as a block
            continue;
        };

        if *game_mode == GameMode::Survival {
            // check if the player has the item in their inventory and remove
            // it.
            if stack.count > 1 {
                let count = stack.count;
                inventory.set_slot_amount(slot_id, count - 1);
            } else {
                inventory.set_slot(slot_id, ItemStack::EMPTY);
            }
        }
        let real_pos = event.position.get_in_direction(event.face);
        layer.set_block(real_pos, block_kind.to_state());
    }
}

#[derive(Debug, Resource)]
struct Portals {
    portals: HashMap<Team, TriggerArea>,
}

fn do_team_selector_portals(
    mut players: Query<
        (
            Entity,
            &mut Position,
            &mut Look,
            &mut HeadYaw,
            &mut GameMode,
            &mut Client,
            &mut VisibleEntityLayers,
            &UniqueId,
        ),
        Without<Team>,
    >,
    portals: Res<Portals>,
    mut commands: Commands,
    ctf_layers: Res<CtfLayers>,
    main_layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for player in &mut players {
        let (
            player,
            mut pos,
            mut look,
            mut head_yaw,
            mut game_mode,
            mut client,
            mut ent_layers,
            unique_id,
        ) = player;
        if pos.0.y < f64::from(SPAWN_BOX[1]) - 5.0 {
            pos.0 = SPAWN_POS.into();
            continue;
        }

        let team = portals
            .portals
            .iter()
            .filter(|(_, area)| area.contains_pos(pos.0))
            .map(|(team, _)| team)
            .next()
            .copied();

        if let Some(team) = team {
            *game_mode = GameMode::Survival;
            let mut inventory = Inventory::new(InventoryKind::Player);
            inventory.set_slot(36, ItemStack::new(ItemKind::WoodenSword, 1, None));
            inventory.set_slot(
                37,
                ItemStack::new(
                    match team {
                        Team::Red => ItemKind::RedWool,
                        Team::Blue => ItemKind::BlueWool,
                    },
                    64,
                    None,
                ),
            );
            let combat_state = CombatState::default();
            commands
                .entity(player)
                .insert((team, inventory, combat_state));
            pos.0 = team.spawn_pos();
            let yaw = match team {
                Team::Red => -90.0,
                Team::Blue => 90.0,
            };
            look.yaw = yaw;
            look.pitch = 0.0;
            head_yaw.0 = yaw;
            let chat_text: Text = "You are on team ".into_text() + team.team_text() + "!";
            client.send_chat_message(chat_text);

            let main_layer = main_layers.single();
            ent_layers.as_mut().0.remove(&main_layer);
            for t in Team::iter() {
                let enemy_layer = ctf_layers.enemy_layers[&t];
                if t == team {
                    ent_layers.as_mut().0.remove(&enemy_layer);
                } else {
                    ent_layers.as_mut().0.insert(enemy_layer);
                }
            }
            let friendly_layer = ctf_layers.friendly_layers[&team];
            ent_layers.as_mut().0.insert(friendly_layer);

            // Copy the player entity to the friendly layer, and make them glow.
            let mut flags = Flags::default();
            flags.set_glowing(true);
            let mut player_glowing = commands.spawn(PlayerEntityBundle {
                layer: EntityLayerId(friendly_layer),
                uuid: *unique_id,
                entity_flags: flags,
                position: *pos,
                ..Default::default()
            });
            player_glowing.insert(ClonedEntity(player));

            let enemy_layer = ctf_layers.enemy_layers[&team];
            let mut player_enemy = commands.spawn(PlayerEntityBundle {
                layer: EntityLayerId(enemy_layer),
                uuid: *unique_id,
                position: *pos,
                ..Default::default()
            });
            player_enemy.insert(ClonedEntity(player));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TriggerArea {
    a: BlockPos,
    b: BlockPos,
}

impl TriggerArea {
    fn new<P: Into<BlockPos>>(a: impl Into<BlockPos>, b: P) -> Self {
        Self {
            a: a.into(),
            b: b.into(),
        }
    }

    fn contains(&self, pos: BlockPos) -> bool {
        let min = BlockPos::new(
            self.a.x.min(self.b.x),
            self.a.y.min(self.b.y),
            self.a.z.min(self.b.z),
        );
        let max = BlockPos::new(
            self.a.x.max(self.b.x),
            self.a.y.max(self.b.y),
            self.a.z.max(self.b.z),
        );

        pos.x >= min.x
            && pos.x <= max.x
            && pos.y >= min.y
            && pos.y <= max.y
            && pos.z >= min.z
            && pos.z <= max.z
    }

    fn contains_pos(&self, pos: DVec3) -> bool {
        self.contains(pos.into())
    }

    fn iter_block_pos(&self) -> impl Iterator<Item = BlockPos> {
        let min = BlockPos::new(
            self.a.x.min(self.b.x),
            self.a.y.min(self.b.y),
            self.a.z.min(self.b.z),
        );
        let max = BlockPos::new(
            self.a.x.max(self.b.x),
            self.a.y.max(self.b.y),
            self.a.z.max(self.b.z),
        );

        (min.x..=max.x)
            .flat_map(move |x| (min.y..=max.y).map(move |y| (x, y)))
            .flat_map(move |(x, y)| (min.z..=max.z).map(move |z| BlockPos::new(x, y, z)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
#[component(storage = "SparseSet")]
struct HasFlag(Team);

#[derive(Debug, Resource)]
struct FlagManager {
    red: Option<Entity>,
    blue: Option<Entity>,
}

#[derive(Debug, Resource)]
struct CtfGlobals {
    pub(crate) scoreboard_layer: Entity,

    pub(crate) red_flag: BlockPos,
    pub(crate) blue_flag: BlockPos,

    pub(crate) red_capture_trigger: TriggerArea,
    pub(crate) blue_capture_trigger: TriggerArea,
}

fn update_flag_visuals(
    flag_manager: Res<FlagManager>,
    globals: Res<CtfGlobals>,
    mut layers: Query<&mut ChunkLayer>,
) {
    if !flag_manager.is_changed() {
        return;
    }
    let red_flag_block = match flag_manager.red {
        Some(_) => BlockState::AIR,
        None => BlockState::RED_WOOL,
    };
    let blue_flag_block = match flag_manager.blue {
        Some(_) => BlockState::AIR,
        None => BlockState::BLUE_WOOL,
    };

    layers
        .single_mut()
        .set_block(globals.red_flag, red_flag_block);
    layers
        .single_mut()
        .set_block(globals.blue_flag, blue_flag_block);
}

fn do_flag_capturing(
    globals: Res<CtfGlobals>,
    mut players: Query<(Entity, &mut Client, &Team, &Position, &HasFlag)>,
    mut commands: Commands,
    mut flag_manager: ResMut<FlagManager>,
    mut score: ResMut<Score>,
) {
    for (ent, mut client, team, position, has_flag) in &mut players {
        let capture_trigger = match team {
            Team::Red => &globals.red_capture_trigger,
            Team::Blue => &globals.blue_capture_trigger,
        };

        if capture_trigger.contains_pos(position.0) {
            client.send_chat_message("You captured the flag!".italic());
            score
                .scores
                .entry(*team)
                .and_modify(|score| *score += 1)
                .or_insert(1);
            client.send_chat_message(score.render_scores());
            commands.entity(ent).remove::<HasFlag>();
            match has_flag.0 {
                Team::Red => flag_manager.red = None,
                Team::Blue => flag_manager.blue = None,
            }
        }
    }
}

#[derive(Debug, Default, Resource)]
struct Score {
    scores: HashMap<Team, u32>,
}

impl Score {
    fn render_scores(&self) -> Text {
        let mut text = "Scores:\n".into_text();
        for team in Team::iter() {
            let score = self.scores.get(&team).unwrap_or(&0);
            text += team.team_text() + ": " + score.to_string() + "\n";
        }
        text
    }
}

#[allow(dead_code)]
/// Visualizes the trigger areas, for debugging.
fn visualize_triggers(globals: Res<CtfGlobals>, mut layers: Query<&mut ChunkLayer>) {
    fn vis_trigger(trigger: &TriggerArea, layer: &mut ChunkLayer) {
        for pos in trigger.iter_block_pos() {
            layer.play_particle(
                &Particle::Crit,
                false,
                [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ],
                [0., 0., 0.],
                0.0,
                1,
            );
        }
    }

    for mut layer in &mut layers {
        vis_trigger(&globals.red_capture_trigger, &mut layer);
        vis_trigger(&globals.blue_capture_trigger, &mut layer);
    }
}

/// Keeps track of the entity layers per team.
#[derive(Debug, Resource)]
struct CtfLayers {
    /// Maps a team to the entity layer that contains how friendly players
    /// should be viewed.
    ///
    /// This is used to make friendly players glow.
    pub(crate) friendly_layers: HashMap<Team, Entity>,
    /// Ditto, but for enemy players.
    pub(crate) enemy_layers: HashMap<Team, Entity>,
}

impl CtfLayers {
    fn init(commands: &mut Commands, server: &Server) -> Self {
        let mut friendly_layers = HashMap::new();
        let mut enemy_layers = HashMap::new();

        for team in Team::iter() {
            let friendly_layer = commands.spawn((EntityLayer::new(server), team)).id();
            friendly_layers.insert(team, friendly_layer);
            let enemy_layer = commands.spawn((EntityLayer::new(server), team)).id();
            enemy_layers.insert(team, enemy_layer);
        }

        Self {
            friendly_layers,
            enemy_layers,
        }
    }
}

/// A marker component for entities that have been cloned, and the primary
/// entity they were cloned from.
#[derive(Debug, Component)]
struct ClonedEntity(Entity);

#[derive(Debug, QueryData)]
#[query_data(mutable)]
struct CloneQuery {
    position: &'static mut Position,
    head_yaw: &'static mut HeadYaw,
    velocity: &'static mut Velocity,
    look: &'static mut Look,
    animations: &'static mut EntityAnimations,
    on_ground: &'static mut OnGround,
    statuses: &'static mut EntityStatuses,
}

fn update_clones(
    ents: Query<CloneQueryReadOnly, Without<ClonedEntity>>,
    mut clone_ents: Query<(CloneQuery, &ClonedEntity, Entity)>,
    mut commands: Commands,
) {
    for clone in &mut clone_ents {
        let (mut clone, cloned_from, ent) = clone;
        let Ok(src) = ents.get(cloned_from.0) else {
            commands.entity(ent).insert(Despawned);
            return;
        };

        *clone.position = *src.position;
        *clone.head_yaw = *src.head_yaw;
        *clone.velocity = *src.velocity;
        *clone.look = *src.look;
        *clone.animations = *src.animations;
        *clone.on_ground = *src.on_ground;
        *clone.statuses = *src.statuses;
    }
}

/// Attached to every client.
#[derive(Component, Default)]
struct CombatState {
    /// The tick the client was last attacked.
    last_attacked_tick: i64,
    has_bonus_knockback: bool,
}

#[derive(QueryData)]
#[query_data(mutable)]
struct CombatQuery {
    client: &'static mut Client,
    pos: &'static Position,
    state: &'static mut CombatState,
    statuses: &'static mut EntityStatuses,
    health: &'static mut Health,
    inventory: &'static Inventory,
    held_item: &'static HeldItem,
    team: &'static Team,
}

fn handle_combat_events(
    server: Res<Server>,
    mut clients: Query<CombatQuery>,
    mut sprinting: EventReader<SprintEvent>,
    mut interact_entity: EventReader<InteractEntityEvent>,
    clones: Query<&ClonedEntity>,
) {
    for &SprintEvent { client, state } in sprinting.read() {
        if let Ok(mut client) = clients.get_mut(client) {
            client.state.has_bonus_knockback = state == SprintState::Start;
        }
    }

    for &InteractEntityEvent {
        client: attacker_client,
        entity: victim_client,
        ..
    } in interact_entity.read()
    {
        let true_victim_ent = clones
            .get(victim_client)
            .map(|cloned| cloned.0)
            .unwrap_or(victim_client);
        let Ok([mut attacker, mut victim]) =
            clients.get_many_mut([attacker_client, true_victim_ent])
        else {
            debug!("Failed to get clients for combat event");
            // Victim or attacker does not exist, or the attacker is attacking itself.
            continue;
        };

        if attacker.team == victim.team {
            // Attacker and victim are on the same team.
            continue;
        }

        if server.current_tick() - victim.state.last_attacked_tick < 10 {
            // Victim is still on attack cooldown.
            continue;
        }

        victim.state.last_attacked_tick = server.current_tick();

        let victim_pos = victim.pos.0.xz();
        let attacker_pos = attacker.pos.0.xz();

        let dir = (victim_pos - attacker_pos).normalize().as_vec2();

        let knockback_xz = if attacker.state.has_bonus_knockback {
            18.0
        } else {
            8.0
        };
        let knockback_y = if attacker.state.has_bonus_knockback {
            8.432
        } else {
            6.432
        };

        victim
            .client
            .set_velocity([dir.x * knockback_xz, knockback_y, dir.y * knockback_xz]);

        attacker.state.has_bonus_knockback = false;

        victim.client.trigger_status(EntityStatus::PlayAttackSound);
        victim.statuses.trigger(EntityStatus::PlayAttackSound);

        let stack = attacker.inventory.slot(attacker.held_item.slot());

        let damage = match stack.item {
            ItemKind::WoodenSword => 4.0,
            ItemKind::StoneSword => 5.0,
            ItemKind::IronSword => 6.0,
            ItemKind::DiamondSword => 7.0,
            _ => 1.0,
        };

        victim.health.0 -= damage;
    }
}

fn teleport_oob_clients(mut clients: Query<(&mut Position, &Team), With<Client>>) {
    for (mut pos, team) in &mut clients {
        if pos.0.y < 0.0 {
            pos.set(team.spawn_pos());
        }
    }
}

/// Handles respawning dead players.
fn necromancy(
    mut clients: Query<(
        &mut VisibleChunkLayer,
        &mut RespawnPosition,
        &Team,
        &mut Health,
    )>,
    mut events: EventReader<RequestRespawnEvent>,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for event in events.read() {
        if let Ok((mut visible_chunk_layer, mut respawn_pos, team, mut health)) =
            clients.get_mut(event.client)
        {
            respawn_pos.pos = team.spawn_pos().into();
            health.0 = PLAYER_MAX_HEALTH;

            let main_layer = layers.single();

            // this gets the client to get rid of the respawn screen
            visible_chunk_layer.0 = main_layer;
        }
    }
}

fn update_scoreboard(
    mut objectives: Query<&mut ObjectiveScores, With<Objective>>,
    score: Res<Score>,
) {
    if !score.is_changed() {
        return;
    }
    let mut s = objectives.single_mut();
    s.insert("Red", *score.scores.get(&Team::Red).unwrap_or(&0) as i32);
    s.insert("Blue", *score.scores.get(&Team::Blue).unwrap_or(&0) as i32);
}
