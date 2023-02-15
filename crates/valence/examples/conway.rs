use std::mem;

use valence::client::despawn_disconnected_clients;
use valence::client::event::{default_event_handler, StartDigging, StartSneaking};
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
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()).with_biomes(vec![Biome {
            grass_color: Some(0x00ff00),
            ..Default::default()
        }]))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_set(PlayerList::default_system_set())
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system_to_stage(EventLoop, toggle_cell_on_dig)
        .add_system(update_board)
        .add_system(pause_on_crouch)
        .add_system(reset_oob_clients)
        .run();
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    for z in -10..10 {
        for x in -10..10 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in BOARD_MIN_Z..=BOARD_MAX_Z {
        for x in BOARD_MIN_X..=BOARD_MAX_X {
            instance.set_block([x, BOARD_Y, z], BlockState::DIRT);
        }
    }

    world.spawn(instance);

    world.insert_resource(LifeBoard {
        paused: true,
        board: vec![false; BOARD_SIZE_X * BOARD_SIZE_Z].into(),
        board_buf: vec![false; BOARD_SIZE_X * BOARD_SIZE_Z].into(),
    });
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for mut client in &mut clients {
        client.set_position(SPAWN_POS);
        client.set_instance(instances.single());
        client.set_game_mode(GameMode::Survival);

        client.send_message("Welcome to Conway's game of life in Minecraft!".italic());
        client.send_message(
            "Sneak to toggle running the simulation and the left mouse button to bring blocks to \
             life."
                .italic(),
        );
    }
}

#[derive(Resource)]
struct LifeBoard {
    pub paused: bool,
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

fn toggle_cell_on_dig(mut events: EventReader<StartDigging>, mut board: ResMut<LifeBoard>) {
    for event in events.iter() {
        let (x, z) = (event.position.x, event.position.z);

        let live = board.get(x, z);
        board.set(x, z, !live);
    }
}

fn update_board(
    mut board: ResMut<LifeBoard>,
    mut instances: Query<&mut Instance>,
    server: Res<Server>,
) {
    if !board.paused && server.current_tick() % 2 == 0 {
        board.update();
    }

    let mut instance = instances.single_mut();

    for z in BOARD_MIN_Z..=BOARD_MAX_Z {
        for x in BOARD_MIN_X..=BOARD_MAX_X {
            let block = if board.get(x, z) {
                BlockState::GRASS_BLOCK
            } else {
                BlockState::DIRT
            };

            instance.set_block([x, BOARD_Y, z], block);
        }
    }
}

fn pause_on_crouch(
    mut events: EventReader<StartSneaking>,
    mut board: ResMut<LifeBoard>,
    mut clients: Query<&mut Client>,
) {
    for _ in events.iter() {
        board.paused = !board.paused;

        for mut client in clients.iter_mut() {
            if board.paused {
                client.set_action_bar("Paused".italic().color(Color::RED));
            } else {
                client.set_action_bar("Playing".italic().color(Color::GREEN));
            }
        }
    }
}

fn reset_oob_clients(mut clients: Query<&mut Client>, mut board: ResMut<LifeBoard>) {
    for mut client in &mut clients {
        if client.position().y < 0.0 {
            client.set_position(SPAWN_POS);
            board.clear();
        }
    }
}
