use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::async_trait;
use valence::block::{BlockPos, BlockState};
use valence::client::{Client, ClientEvent, GameMode, Hand};
use valence::config::{Config, ServerListPing};
use valence::dimension::DimensionId;
use valence::entity::types::Pose;
use valence::entity::{Entity, EntityEvent, EntityId, EntityKind, TrackedData};
use valence::player_list::PlayerListId;
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::spatial_index::RaycastHit;
use valence::text::{Color, TextFormat};
use valence::util::from_yaw_and_pitch;
use vek::Vec3;

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

const MAX_PLAYERS: usize = 10;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -5);

const PLAYER_EYE_HEIGHT: f64 = 1.6;

#[async_trait]
impl Config for Game {
    type ChunkState = ();
    type ClientState = EntityId;
    /// `true` for entities that have been intersected with.
    type EntityState = bool;
    type PlayerListState = ();
    type ServerState = Option<PlayerListId>;
    type WorldState = ();

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
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("../assets/favicon.png")),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.insert(DimensionId::default(), ());
        server.state = Some(server.player_lists.insert(()).0);

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                world.chunks.insert([x, z], ());
            }
        }

        world.chunks.set_block_state(SPAWN_POS, BlockState::BEDROCK);

        const SHEEP_COUNT: usize = 10;
        for i in 0..SHEEP_COUNT {
            let offset = (i as f64 - (SHEEP_COUNT - 1) as f64 / 2.0) * 1.25;

            let (_, sheep) = server.entities.insert(EntityKind::Sheep, false);
            sheep.set_world(world_id);
            sheep.set_position([offset + 0.5, SPAWN_POS.y as f64 + 1.0, 0.0]);
            sheep.set_yaw(180.0);
            sheep.set_head_yaw(180.0);
        }
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

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
                    .insert_with_uuid(EntityKind::Player, client.uuid(), false)
                {
                    Some((id, _)) => client.state = id,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.spawn(world_id);
                client.set_flat(true);
                client.set_game_mode(GameMode::Creative);
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

                client.send_message(
                    "Look at a sheep to change its ".italic()
                        + "color".italic().color(Color::GREEN)
                        + ".",
                );
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                server.entities.remove(client.state);

                return false;
            }

            let client_pos = client.position();

            let origin = Vec3::new(client_pos.x, client_pos.y + PLAYER_EYE_HEIGHT, client_pos.z);
            let direction = from_yaw_and_pitch(client.yaw() as f64, client.pitch() as f64);
            let only_sheep = |hit: &RaycastHit| {
                server
                    .entities
                    .get(hit.entity)
                    .map_or(false, |e| e.kind() == EntityKind::Sheep)
            };

            if let Some(hit) = world.spatial_index.raycast(origin, direction, only_sheep) {
                if let Some(e) = server.entities.get_mut(hit.entity) {
                    e.state = true;
                }
            }

            while client_event_boilerplate(client, server.entities.get_mut(client.state).unwrap())
                .is_some()
            {}

            true
        });

        for (_, e) in server.entities.iter_mut() {
            let intersected = e.state;
            if let TrackedData::Sheep(sheep) = &mut e.data_mut() {
                if intersected {
                    sheep.set_color(5);
                } else {
                    sheep.set_color(0);
                }
            }
            e.state = false;
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
