#![allow(clippy::type_complexity)]

use std::mem;

use valence::prelude::*;

const BOARD_MIN_X: i32 = -30;
const BOARD_MAX_X: i32 = 30;
const BOARD_MIN_Z: i32 = -30;
const BOARD_MAX_Z: i32 = 30;
const BOARD_Y: i32 = 64;

const BOARD_SIZE_X: usize = (BOARD_MAX_X - BOARD_MIN_X + 1) as usize;
const BOARD_SIZE_Z: usize = (BOARD_MAX_Z - BOARD_MIN_Z + 1) as usize;

const SPAWN_POS: DVec3 = DVec3::new(
    (BOARD_MIN_X + BOARD_MAX_X) as f64 / 2.0,
    BOARD_Y as f64 + 1.0,
    (BOARD_MIN_Z + BOARD_MAX_Z) as f64 / 2.0,
);

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                toggle_cell_on_dig,
                update_board,
                pause_on_crouch,
                reset_oob_clients,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: ResMut<DimensionTypeRegistry>,
    mut biomes: ResMut<BiomeRegistry>,
) {
    for (_, _, biome) in biomes.iter_mut() {
        biome.effects.grass_color = Some(0x00ff00);
    }

    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -10..10 {
        for x in -10..10 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in BOARD_MIN_Z..=BOARD_MAX_Z {
        for x in BOARD_MIN_X..=BOARD_MAX_X {
            layer.chunk.set_block([x, BOARD_Y, z], BlockState::DIRT);
        }
    }

    commands.spawn(layer);

    commands.insert_resource(LifeBoard {
        playing: false,
        board: vec![false; BOARD_SIZE_X * BOARD_SIZE_Z].into(),
        board_buf: vec![false; BOARD_SIZE_X * BOARD_SIZE_Z].into(),
    });
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
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut client,
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
        pos.set([0.0, 65.0, 0.0]);
        *game_mode = GameMode::Survival;

        client.send_chat_message("Welcome to Conway's game of life in Minecraft!".italic());
        client.send_chat_message(
            "Sneak to toggle running the simulation and the left mouse button to bring blocks to \
             life."
                .italic(),
        );
    }
}

#[derive(Resource)]
struct LifeBoard {
    pub playing: bool,
    board: Box<[bool]>,
    board_buf: Box<[bool]>,
}

impl LifeBoard {
    pub fn get(&self, x: i32, z: i32) -> bool {
        if (BOARD_MIN_X..=BOARD_MAX_X).contains(&x) && (BOARD_MIN_Z..=BOARD_MAX_Z).contains(&z) {
            let x = (x - BOARD_MIN_X) as usize;
            let z = (z - BOARD_MIN_Z) as usize;

            self.board[x + z * BOARD_SIZE_X]
        } else {
            false
        }
    }

    pub fn set(&mut self, x: i32, z: i32, value: bool) {
        if (BOARD_MIN_X..=BOARD_MAX_X).contains(&x) && (BOARD_MIN_Z..=BOARD_MAX_Z).contains(&z) {
            let x = (x - BOARD_MIN_X) as usize;
            let z = (z - BOARD_MIN_Z) as usize;

            self.board[x + z * BOARD_SIZE_X] = value;
        }
    }

    pub fn update(&mut self) {
        for (idx, cell) in self.board_buf.iter_mut().enumerate() {
            let x = (idx % BOARD_SIZE_X) as i32;
            let z = (idx / BOARD_SIZE_X) as i32;

            let mut live_neighbors = 0;

            for cz in z - 1..=z + 1 {
                for cx in x - 1..=x + 1 {
                    if !(cx == x && cz == z) {
                        let idx = cx.rem_euclid(BOARD_SIZE_X as i32) as usize
                            + cz.rem_euclid(BOARD_SIZE_Z as i32) as usize * BOARD_SIZE_X;

                        live_neighbors += self.board[idx] as i32;
                    }
                }
            }

            let live = self.board[idx];
            if live {
                *cell = (2..=3).contains(&live_neighbors);
            } else {
                *cell = live_neighbors == 3;
            }
        }

        mem::swap(&mut self.board, &mut self.board_buf);
    }

    pub fn clear(&mut self) {
        self.board.fill(false);
    }
}

fn toggle_cell_on_dig(mut events: EventReader<DiggingEvent>, mut board: ResMut<LifeBoard>) {
    for event in events.iter() {
        if event.state == DiggingState::Start {
            let (x, z) = (event.position.x, event.position.z);

            let live = board.get(x, z);
            board.set(x, z, !live);
        }
    }
}

fn update_board(
    mut board: ResMut<LifeBoard>,
    mut layers: Query<&mut ChunkLayer>,
    server: Res<Server>,
) {
    if board.playing && server.current_tick() % 2 == 0 {
        board.update();
    }

    let mut layer = layers.single_mut();

    for z in BOARD_MIN_Z..=BOARD_MAX_Z {
        for x in BOARD_MIN_X..=BOARD_MAX_X {
            let block = if board.get(x, z) {
                BlockState::GRASS_BLOCK
            } else {
                BlockState::DIRT
            };

            layer.set_block([x, BOARD_Y, z], block);
        }
    }
}

fn pause_on_crouch(
    mut events: EventReader<SneakEvent>,
    mut board: ResMut<LifeBoard>,
    mut layers: Query<&mut EntityLayer>,
) {
    for event in events.iter() {
        if event.state == SneakState::Start {
            let mut layer = layers.single_mut();

            if board.playing {
                board.playing = false;
                layer.set_action_bar("Paused".italic().color(Color::RED));
            } else {
                board.playing = true;
                layer.set_action_bar("Playing".italic().color(Color::GREEN));
            }
        }
    }
}

fn reset_oob_clients(
    mut clients: Query<&mut Position, With<Client>>,
    mut board: ResMut<LifeBoard>,
) {
    for mut pos in &mut clients {
        if pos.0.y < 0.0 {
            pos.0 = SPAWN_POS;
            board.clear();
        }
    }
}
