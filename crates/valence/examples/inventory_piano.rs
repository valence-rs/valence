use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use num::Integer;
pub use valence::prelude::*;
use valence_protocol::types::ClickContainerMode;

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState { player_list: None },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
}

const MAX_PLAYERS: usize = 10;

const SIZE_X: usize = 100;
const SIZE_Z: usize = 100;

const SLOT_MIN: i16 = 36;
const SLOT_MAX: i16 = 43;
const PITCH_MIN: f32 = 0.5;
const PITCH_MAX: f32 = 1.0;

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = ClientState;
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();
    type InventoryState = ();

    fn dimensions(&self) -> Vec<Dimension> {
        vec![Dimension {
            fixed_time: Some(6000),
            ..Dimension::default()
        }]
    }

    async fn server_list_ping(
        &self,
        _server: &SharedServer<Self>,
        _remote_addr: SocketAddr,
        _protocol_version: i32,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: self.player_count.load(Ordering::SeqCst) as i32,
            max_players: MAX_PLAYERS as i32,
            player_sample: Default::default(),
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("../../../assets/logo-64x64.png").as_slice().into()),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let world = server.worlds.insert(DimensionId::default(), ()).1;
        server.state.player_list = Some(server.player_lists.insert(()).0);

        // initialize chunks
        for chunk_z in -2..Integer::div_ceil(&(SIZE_Z as i32), &16) + 2 {
            for chunk_x in -2..Integer::div_ceil(&(SIZE_X as i32), &16) + 2 {
                world
                    .chunks
                    .insert([chunk_x, chunk_z], UnloadedChunk::default(), ());
            }
        }

        // initialize blocks in the chunks
        for chunk_x in 0..Integer::div_ceil(&SIZE_X, &16) {
            for chunk_z in 0..Integer::div_ceil(&SIZE_Z, &16) {
                let chunk = world
                    .chunks
                    .get_mut([chunk_x as i32, chunk_z as i32])
                    .unwrap();
                for x in 0..16 {
                    for z in 0..16 {
                        let cell_x = chunk_x * 16 + x;
                        let cell_z = chunk_z * 16 + z;

                        if cell_x < SIZE_X && cell_z < SIZE_Z {
                            chunk.set_block_state(x, 63, z, BlockState::GRASS_BLOCK);
                        }
                    }
                }
            }
        }
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, _) = server.worlds.iter_mut().next().unwrap();

        let spawn_pos = [SIZE_X as f64 / 2.0, 1.0, SIZE_Z as f64 / 2.0];

        server.clients.retain(|_, client| {
            if client.created_this_tick() {
                if self
                    .player_count
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                        (count < MAX_PLAYERS).then_some(count + 1)
                    })
                    .is_err()
                {
                    client.disconnect("The server is full!".color(Color::RED));
                    return false;
                }

                match server
                    .entities
                    .insert_with_uuid(EntityKind::Player, client.uuid(), ())
                {
                    Some((id, _)) => client.state.entity_id = id,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.respawn(world_id);
                client.set_flat(true);
                client.teleport(spawn_pos, 0.0, 0.0);
                client.set_player_list(server.state.player_list.clone());

                if let Some(id) = &server.state.player_list {
                    server.player_lists[id].insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                        true,
                    );
                }

                client.set_game_mode(GameMode::Creative);
                client.send_message(
                    "Welcome to Valence! Open your inventory, and click on your hotbar to play \
                     the piano."
                        .italic(),
                );
                client.send_message(
                    "Click the rightmost hotbar slot to toggle between creative and survival."
                        .italic(),
                );
            }

            let player = server.entities.get_mut(client.state.entity_id).unwrap();

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                player.set_deleted(true);
                if let Some(id) = &server.state.player_list {
                    server.player_lists[id].remove(client.uuid());
                }
                return false;
            }

            if client.position().y <= -20.0 {
                client.teleport(spawn_pos, client.yaw(), client.pitch());
            }

            while let Some(event) = client.next_event() {
                match event {
                    ClientEvent::CloseContainer { .. } => {
                        client.send_message("Done already?");
                    }
                    ClientEvent::SetCreativeModeSlot { slot, .. } => {
                        client.send_message(format!("{event:#?}"));
                        // If the user does a double click, 3 notes will be played.
                        // This is not possible to fix :(
                        play_note(client, player, slot);
                    }
                    ClientEvent::ClickContainer { slot_id, mode, .. } => {
                        client.send_message(format!("{event:#?}"));
                        if mode != ClickContainerMode::Click {
                            // Prevent notes from being played twice if the user clicks quickly
                            continue;
                        }
                        play_note(client, player, slot_id);
                    }
                    _ => {}
                }
            }

            true
        });
    }
}

fn play_note(client: &mut Client<Game>, player: &mut Entity<Game>, clicked_slot: i16) {
    if (SLOT_MIN..=SLOT_MAX).contains(&clicked_slot) {
        let pitch = (clicked_slot - SLOT_MIN) as f32 * (PITCH_MAX - PITCH_MIN)
            / (SLOT_MAX - SLOT_MIN) as f32
            + PITCH_MIN;

        client.send_message(format!("playing note with pitch: {pitch}"));

        let _ = player;
        // client.play_sound(
        //     Ident::new("block.note_block.harp").unwrap(),
        //     SoundCategory::Block,
        //     player.position(),
        //     10.0,
        //     pitch,
        // );
    } else if clicked_slot == 44 {
        client.set_game_mode(match client.game_mode() {
            GameMode::Survival => GameMode::Creative,
            GameMode::Creative => GameMode::Survival,
            _ => GameMode::Creative,
        });
    }
}
