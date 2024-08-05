#![doc = include_str!("../README.md")]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use derive_more::{Deref, DerefMut};
use valence_server::client::{Client, FlushPacketsSet, UpdateClientsSet, VisibleChunkLayer};
use valence_server::protocol::packets::play::game_state_change_s2c::GameEventKind;
use valence_server::protocol::packets::play::GameStateChangeS2c;
use valence_server::protocol::WritePacket;
use valence_server::ChunkLayer;

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
#[derive(Component, Default, PartialEq, PartialOrd, Deref, DerefMut)]
pub struct Rain(pub f32);

/// Component containing the thunder level. Valid values are in \[0, 1] with 0
/// being no rain and 1 being full rain.
#[derive(Component, Default, PartialEq, PartialOrd, Deref, DerefMut)]
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
