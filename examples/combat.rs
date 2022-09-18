use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::block::{BlockPos, BlockState};
use valence::chunk::{Chunk, UnloadedChunk};
use valence::client::{
    handle_event_default, ClientEvent, ClientId, GameMode, InteractWithEntityKind,
};
use valence::config::{Config, ServerListPing};
use valence::dimension::DimensionId;
use valence::entity::{EntityEvent, EntityId, EntityKind};
use valence::player_list::PlayerListId;
use valence::retain::RetainDecision;
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::text::{Color, TextFormat};
use valence::{async_trait, Ticks};
use vek::{Vec2, Vec3};

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        None,
    )
}

struct Game {
    player_count: AtomicUsize,
}

#[derive(Default)]
struct ClientState {
    /// The client's player entity.
    player: EntityId,
    /// The extra knockback on the first hit while sprinting.
    extra_knockback: bool,
}

#[derive(Default)]
struct EntityState {
    client: ClientId,
    attacked: bool,
    attacker_pos: Vec3<f64>,
    extra_knockback: bool,
    last_attack_time: Ticks,
}

const MAX_PLAYERS: usize = 10;

const SPAWN_POS: BlockPos = BlockPos::new(0, 20, 0);

#[async_trait]
impl Config for Game {
    type ServerState = Option<PlayerListId>;
    type ClientState = ClientState;
    type EntityState = EntityState;
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
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
            favicon_png: Some(include_bytes!("../assets/logo-64x64.png").as_slice().into()),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let (_, world) = server.worlds.insert(DimensionId::default(), ());
        server.state = Some(server.player_lists.insert(()).0);

        let dim = server.shared.dimension(DimensionId::default());
        let min_y = dim.min_y;
        let height = dim.height as usize;

        // Create circular arena.
        let size = 2;
        for chunk_z in -size - 2..size + 2 {
            for chunk_x in -size - 2..size + 2 {
                let mut chunk = UnloadedChunk::new(height);

                let r = -size..size;
                if r.contains(&chunk_x) && r.contains(&chunk_z) {
                    for z in 0..16 {
                        for x in 0..16 {
                            let block_x = chunk_x * 16 + x as i32;
                            let block_z = chunk_z * 16 + z as i32;
                            if f64::hypot(block_x as f64, block_z as f64) <= size as f64 * 16.0 {
                                for y in 0..(SPAWN_POS.y - min_y + 1) as usize {
                                    chunk.set_block_state(x, y, z, BlockState::STONE);
                                }
                            }
                        }
                    }
                }

                world.chunks.insert([chunk_x, chunk_z], chunk, ());
            }
        }

        world.chunks.set_block_state(SPAWN_POS, BlockState::BEDROCK);
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, _) = server.worlds.iter_mut().next().unwrap();

        let current_tick = server.shared.current_tick();

        server.clients.retain(|client_id, client| {
            if client.created_this_tick() {
                if self
                    .player_count
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                        (count < MAX_PLAYERS).then_some(count + 1)
                    })
                    .is_err()
                {
                    client.disconnect("The server is full!".color(Color::RED));
                    return RetainDecision::Remove;
                }

                let (player_id, player) = match server.entities.insert_with_uuid(
                    EntityKind::Player,
                    client.uuid(),
                    EntityState::default(),
                ) {
                    Some(e) => e,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return RetainDecision::Remove;
                    }
                };

                player.state.client = client_id;

                client.state.player = player_id;
                client.state.extra_knockback = true;

                client.spawn(world_id);
                client.set_flat(true);
                client.set_game_mode(GameMode::Survival);
                client.teleport(
                    [
                        SPAWN_POS.x as f64 + 0.5,
                        SPAWN_POS.y as f64 + 1.0,
                        SPAWN_POS.z as f64 + 0.5,
                    ],
                    0.0,
                    0.0,
                );
                client.set_player_list(server.state.clone());

                if let Some(id) = &server.state {
                    server.player_lists.get_mut(id).insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                    );
                }

                client.send_message("Welcome to the arena.".italic());
                if self.player_count.load(Ordering::SeqCst) <= 1 {
                    client.send_message("Have another player join the game with you.".italic());
                }
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities.remove(client.state.player);
                if let Some(id) = &server.state {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                return RetainDecision::Remove;
            }

            if client.position().y <= 0.0 {
                client.teleport(
                    [
                        SPAWN_POS.x as f64 + 0.5,
                        SPAWN_POS.y as f64 + 1.0,
                        SPAWN_POS.z as f64 + 0.5,
                    ],
                    client.yaw(),
                    client.pitch(),
                );
            }

            loop {
                let player = server
                    .entities
                    .get_mut(client.state.player)
                    .expect("missing player entity");

                match handle_event_default(client, player) {
                    Some(ClientEvent::StartSprinting) => {
                        client.state.extra_knockback = true;
                    }
                    Some(ClientEvent::StopSprinting) => {
                        client.state.extra_knockback = false;
                    }
                    Some(ClientEvent::InteractWithEntity {
                        id,
                        kind: InteractWithEntityKind::Attack,
                        ..
                    }) => {
                        if let Some(target) = server.entities.get_mut(id) {
                            if !target.state.attacked
                                && current_tick - target.state.last_attack_time >= 10
                                && id != client.state.player
                            {
                                target.state.attacked = true;
                                target.state.attacker_pos = client.position();
                                target.state.extra_knockback = client.state.extra_knockback;
                                target.state.last_attack_time = current_tick;

                                client.state.extra_knockback = false;
                            }
                        }
                    }
                    Some(_) => {}
                    None => break,
                }
            }

            RetainDecision::Keep
        });

        for (_, entity) in server.entities.iter_mut() {
            if entity.state.attacked {
                entity.state.attacked = false;
                if let Some(victim) = server.clients.get_mut(entity.state.client) {
                    let victim_pos = Vec2::new(victim.position().x, victim.position().z);
                    let attacker_pos =
                        Vec2::new(entity.state.attacker_pos.x, entity.state.attacker_pos.z);

                    let dir = (victim_pos - attacker_pos).normalized();

                    let knockback_xz = if entity.state.extra_knockback {
                        18.0
                    } else {
                        8.0
                    };
                    let knockback_y = if entity.state.extra_knockback {
                        8.432
                    } else {
                        6.432
                    };

                    let vel = Vec3::new(dir.x * knockback_xz, knockback_y, dir.y * knockback_xz);
                    victim.set_velocity(vel.as_());

                    entity.push_event(EntityEvent::DamageFromGenericSource);
                    entity.push_event(EntityEvent::Damage);
                    victim.push_entity_event(EntityEvent::DamageFromGenericSource);
                    victim.push_entity_event(EntityEvent::Damage);
                }
            }
        }
    }
}
