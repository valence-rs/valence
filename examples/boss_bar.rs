#![allow(clippy::type_complexity)]

use rand::seq::SliceRandom;
use valence::prelude::*;
use valence_boss_bar::{
    BossBarBundle, BossBarColor, BossBarDivision, BossBarFlags, BossBarHealth, BossBarStyle,
    BossBarTitle,
};
use valence_server::entity::cow::CowEntityBundle;
use valence_server::message::ChatMessageEvent;
use valence_text::color::NamedColor;

const SPAWN_Y: i32 = 64;

#[derive(Component)]
struct CustomBossBar;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (init_clients, despawn_disconnected_clients, listen_messages),
        )
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

    let layer_id = commands.spawn(layer).id();

    commands.spawn((
        BossBarBundle {
            title: BossBarTitle("Boss Bar".into_text()),
            health: BossBarHealth(0.5),
            layer: EntityLayerId(layer_id),
            ..Default::default()
        },
        CustomBossBar,
    ));

    commands.spawn((
        CowEntityBundle {
            position: Position::new([0.0, SPAWN_Y as f64 + 1.0, 0.0]),
            layer: EntityLayerId(layer_id),
            ..Default::default()
        },
        BossBarTitle("Louis XVI".color(NamedColor::Red)),
        BossBarHealth(0.5),
        BossBarStyle {
            color: BossBarColor::Red,
            division: BossBarDivision::default(),
        },
        BossBarFlags::default(),
    ));
}

fn init_clients(
    mut clients_query: Query<
        (
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers_query: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    let layer = layers_query.single();

    for (
        mut client,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
    ) in &mut clients_query
    {
        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
        *game_mode = GameMode::Creative;

        client.send_chat_message(
            "Type 'view' to toggle bar display"
                .on_click_suggest_command("view")
                .on_hover_show_text("Type 'view'"),
        );
        client.send_chat_message(
            "Type 'color' to set a random color"
                .on_click_suggest_command("color")
                .on_hover_show_text("Type 'color'"),
        );
        client.send_chat_message(
            "Type 'division' to set a random division"
                .on_click_suggest_command("division")
                .on_hover_show_text("Type 'division'"),
        );
        client.send_chat_message(
            "Type 'flags' to set random flags"
                .on_click_suggest_command("flags")
                .on_hover_show_text("Type 'flags'"),
        );
        client.send_chat_message(
            "Type any string to set the title".on_click_suggest_command("title"),
        );
        client.send_chat_message(
            "Type any number between 0 and 1 to set the health".on_click_suggest_command("health"),
        );
    }
}

fn listen_messages(
    mut message_events: EventReader<ChatMessageEvent>,
    mut boss_bars_query: Query<
        (
            &mut BossBarStyle,
            &mut BossBarFlags,
            &mut BossBarHealth,
            &mut BossBarTitle,
            &EntityLayerId,
        ),
        With<CustomBossBar>,
    >,
    mut clients_query: Query<&mut VisibleEntityLayers, With<Client>>,
) {
    let (
        mut boss_bar_style,
        mut boss_bar_flags,
        mut boss_bar_health,
        mut boss_bar_title,
        entity_layer_id,
    ) = boss_bars_query.single_mut();

    for ChatMessageEvent {
        client, message, ..
    } in message_events.iter()
    {
        match message.as_ref() {
            "view" => {
                if let Ok(mut visible_entity_layers) = clients_query.get_mut(*client) {
                    if visible_entity_layers.0.contains(&entity_layer_id.0) {
                        visible_entity_layers.0.remove(&entity_layer_id.0);
                    } else {
                        visible_entity_layers.0.insert(entity_layer_id.0);
                    }
                }
            }
            "color" => {
                let mut colors = vec![
                    BossBarColor::Pink,
                    BossBarColor::Blue,
                    BossBarColor::Red,
                    BossBarColor::Green,
                    BossBarColor::Yellow,
                ];
                colors.retain(|c| *c != boss_bar_style.color);

                let random_color = colors.choose(&mut rand::thread_rng()).unwrap();

                boss_bar_style.color = *random_color;
            }
            "division" => {
                let mut divisions = vec![
                    BossBarDivision::NoDivision,
                    BossBarDivision::SixNotches,
                    BossBarDivision::TenNotches,
                    BossBarDivision::TwelveNotches,
                    BossBarDivision::TwentyNotches,
                ];
                divisions.retain(|d| *d != boss_bar_style.division);

                let random_division = divisions.choose(&mut rand::thread_rng()).unwrap();

                boss_bar_style.division = *random_division;
            }
            "flags" => {
                let darken_sky: bool = rand::random();
                let dragon_bar: bool = rand::random();
                let create_fog: bool = rand::random();

                boss_bar_flags.set_darken_sky(darken_sky);
                boss_bar_flags.set_dragon_bar(dragon_bar);
                boss_bar_flags.set_create_fog(create_fog);
            }
            _ => {
                if let Ok(health) = message.parse::<f32>() {
                    if (0.0..=1.0).contains(&health) {
                        boss_bar_health.0 = health;
                    }
                } else {
                    boss_bar_title.0 = message.to_string().into_text();
                }
            }
        };
    }
}
