use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::async_trait;
use valence::block::{BlockPos, BlockState};
use valence::chunk::UnloadedChunk;
use valence::client::{handle_event_default, GameMode};
use valence::config::{Config, ServerListPing};
use valence::dimension::DimensionId;
use valence::entity::{types, EntityId, EntityKind, TrackedData};
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

const PLAYER_EYE_HEIGHT: f64 = 1.62;

// TODO
// const PLAYER_SNEAKING_EYE_HEIGHT: f64 = 1.495;

#[async_trait]
impl Config for Game {
    type ServerState = Option<PlayerListId>;
    type ClientState = EntityId;
    type EntityState = ();
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
        let (world_id, world) = server.worlds.insert(DimensionId::default(), ());
        server.state = Some(server.player_lists.insert(()).0);

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                world.chunks.insert([x, z], UnloadedChunk::default(), ());
            }
        }

        world.chunks.set_block_state(SPAWN_POS, BlockState::BEDROCK);

        // Item Frames
        let (_, e) = server.entities.insert(EntityKind::ItemFrame, ());
        if let TrackedData::ItemFrame(i) = e.data_mut() {
            i.set_rotation(types::Facing::North as i32);
        }
        e.set_world(world_id);
        e.set_position(Vec3::new(2f64, 102f64, 0f64));

        let (_, e) = server.entities.insert(EntityKind::ItemFrame, ());
        if let TrackedData::ItemFrame(i) = e.data_mut() {
            i.set_rotation(types::Facing::West as i32);
        }
        e.set_world(world_id);
        e.set_position(Vec3::new(4f64, 102f64, 0f64));

        // Paintings
        let (_, e) = server.entities.insert(EntityKind::Painting, ());
        if let TrackedData::Painting(p) = e.data_mut() {
            p.set_variant(types::PaintingKind::Graham);
        }
        e.set_world(world_id);
        e.set_position(Vec3::new(0f64, 101f64, 0f64));
        e.set_yaw(-180f32);

        let (_, e) = server.entities.insert(EntityKind::Painting, ());
        if let TrackedData::Painting(p) = e.data_mut() {
            p.set_variant(types::PaintingKind::DonkeyKong);
        }
        e.set_world(world_id);
        e.set_position(Vec3::new(0f64, 102f64, -10f64));
        e.set_yaw(0f32);

        // Shulkers
        let (_, e) = server.entities.insert(EntityKind::Shulker, ());
        if let TrackedData::Shulker(s) = e.data_mut() {
            s.set_peek_amount(100);
            s.set_attached_face(types::Facing::West);
        }
        e.set_world(world_id);
        e.set_position(Vec3::new(-3f64, 102f64, 0f64));

        let (_, e) = server.entities.insert(EntityKind::Shulker, ());
        if let TrackedData::Shulker(s) = e.data_mut() {
            s.set_peek_amount(32);
            s.set_attached_face(types::Facing::South);
        }
        e.set_world(world_id);
        e.set_position(Vec3::new(-5f64, 102f64, 0f64));

        let (_, e) = server.entities.insert(EntityKind::Shulker, ());
        if let TrackedData::Shulker(s) = e.data_mut() {
            s.set_peek_amount(0);
            s.set_attached_face(types::Facing::Down);
        }
        e.set_world(world_id);
        e.set_position(Vec3::new(-7f64, 102f64, 0f64));

        // Sheep
        let (_, e) = server.entities.insert(EntityKind::Sheep, ());
        if let TrackedData::Sheep(sheep) = e.data_mut() {
            sheep.set_color(6);
            sheep.set_child(true);
        }
        e.set_world(world_id);
        e.set_position(Vec3::new(-5f64, 101f64, -4.5f64));
        e.set_yaw(-90f32);
        e.set_head_yaw(-90f32);

        let (_, e) = server.entities.insert(EntityKind::Sheep, ());
        if let TrackedData::Sheep(sheep) = e.data_mut() {
            sheep.set_color(6);
        }
        e.set_world(world_id);
        e.set_position(Vec3::new(5f64, 101f64, -4.5f64));
        e.set_yaw(90f32);
        e.set_head_yaw(90f32);
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
                    .insert_with_uuid(EntityKind::Player, client.uuid(), ())
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
                    "Press ".italic()
                        + "F3 + B".italic().color(Color::AQUA)
                        + " to show hitboxes.".italic(),
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
            let not_player = |hit: &RaycastHit| {
                server
                    .entities
                    .get(hit.entity)
                    .map_or(false, |e| e.kind() != EntityKind::Player)
            };

            if let Some(_) = world.spatial_index.raycast(origin, direction, not_player) {
                client.set_action_bar("Intersection".color(Color::GREEN));
            } else {
                client.set_action_bar("No Intersection".color(Color::RED))
            }

            while handle_event_default(client, server.entities.get_mut(client.state).unwrap())
                .is_some()
            {}

            true
        });
    }
}
