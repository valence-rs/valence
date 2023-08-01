#![doc = include_str!("../README.md")]
#![allow(clippy::type_complexity)]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_client::packet::{GameEventKind, GameStateChangeS2c};
use valence_client::{Client, FlushPacketsSet, UpdateClientsSet, VisibleChunkLayer};
use valence_core::protocol::encode::WritePacket;
use valence_layer::ChunkLayer;

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                init_weather_on_layer_join,
                change_client_rain_level,
                change_client_thunder_level,
            )
                .before(FlushPacketsSet),
        )
        .add_systems(
            PostUpdate,
            (change_layer_rain_level, change_layer_thunder_level).before(UpdateClientsSet),
        );
    }
}

/// Bundle containing rain and thunder components. `valence_weather` allows this
/// to be added to clients and chunk layer entities.
#[derive(Bundle, Default, PartialEq, PartialOrd)]
pub struct WeatherBundle {
    pub rain: Rain,
    pub thunder: Thunder,
}

/// Component containing the rain level. Valid values are in \[0, 1] with 0
/// being no rain and 1 being full rain.
#[derive(Component, Default, PartialEq, PartialOrd)]
pub struct Rain(pub f32);

/// Component containing the thunder level. Valid values are in \[0, 1] with 0
/// being no rain and 1 being full rain.
#[derive(Component, Default, PartialEq, PartialOrd)]
pub struct Thunder(pub f32);

fn init_weather_on_layer_join(
    mut clients: Query<(&mut Client, &VisibleChunkLayer), Changed<VisibleChunkLayer>>,
    layers: Query<(Option<&Rain>, Option<&Thunder>), With<ChunkLayer>>,
) {
    for (mut client, visible_chunk_layer) in &mut clients {
        if let Ok((rain, thunder)) = layers.get(visible_chunk_layer.0) {
            if let Some(rain) = rain {
                if rain.0 != 0.0 {
                    client.write_packet(&GameStateChangeS2c {
                        kind: GameEventKind::RainLevelChange,
                        value: rain.0,
                    });
                }
            }

            if let Some(thunder) = thunder {
                if thunder.0 != 0.0 {
                    client.write_packet(&GameStateChangeS2c {
                        kind: GameEventKind::ThunderLevelChange,
                        value: thunder.0,
                    });
                }
            }
        }
    }
}

fn change_layer_rain_level(
    mut layers: Query<(&mut ChunkLayer, &Rain), (Changed<Rain>, Without<Client>)>,
) {
    for (mut layer, rain) in &mut layers {
        layer.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: rain.0,
        });
    }
}

fn change_layer_thunder_level(
    mut layers: Query<(&mut ChunkLayer, &Thunder), (Changed<Thunder>, Without<Client>)>,
) {
    for (mut layer, thunder) in &mut layers {
        layer.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: thunder.0,
        });
    }
}

fn change_client_rain_level(mut clients: Query<(&mut Client, &Rain), Changed<Rain>>) {
    for (mut client, rain) in &mut clients {
        client.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: rain.0,
        });
    }
}

fn change_client_thunder_level(mut clients: Query<(&mut Client, &Thunder), Changed<Thunder>>) {
    for (mut client, thunder) in &mut clients {
        client.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: thunder.0,
        });
    }
}
