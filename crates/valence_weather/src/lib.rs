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
use derive_more::{Deref, DerefMut};
use valence_server::client::Client;
use valence_server::layer::message::LayerMessages;
use valence_server::layer::{BroadcastLayerMessagesSet, OldVisibleLayers, VisibleLayers};
use valence_server::protocol::packets::play::game_state_change_s2c::GameEventKind;
use valence_server::protocol::packets::play::GameStateChangeS2c;
use valence_server::protocol::WritePacket;

pub struct WeatherPlugin;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateWeatherSet;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(
            PostUpdate,
            UpdateWeatherSet.before(BroadcastLayerMessagesSet),
        )
        .add_systems(
            PostUpdate,
            (
                init_weather_on_layer_join,
                update_rain_level,
                update_thunder_level,
            )
                .in_set(UpdateWeatherSet),
        );
    }
}

/// Bundle containing rain and thunder components. This can be added to any
/// layer.
#[derive(Bundle, Default, Debug)]
pub struct WeatherBundle {
    pub rain: Rain,
    pub thunder: Thunder,
}

/// Component containing the rain level. Valid values are in \[0, 1] with 0
/// being no rain and 1 being full rain.
#[derive(Component, Default, PartialEq, PartialOrd, Deref, DerefMut, Debug)]
pub struct Rain(pub f32);

/// Component containing the thunder level. Valid values are in \[0, 1] with 0
/// being no rain and 1 being full rain.
#[derive(Component, Default, PartialEq, PartialOrd, Deref, DerefMut, Debug)]
pub struct Thunder(pub f32);

fn init_weather_on_layer_join(
    mut clients: Query<(&mut Client, &VisibleLayers, &OldVisibleLayers), Changed<VisibleLayers>>,
    layers: Query<(Option<&Rain>, Option<&Thunder>)>,
) {
    for (mut client, vis_layers, old_vis_layers) in &mut clients {
        if let Some((rain, thunder)) = vis_layers
            .difference(old_vis_layers)
            .find_map(|&layer| layers.get(layer).ok())
        {
            if let Some(rain) = rain {
                client.write_packet(&GameStateChangeS2c {
                    kind: GameEventKind::RainLevelChange,
                    value: rain.0,
                });
            }

            if let Some(thunder) = thunder {
                client.write_packet(&GameStateChangeS2c {
                    kind: GameEventKind::ThunderLevelChange,
                    value: thunder.0,
                });
            }
        }
    }
}

fn update_rain_level(mut layers: Query<(&mut LayerMessages, &Rain), Changed<Rain>>) {
    for (mut layer, rain) in &mut layers {
        layer.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: rain.0,
        });
    }
}

fn update_thunder_level(mut layers: Query<(&mut LayerMessages, &Thunder), Changed<Rain>>) {
    for (mut layer, thunder) in &mut layers {
        layer.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: thunder.0,
        });
    }
}
