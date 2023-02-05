use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::{default_event_handler, StartDigging, StartSneaking};
use valence_new::prelude::*;

const SIZE_X: i32 = 100;
const SIZE_Z: i32 = 100;
const BOARD_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()).with_biomes(vec![Biome {
            grass_color: Some(0x00ff00),
            ..Default::default()
        }]))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system_to_stage(EventLoop, alive_on_dig)
        .add_system(update_board)
        .add_system(pause_on_crouch)
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

    for z in 0..SIZE_Z {
        for x in 0..SIZE_X {
            instance.set_block_state([x, BOARD_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    world.spawn(instance);

    let board = LifeBoard {
        paused: false,
        board: vec![false; (SIZE_X * SIZE_Z) as usize].into_boxed_slice(),
        board_buf: vec![false; (SIZE_X * SIZE_Z) as usize].into_boxed_slice(),
    };
    world.insert_resource(board);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([
            SIZE_X as f64 / 2.0,
            BOARD_Y as f64 + 1.0,
            SIZE_Z as f64 / 2.0,
        ]);
        client.set_instance(instance);
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
    pub fn set(&mut self, x: i32, z: i32, value: bool) {
        self.board[Self::index(x, z).unwrap()] = value;
    }

    pub fn get(&self, x: i32, z: i32) -> bool {
        self.board[Self::index(x, z).unwrap()]
    }

    #[inline]
    fn index(x: i32, z: i32) -> Option<usize> {
        let x = x as usize;
        let z = z as usize;
        if x > SIZE_X as usize || z > SIZE_Z as usize {
            return None;
        }
        Some((x + z * SIZE_X as usize) % (SIZE_X * SIZE_Z) as usize)
    }

    fn count_live_neighbors(board: &Box<[bool]>, x: i32, z: i32) -> u8 {
        let mut count = 0;

        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }

                let Some(index) = Self::index(x + dx, z + dz) else {
                    continue;
                };

                if board[index] {
                    count += 1;
                }
            }
        }

        count
    }

    pub fn update(&mut self) {
        self.board_buf.iter_mut().enumerate().for_each(|(i, cell)| {
            let x = i as i32 % SIZE_X;
            let z = i as i32 / SIZE_Z;
            let neighbors = Self::count_live_neighbors(&self.board, x, z);
            let alive = &self.board[Self::index(x, z).unwrap()];

            let new_alive = match (alive, neighbors) {
                (true, n) if (2..=3).contains(&n) => true,
                (false, 3) => true,
                _ => false,
            };

            *cell = new_alive;
        });
        std::mem::swap(&mut self.board, &mut self.board_buf);
    }
}

fn alive_on_dig(mut events: EventReader<StartDigging>, mut board: ResMut<LifeBoard>) {
    for event in events.iter() {
        let (x, z) = (event.position.x, event.position.z);
        board.set(x, z, true);
    }
}

fn update_board(mut board: ResMut<LifeBoard>, mut instances: Query<&mut Instance>) {
    if !board.paused {
        board.update();
    }

    let mut instance = instances.get_single_mut().unwrap();

    for z in 0..SIZE_Z {
        for x in 0..SIZE_X {
            let alive = board.get(x, z);
            let block = if alive {
                BlockState::GRASS_BLOCK
            } else {
                BlockState::DIRT
            };
            instance.set_block_state([x, BOARD_Y, z], block);
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
                client.set_action_bar("Paused.".italic());
            } else {
                client.set_action_bar("Playing.".italic());
            }
        }
    }
}
