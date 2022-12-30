use std::net::SocketAddr;

use valence::prelude::*;

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

    valence::start_server(Game::default(), ServerState::default())
}

#[derive(Default)]
struct Game {}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
}

#[derive(Default)]
struct ServerState {
    world: WorldId,
}

const FLOOR_Y: i32 = 1;
const PLATFORM_X: i32 = 3;
const PLATFORM_Z: i32 = 3;
const SPAWN_POS: Vec3<f64> = Vec3::new(1.5, 2.0, 1.5);

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
            online_players: -1,
            max_players: -1,
            description: "Hello Valence! ".into_text() + "Text Example".color(Color::AQUA),
            favicon_png: Some(
                include_bytes!("../../../assets/logo-64x64.png")
                    .as_slice()
                    .into(),
            ),
            player_sample: Default::default(),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        server.state = ServerState {
            world: create_world(server),
        };
    }

    fn update(&self, server: &mut Server<Self>) {
        server.clients.retain(|_, client| {
            if client.created_this_tick() {
                // Boilerplate for client initialization
                match server
                    .entities
                    .insert_with_uuid(EntityKind::Player, client.uuid(), ())
                {
                    Some((id, _)) => client.entity_id = id,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                let world_id = server.state.world;

                client.set_flat(true);
                client.respawn(world_id);
                client.teleport(SPAWN_POS, -90.0, 0.0);
                client.set_game_mode(GameMode::Creative);

                client.send_message("Welcome to the text example.".bold());
                client.send_message(
                    "The following examples show ways to use the different text components.",
                );

                // Text examples
                client.send_message("\nText");
                client.send_message(" - ".into_text() + Text::text("Plain text"));
                client.send_message(" - ".into_text() + Text::text("Styled text").italic());
                client.send_message(
                    " - ".into_text() + Text::text("Colored text").color(Color::GOLD),
                );
                client.send_message(
                    " - ".into_text()
                        + Text::text("Colored and styled text")
                            .color(Color::GOLD)
                            .italic()
                            .underlined(),
                );

                // Translated text examples
                client.send_message("\nTranslated Text");
                client.send_message(
                    " - 'chat.type.advancement.task': ".into_text()
                        + Text::translate(translation_key::CHAT_TYPE_ADVANCEMENT_TASK, []),
                );
                client.send_message(
                    " - 'chat.type.advancement.task' with slots: ".into_text()
                        + Text::translate(
                            translation_key::CHAT_TYPE_ADVANCEMENT_TASK,
                            ["arg1".into(), "arg2".into()],
                        ),
                );
                client.send_message(
                    " - 'custom.translation_key': ".into_text()
                        + Text::translate("custom.translation_key", []),
                );

                // Scoreboard value example
                client.send_message("\nScoreboard Values");
                client.send_message(" - Score: ".into_text() + Text::score("*", "objective", None));
                client.send_message(
                    " - Score with custom value: ".into_text()
                        + Text::score("*", "objective", Some("value".into())),
                );

                // Entity names example
                client.send_message("\nEntity Names (Selector)");
                client.send_message(" - Nearest player: ".into_text() + Text::selector("@p", None));
                client.send_message(" - Random player: ".into_text() + Text::selector("@r", None));
                client.send_message(" - All players: ".into_text() + Text::selector("@a", None));
                client.send_message(" - All entities: ".into_text() + Text::selector("@e", None));
                client.send_message(
                    " - All entities with custom separator: ".into_text()
                        + Text::selector("@e", Some(", ".into_text().color(Color::GOLD))),
                );

                // Keybind example
                client.send_message("\nKeybind");
                client.send_message(
                    " - 'key.inventory': ".into_text() + Text::keybind("key.inventory"),
                );

                // NBT examples
                client.send_message("\nNBT");
                client.send_message(
                    " - Block NBT: ".into_text() + Text::block_nbt("{}", "0 1 0", None, None),
                );
                client.send_message(
                    " - Entity NBT: ".into_text() + Text::entity_nbt("{}", "@a", None, None),
                );
                client.send_message(
                    " - Storage NBT: ".into_text()
                        + Text::storage_nbt(ident!("storage.key"), "@a", None, None),
                );

                client.send_message(
                    "\n\n↑ ".into_text().bold().color(Color::GOLD)
                        + "Scroll up to see the full example!".into_text().not_bold(),
                );
            }

            if client.position().y < 0.0 {
                client.teleport(SPAWN_POS, 0.0, 0.0);
            }

            let player = server.entities.get_mut(client.entity_id).unwrap();

            while let Some(event) = client.next_event() {
                event.handle_default(client, player);
            }

            if client.is_disconnected() {
                player.set_deleted(true);
                return false;
            }

            true
        });
    }
}

// Boilerplate for creating world
fn create_world(server: &mut Server<Game>) -> WorldId {
    let dimension = server.shared.dimensions().next().unwrap();

    let (world_id, world) = server.worlds.insert(dimension.0, ());

    // Create chunks
    for z in -3..3 {
        for x in -3..3 {
            world.chunks.insert([x, z], UnloadedChunk::default(), ());
        }
    }

    // Create platform
    let platform_block = BlockState::GLASS;

    for z in 0..PLATFORM_Z {
        for x in 0..PLATFORM_X {
            world
                .chunks
                .set_block_state([x, FLOOR_Y, z], platform_block);
        }
    }

    world_id
}
