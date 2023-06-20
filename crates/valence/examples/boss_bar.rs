use rand::seq::SliceRandom;
use valence::prelude::*;
use valence_boss_bar::{BossBarViewers, BossBarColor, BossBarStyle, BossBarDivision, BossBarFlags, BossBarHealth, BossBarTitle, BossBarBundle};
use valence_client::message::{SendMessage, ChatMessageEvent};

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(listen_messages)
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

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

    commands.spawn(BossBarBundle::new(Text::text("Boss bar"), BossBarColor::Blue, BossBarDivision::TenNotches, BossBarFlags::new()));

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(Entity, &mut Client, &mut Location, &mut Position, &mut GameMode), Added<Client>>,
    mut boss_bar_viewers: Query<&mut BossBarViewers>,
    instances: Query<Entity, With<Instance>>,
) {
    let mut boss_bar_viewers = boss_bar_viewers.single_mut();
    for (entity, mut client, mut loc, mut pos, mut game_mode) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
        *game_mode = GameMode::Creative;

        client.send_chat_message("Type 'view' to toggle bar display".on_click_suggest_command("view").on_hover_show_text("Type 'view'"));
        client.send_chat_message("Type 'color' to set a random color".on_click_suggest_command("color").on_hover_show_text("Type 'color'"));
        client.send_chat_message("Type 'division' to set a random division".on_click_suggest_command("division").on_hover_show_text("Type 'division'"));
        client.send_chat_message("Type 'flags' to set random flags".on_click_suggest_command("flags").on_hover_show_text("Type 'flags'"));
        client.send_chat_message("Type any string to set the title".on_click_suggest_command("title"));
        client.send_chat_message("Type any number between 0 and 1 to set the health".on_click_suggest_command("health"));

        boss_bar_viewers.viewers.insert(entity);
    }
}

fn listen_messages(mut message_events: EventReader<ChatMessageEvent>, mut boss_bar: Query<(&mut BossBarViewers, &mut BossBarStyle, &mut BossBarFlags, &mut BossBarHealth, &mut BossBarTitle)>) {
    let (mut boss_bar_viewers, mut boss_bar_style, mut boss_bar_flags, mut boss_bar_health, mut boss_bar_title) = boss_bar.single_mut();

    let events: Vec<ChatMessageEvent> = message_events.iter().cloned().collect();
    for ChatMessageEvent { client, message, .. } in events.iter() {

        match message.as_ref() {
            "view" => {
                if boss_bar_viewers.viewers.contains(client) {
                    boss_bar_viewers.viewers.remove(client);
                } else {
                    boss_bar_viewers.viewers.insert(*client);
                }
            },
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
            },
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
            },
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
                    if health >= 0.0 && health <= 1.0 {
                        boss_bar_health.0 = health;
                    }
                } else {
                    boss_bar_title.0 = message.to_string().into();
                }
            }
        };
    }
}