use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use valence::prelude::*;

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

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
    type InventoryState = ();

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
            favicon_png: Some(
                include_bytes!("../../../assets/logo-64x64.png")
                    .as_slice()
                    .into(),
            ),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let (_, world) = server.worlds.insert(DimensionId::default(), ());
        server.state = Some(server.player_lists.insert(()).0);

        let min_y = world.chunks.min_y();
        let height = world.chunks.height();

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
        let current_tick = server.current_tick();
        let (world_id, _) = server.worlds.iter_mut().next().unwrap();

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
                    return false;
                }

                let (player_id, player) = match server.entities.insert_with_uuid(
                    EntityKind::Player,
                    client.uuid(),
                    EntityState::default(),
                ) {
                    Some(e) => e,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                };

                player.set_world(world_id);
                player.client = client_id;

                client.player = player_id;

                client.respawn(world_id);
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

                client.send_message("Welcome to the arena.".italic());
                if self.player_count.load(Ordering::SeqCst) <= 1 {
                    client.send_message("Have another player join the game with you.".italic());
                }
            }

            while let Some(event) = client.next_event() {
                let player = server
                    .entities
                    .get_mut(client.player)
                    .expect("missing player entity");

                event.handle_default(client, player);
                match event {
                    ClientEvent::StartSprinting => {
                        client.extra_knockback = true;
                    }
                    ClientEvent::StopSprinting => {
                        client.extra_knockback = false;
                    }
                    ClientEvent::InteractWithEntity { entity_id, .. } => {
                        if let Some((id, target)) = server.entities.get_with_raw_id_mut(entity_id) {
                            if !target.attacked
                                && current_tick - target.last_attack_time >= 10
                                && id != client.player
                            {
                                target.attacked = true;
                                target.attacker_pos = client.position();
                                target.extra_knockback = client.extra_knockback;
                                target.last_attack_time = current_tick;

                                client.extra_knockback = false;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities[client.player].set_deleted(true);
                if let Some(id) = &server.state {
                    server.player_lists[id].remove(client.uuid());
                }
                return false;
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

            true
        });

        for (_, entity) in server.entities.iter_mut() {
            if entity.attacked {
                entity.attacked = false;
                if let Some(victim) = server.clients.get_mut(entity.client) {
                    let victim_pos = Vec2::new(victim.position().x, victim.position().z);
                    let attacker_pos = Vec2::new(entity.attacker_pos.x, entity.attacker_pos.z);

                    let dir = (victim_pos - attacker_pos).normalized();

                    let knockback_xz = if entity.extra_knockback { 18.0 } else { 8.0 };
                    let knockback_y = if entity.extra_knockback { 8.432 } else { 6.432 };

                    let vel = Vec3::new(dir.x * knockback_xz, knockback_y, dir.y * knockback_xz);
                    victim.set_velocity(vel.as_());

                    entity.push_event(EntityEvent::DamageFromGenericSource);
                    entity.push_event(EntityEvent::Damage);
                    victim.send_entity_event(EntityEvent::DamageFromGenericSource);
                    victim.send_entity_event(EntityEvent::Damage);
                }
            }
        }
    }
}
