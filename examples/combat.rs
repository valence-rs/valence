use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::block::{BlockPos, BlockState};
use valence::client::{Client, ClientEvent, ClientId, GameMode, Hand, InteractWithEntityKind};
use valence::config::{Config, ServerListPing};
use valence::dimension::DimensionId;
use valence::entity::types::Pose;
use valence::entity::{Entity, EntityEvent, EntityId, EntityKind, TrackedData};
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
        (),
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
    type ChunkState = ();
    type ClientState = ClientState;
    type EntityState = EntityState;
    type ServerState = ();
    type WorldState = ();

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
    }

    fn online_mode(&self) -> bool {
        false
    }

    async fn server_list_ping(
        &self,
        _server: &SharedServer<Self>,
        _remote_addr: SocketAddr,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: self.player_count.load(Ordering::SeqCst) as i32,
            max_players: MAX_PLAYERS as i32,
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("../assets/favicon.png")),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let (_, world) = server.worlds.create(DimensionId::default(), ());
        world.meta.set_flat(true);

        let min_y = server.shared.dimension(DimensionId::default()).min_y;

        // Create circular arena.
        let size = 2;
        for chunk_z in -size - 2..size + 2 {
            for chunk_x in -size - 2..size + 2 {
                let chunk = world.chunks.create([chunk_x, chunk_z], ());
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
            }
        }

        world.chunks.set_block_state(SPAWN_POS, BlockState::BEDROCK);
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

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
                    return false;
                }

                let (player_id, player) = match server.entities.create_with_uuid(
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

                player.state.client = client_id;

                client.state.player = player_id;
                client.state.extra_knockback = true;

                client.spawn(world_id);
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

                world.meta.player_list_mut().insert(
                    client.uuid(),
                    client.username().to_owned(),
                    client.textures().cloned(),
                    client.game_mode(),
                    0,
                    None,
                );

                client.send_message("Welcome to the arena.".italic());
                if self.player_count.load(Ordering::SeqCst) <= 1 {
                    client.send_message("Have another player join the game with you.".italic());
                }
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities.delete(client.state.player);
                world.meta.player_list_mut().remove(client.uuid());
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

            loop {
                let player = server
                    .entities
                    .get_mut(client.state.player)
                    .expect("missing player entity");

                match client_event_boilerplate(client, player) {
                    Some(ClientEvent::StartSprinting) => {
                        client.state.extra_knockback = true;
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

            true
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

fn client_event_boilerplate(
    client: &mut Client<Game>,
    entity: &mut Entity<Game>,
) -> Option<ClientEvent> {
    let event = client.pop_event()?;

    match &event {
        ClientEvent::ChatMessage { .. } => {}
        ClientEvent::SettingsChanged {
            view_distance,
            main_hand,
            displayed_skin_parts,
            ..
        } => {
            client.set_view_distance(*view_distance);

            let player = client.player_mut();

            player.set_cape(displayed_skin_parts.cape());
            player.set_jacket(displayed_skin_parts.jacket());
            player.set_left_sleeve(displayed_skin_parts.left_sleeve());
            player.set_right_sleeve(displayed_skin_parts.right_sleeve());
            player.set_left_pants_leg(displayed_skin_parts.left_pants_leg());
            player.set_right_pants_leg(displayed_skin_parts.right_pants_leg());
            player.set_hat(displayed_skin_parts.hat());
            player.set_main_arm(*main_hand as u8);

            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_cape(displayed_skin_parts.cape());
                player.set_jacket(displayed_skin_parts.jacket());
                player.set_left_sleeve(displayed_skin_parts.left_sleeve());
                player.set_right_sleeve(displayed_skin_parts.right_sleeve());
                player.set_left_pants_leg(displayed_skin_parts.left_pants_leg());
                player.set_right_pants_leg(displayed_skin_parts.right_pants_leg());
                player.set_hat(displayed_skin_parts.hat());
                player.set_main_arm(*main_hand as u8);
            }
        }
        ClientEvent::MovePosition {
            position,
            on_ground,
        } => {
            entity.set_position(*position);
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MovePositionAndRotation {
            position,
            yaw,
            pitch,
            on_ground,
        } => {
            entity.set_position(*position);
            entity.set_yaw(*yaw);
            entity.set_head_yaw(*yaw);
            entity.set_pitch(*pitch);
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MoveRotation {
            yaw,
            pitch,
            on_ground,
        } => {
            entity.set_yaw(*yaw);
            entity.set_head_yaw(*yaw);
            entity.set_pitch(*pitch);
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MoveOnGround { on_ground } => {
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MoveVehicle { .. } => {}
        ClientEvent::StartSneaking => {
            if let TrackedData::Player(player) = entity.data_mut() {
                if player.get_pose() == Pose::Standing {
                    player.set_pose(Pose::Sneaking);
                }
            }
        }
        ClientEvent::StopSneaking => {
            if let TrackedData::Player(player) = entity.data_mut() {
                if player.get_pose() == Pose::Sneaking {
                    player.set_pose(Pose::Standing);
                }
            }
        }
        ClientEvent::StartSprinting => {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_sprinting(true);
            }
        }
        ClientEvent::StopSprinting => {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_sprinting(false);
            }
        }
        ClientEvent::StartJumpWithHorse { .. } => {}
        ClientEvent::StopJumpWithHorse => {}
        ClientEvent::LeaveBed => {}
        ClientEvent::OpenHorseInventory => {}
        ClientEvent::StartFlyingWithElytra => {}
        ClientEvent::ArmSwing(hand) => {
            entity.push_event(match hand {
                Hand::Main => EntityEvent::SwingMainHand,
                Hand::Off => EntityEvent::SwingOffHand,
            });
        }
        ClientEvent::InteractWithEntity { .. } => {}
        ClientEvent::SteerBoat { .. } => {}
        ClientEvent::Digging { .. } => {}
    }

    entity.set_world(client.world());

    Some(event)
}
