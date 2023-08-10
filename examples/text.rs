#![allow(clippy::type_complexity)]

use valence::lang::keys;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, despawn_disconnected_clients))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut Position,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut client,
        mut pos,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        pos.0 = [0.0, SPAWN_Y as f64 + 1.0, 0.0].into();
        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        *game_mode = GameMode::Creative;

        client.send_chat_message("Welcome to the text example.".bold());
        client.send_chat_message(
            "The following examples show ways to use the different text components.",
        );

        // Text examples
        client.send_chat_message("\nText");
        client.send_chat_message(" - ".into_text() + Text::text("Plain text"));
        client.send_chat_message(" - ".into_text() + Text::text("Styled text").italic());
        client.send_chat_message(" - ".into_text() + Text::text("Colored text").color(Color::GOLD));
        client.send_chat_message(
            " - ".into_text()
                + Text::text("Colored and styled text")
                    .color(Color::GOLD)
                    .italic()
                    .underlined(),
        );

        // Translated text examples
        client.send_chat_message("\nTranslated Text");
        client.send_chat_message(
            " - 'chat.type.advancement.task': ".into_text()
                + Text::translate(keys::CHAT_TYPE_ADVANCEMENT_TASK, []),
        );
        client.send_chat_message(
            " - 'chat.type.advancement.task' with slots: ".into_text()
                + Text::translate(
                    keys::CHAT_TYPE_ADVANCEMENT_TASK,
                    ["arg1".into(), "arg2".into()],
                ),
        );
        client.send_chat_message(
            " - 'custom.translation_key': ".into_text()
                + Text::translate("custom.translation_key", []),
        );

        // Scoreboard value example
        client.send_chat_message("\nScoreboard Values");
        client.send_chat_message(" - Score: ".into_text() + Text::score("*", "objective", None));
        client.send_chat_message(
            " - Score with custom value: ".into_text()
                + Text::score("*", "objective", Some("value".into())),
        );

        // Entity names example
        client.send_chat_message("\nEntity Names (Selector)");
        client.send_chat_message(" - Nearest player: ".into_text() + Text::selector("@p", None));
        client.send_chat_message(" - Random player: ".into_text() + Text::selector("@r", None));
        client.send_chat_message(" - All players: ".into_text() + Text::selector("@a", None));
        client.send_chat_message(" - All entities: ".into_text() + Text::selector("@e", None));
        client.send_chat_message(
            " - All entities with custom separator: ".into_text()
                + Text::selector("@e", Some(", ".into_text().color(Color::GOLD))),
        );

        // Keybind example
        client.send_chat_message("\nKeybind");
        client
            .send_chat_message(" - 'key.inventory': ".into_text() + Text::keybind("key.inventory"));

        // NBT examples
        client.send_chat_message("\nNBT");
        client.send_chat_message(
            " - Block NBT: ".into_text() + Text::block_nbt("{}", "0 1 0", None, None),
        );
        client.send_chat_message(
            " - Entity NBT: ".into_text() + Text::entity_nbt("{}", "@a", None, None),
        );
        client.send_chat_message(
            " - Storage NBT: ".into_text()
                + Text::storage_nbt(ident!("storage.key"), "@a", None, None),
        );

        client.send_chat_message(
            "\n\n↑ ".into_text().bold().color(Color::GOLD)
                + "Scroll up to see the full example!".into_text().not_bold(),
        );
    }
}
