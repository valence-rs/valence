#![allow(clippy::type_complexity)]

use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::prelude::*;
use valence::protocol::translation_key;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(default_event_handler.in_schedule(EventLoopSchedule))
        .add_systems(PlayerList::default_systems())
        .add_system(despawn_disconnected_clients)
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

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Client, &mut Position, &mut Location, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, mut pos, mut loc, mut game_mode) in &mut clients {
        pos.0 = [0.0, SPAWN_Y as f64 + 1.0, 0.0].into();
        loc.0 = instances.single();
        *game_mode = GameMode::Creative;

        client.send_message("Welcome to the text example.".bold());
        client
            .send_message("The following examples show ways to use the different text components.");

        // Text examples
        client.send_message("\nText");
        client.send_message(" - ".into_text() + Text::text("Plain text"));
        client.send_message(" - ".into_text() + Text::text("Styled text").italic());
        client.send_message(" - ".into_text() + Text::text("Colored text").color(Color::GOLD));
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
        client.send_message(" - 'key.inventory': ".into_text() + Text::keybind("key.inventory"));

        // NBT examples
        client.send_message("\nNBT");
        client.send_message(
            " - Block NBT: ".into_text() + Text::block_nbt("{}", "0 1 0", None, None),
        );
        client
            .send_message(" - Entity NBT: ".into_text() + Text::entity_nbt("{}", "@a", None, None));
        client.send_message(
            " - Storage NBT: ".into_text()
                + Text::storage_nbt(ident!("storage.key"), "@a", None, None),
        );

        client.send_message(
            "\n\n↑ ".into_text().bold().color(Color::GOLD)
                + "Scroll up to see the full example!".into_text().not_bold(),
        );
    }
}
