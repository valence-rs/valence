use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use valence::prelude::*;
use valence_protocol::entity_meta::{Facing, PaintingKind};
use valence_spatial_index::bvh::Bvh;
use valence_spatial_index::{RaycastHit, SpatialIndex, WithAabb};

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState {
            player_list: None,
            bvh: Bvh::new(),
            world: WorldId::NULL,
        },
    )
}

struct Game {
    player_count: AtomicUsize,
}

#[derive(Default)]
struct ClientState {
    player: EntityId,
    shulker_bullet: EntityId,
}

const MAX_PLAYERS: usize = 10;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -5);

const PLAYER_EYE_HEIGHT: f64 = 1.62;

// TODO
// const PLAYER_SNEAKING_EYE_HEIGHT: f64 = 1.495;

struct ServerState {
    player_list: Option<PlayerListId>,
    bvh: Bvh<WithAabb<EntityId>>,
    world: WorldId,
}

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = ClientState;
    type EntityState = ();
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
            favicon_png: Some(include_bytes!("../assets/logo-64x64.png").as_slice().into()),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.insert(DimensionId::default(), ());
        server.state.world = world_id;

        let (player_list_id, player_list) = server.player_lists.insert(());
        server.state.player_list = Some(player_list_id);

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                world.chunks.insert([x, z], UnloadedChunk::default(), ());
            }
        }

        world.chunks.set_block_state(SPAWN_POS, BlockState::BEDROCK);

        // ==== Item Frames ==== //
        let (_, e) = server.entities.insert(EntityKind::ItemFrame, ());
        if let TrackedData::ItemFrame(i) = e.data_mut() {
            i.set_rotation(Facing::North as i32);
        }
        e.set_world(world_id);
        e.set_position([2.0, 102.0, 0.0]);

        let (_, e) = server.entities.insert(EntityKind::ItemFrame, ());
        if let TrackedData::ItemFrame(i) = e.data_mut() {
            i.set_rotation(Facing::East as i32);
        }
        e.set_world(world_id);
        e.set_position([3.0, 102.0, 0.0]);

        let (_, e) = server.entities.insert(EntityKind::GlowItemFrame, ());
        if let TrackedData::GlowItemFrame(i) = e.data_mut() {
            i.set_rotation(Facing::South as i32);
        }
        e.set_world(world_id);
        e.set_position([4.0, 102.0, 0.0]);

        let (_, e) = server.entities.insert(EntityKind::GlowItemFrame, ());
        if let TrackedData::GlowItemFrame(i) = e.data_mut() {
            i.set_rotation(Facing::West as i32);
        }
        e.set_world(world_id);
        e.set_position([5.0, 102.0, 0.0]);

        // ==== Paintings ==== //
        let (_, e) = server.entities.insert(EntityKind::Painting, ());
        if let TrackedData::Painting(p) = e.data_mut() {
            p.set_variant(PaintingKind::Pigscene);
        }
        e.set_world(world_id);
        e.set_yaw(180.0);
        e.set_position([0.0, 102.0, 0.0]);

        let (_, e) = server.entities.insert(EntityKind::Painting, ());
        if let TrackedData::Painting(p) = e.data_mut() {
            p.set_variant(PaintingKind::DonkeyKong);
        }
        e.set_world(world_id);
        e.set_yaw(90.0);
        e.set_position([-4.0, 102.0, 0.0]);

        let (_, e) = server.entities.insert(EntityKind::Painting, ());
        if let TrackedData::Painting(p) = e.data_mut() {
            p.set_variant(PaintingKind::Void);
        }
        e.set_world(world_id);
        e.set_position([-6.0, 102.0, 0.0]);

        let (_, e) = server.entities.insert(EntityKind::Painting, ());
        if let TrackedData::Painting(p) = e.data_mut() {
            p.set_variant(PaintingKind::Aztec);
        }
        e.set_yaw(270.0);
        e.set_world(world_id);
        e.set_position([-7.0, 102.0, 0.0]);

        // ==== Shulkers ==== //
        let (_, e) = server.entities.insert(EntityKind::Shulker, ());
        if let TrackedData::Shulker(s) = e.data_mut() {
            s.set_peek_amount(100);
            s.set_attached_face(Facing::West);
        }
        e.set_world(world_id);
        e.set_position([-4.0, 102.0, -8.0]);

        let (_, e) = server.entities.insert(EntityKind::Shulker, ());
        if let TrackedData::Shulker(s) = e.data_mut() {
            s.set_peek_amount(75);
            s.set_attached_face(Facing::Up);
        }
        e.set_world(world_id);
        e.set_position([-1.0, 102.0, -8.0]);

        let (_, e) = server.entities.insert(EntityKind::Shulker, ());
        if let TrackedData::Shulker(s) = e.data_mut() {
            s.set_peek_amount(50);
            s.set_attached_face(Facing::Down);
        }
        e.set_world(world_id);
        e.set_position([2.0, 102.0, -8.0]);

        let (_, e) = server.entities.insert(EntityKind::Shulker, ());
        if let TrackedData::Shulker(s) = e.data_mut() {
            s.set_peek_amount(25);
            s.set_attached_face(Facing::East);
        }
        e.set_world(world_id);
        e.set_position([5.0, 102.0, -8.0]);

        let (_, e) = server.entities.insert(EntityKind::Shulker, ());
        if let TrackedData::Shulker(s) = e.data_mut() {
            s.set_peek_amount(0);
            s.set_attached_face(Facing::North);
        }
        e.set_world(world_id);
        e.set_position([8.0, 102.0, -8.0]);

        // ==== Slimes ==== //
        let (_, e) = server.entities.insert(EntityKind::Slime, ());
        if let TrackedData::Slime(s) = e.data_mut() {
            s.set_slime_size(30);
        }
        e.set_world(world_id);
        e.set_yaw(180.0);
        e.set_head_yaw(180.0);
        e.set_position([12.0, 102.0, 10.0]);

        let (_, e) = server.entities.insert(EntityKind::MagmaCube, ());
        if let TrackedData::MagmaCube(m) = e.data_mut() {
            m.set_slime_size(30);
        }
        e.set_world(world_id);
        e.set_yaw(180.0);
        e.set_head_yaw(180.0);
        e.set_position([-12.0, 102.0, 10.0]);

        // ==== Sheep ==== //
        let (_, e) = server.entities.insert(EntityKind::Sheep, ());
        if let TrackedData::Sheep(s) = e.data_mut() {
            s.set_color(6);
            s.set_child(true);
        }
        e.set_world(world_id);
        e.set_position([-5.0, 101.0, -4.5]);
        e.set_yaw(270.0);
        e.set_head_yaw(270.0);

        let (_, e) = server.entities.insert(EntityKind::Sheep, ());
        if let TrackedData::Sheep(s) = e.data_mut() {
            s.set_color(6);
        }
        e.set_world(world_id);
        e.set_position([5.0, 101.0, -4.5]);
        e.set_yaw(90.0);
        e.set_head_yaw(90.0);

        // ==== Players ==== //
        let player_poses = [
            Pose::Standing,
            Pose::Sneaking,
            Pose::FallFlying,
            Pose::Sleeping,
            Pose::Swimming,
            Pose::SpinAttack,
            Pose::Dying,
        ];

        for (i, pose) in player_poses.into_iter().enumerate() {
            player_list.insert(
                Uuid::from_u128(i as u128),
                format!("fake_player_{i}"),
                None,
                GameMode::Survival,
                0,
                None,
            );

            let (_, e) = server
                .entities
                .insert_with_uuid(EntityKind::Player, Uuid::from_u128(i as u128), ())
                .unwrap();
            if let TrackedData::Player(p) = e.data_mut() {
                p.set_pose(pose);
            }
            e.set_world(world_id);
            e.set_position([-3.0 + i as f64 * 2.0, 104.0, -9.0]);
        }

        // ==== Warden ==== //
        let (_, e) = server.entities.insert(EntityKind::Warden, ());
        e.set_world(world_id);
        e.set_position([-7.0, 102.0, -4.5]);
        e.set_yaw(270.0);
        e.set_head_yaw(270.0);

        let (_, e) = server.entities.insert(EntityKind::Warden, ());
        if let TrackedData::Warden(w) = e.data_mut() {
            w.set_pose(Pose::Emerging);
        }
        e.set_world(world_id);
        e.set_position([-7.0, 102.0, -6.5]);
        e.set_yaw(270.0);
        e.set_head_yaw(270.0);

        // ==== Goat ==== //
        let (_, e) = server.entities.insert(EntityKind::Goat, ());
        e.set_world(world_id);
        e.set_position([5.0, 103.0, -4.5]);
        e.set_yaw(270.0);
        e.set_head_yaw(90.0);

        let (_, e) = server.entities.insert(EntityKind::Goat, ());
        if let TrackedData::Goat(g) = e.data_mut() {
            g.set_pose(Pose::LongJumping);
        }
        e.set_world(world_id);
        e.set_position([5.0, 103.0, -3.5]);
        e.set_yaw(270.0);
        e.set_head_yaw(90.0);

        // ==== Giant ==== //
        let (_, e) = server.entities.insert(EntityKind::Giant, ());
        e.set_world(world_id);
        e.set_position([20.0, 101.0, -5.0]);
        e.set_yaw(270.0);
        e.set_head_yaw(90.0);
    }

    fn update(&self, server: &mut Server<Self>) {
        let world_id = server.state.world;

        // Rebuild our BVH every tick. All of the entities are in the same world.
        server.state.bvh.rebuild(
            server
                .entities
                .iter()
                .map(|(id, entity)| WithAabb::new(id, entity.hitbox())),
        );

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
                    Some((id, entity)) => {
                        entity.set_world(world_id);
                        client.player = id
                    }
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.respawn(world_id);
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
                client.set_player_list(server.state.player_list.clone());

                if let Some(id) = &server.state.player_list {
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

            let player = &mut server.entities[client.player];

            while let Some(event) = client.next_event() {
                event.handle_default(client, player);
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                player.set_deleted(true);

                return false;
            }

            let client_pos = client.position();

            let origin = Vec3::new(client_pos.x, client_pos.y + PLAYER_EYE_HEIGHT, client_pos.z);
            let direction = from_yaw_and_pitch(client.yaw() as f64, client.pitch() as f64);
            let not_self_or_bullet = |hit: RaycastHit<WithAabb<EntityId>>| {
                hit.object.object != client.player && hit.object.object != client.shulker_bullet
            };

            if let Some(hit) = server
                .state
                .bvh
                .raycast(origin, direction, not_self_or_bullet)
            {
                let bullet = if let Some(bullet) = server.entities.get_mut(client.shulker_bullet) {
                    bullet
                } else {
                    let (id, bullet) = server.entities.insert(EntityKind::ShulkerBullet, ());
                    client.shulker_bullet = id;
                    bullet.set_world(world_id);
                    bullet
                };

                let mut hit_pos = origin + direction * hit.near;
                let hitbox = bullet.hitbox();

                hit_pos.y -= (hitbox.max.y - hitbox.min.y) / 2.0;

                bullet.set_position(hit_pos);

                client.set_action_bar("Intersection".color(Color::GREEN));
            } else {
                server.entities.delete(client.shulker_bullet);
                client.set_action_bar("No Intersection".color(Color::RED));
            }

            true
        });
    }
}
