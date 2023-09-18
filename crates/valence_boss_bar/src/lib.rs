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

use std::borrow::Cow;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use derive_more::{Deref, DerefMut};
use valence_server::client::Client;
use valence_server::layer::message::LayerMessages;
use valence_server::layer::{BroadcastLayerMessagesSet, OldVisibleLayers, VisibleLayers};
pub use valence_server::protocol::packets::play::boss_bar_s2c::{
    BossBarAction, BossBarColor, BossBarDivision, BossBarFlags,
};
use valence_server::protocol::packets::play::BossBarS2c;
use valence_server::protocol::WritePacket;
use valence_server::{Despawned, LayerId, OldLayerId, Text, UniqueId};

pub struct BossBarPlugin;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct BossBarSet;

impl Plugin for BossBarPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.configure_set(PostUpdate, BossBarSet.before(BroadcastLayerMessagesSet))
            .add_systems(
                PostUpdate,
                (
                    init_boss_bar_for_client,
                    update_boss_bar_layer,
                    update_boss_bar_title,
                    update_boss_bar_health,
                    update_boss_bar_style,
                    update_boss_bar_flags,
                    despawn_boss_bar,
                )
                    .chain()
                    .in_set(BossBarSet),
            );
    }
}

/// The bundle of components that make up a boss bar.
#[derive(Bundle, Default)]
pub struct BossBarBundle {
    pub uuid: UniqueId,
    pub title: BossBarTitle,
    pub health: BossBarHealth,
    pub color: BossBarColor,
    pub division: BossBarDivision,
    pub flags: BossBarFlags,
    pub layer: LayerId,
    pub old_layer: OldLayerId,
}

/// The title of a boss bar.
#[derive(Component, Clone, Default, Deref, DerefMut)]
pub struct BossBarTitle(pub Text);

/// The health of a boss bar.
#[derive(Component, Default, Deref, DerefMut)]
pub struct BossBarHealth(pub f32);

#[derive(WorldQuery)]
struct FullBossBarQuery {
    uuid: &'static UniqueId,
    title: &'static BossBarTitle,
    health: &'static BossBarHealth,
    color: &'static BossBarColor,
    division: &'static BossBarDivision,
    flags: &'static BossBarFlags,
    layer: &'static LayerId,
}

fn init_boss_bar_for_client(
    mut clients: Query<(&mut Client, &VisibleLayers, &OldVisibleLayers), Changed<VisibleLayers>>,
    boss_bars: Query<FullBossBarQuery>,
) {
    for (mut client, layers, old_layers) in &mut clients {
        // TODO: this could be improved with fragmenting relations.

        // Remove boss bars from old layers.
        for &layer in old_layers.difference(&layers) {
            for bb in &boss_bars {
                if bb.layer.0 == layer {
                    client.write_packet(&BossBarS2c {
                        id: bb.uuid.0,
                        action: BossBarAction::Remove,
                    });
                }
            }
        }

        // Add boss bars from new layers.
        for &layer in layers.difference(&old_layers) {
            for bb in &boss_bars {
                if bb.layer.0 == layer {
                    client.write_packet(&BossBarS2c {
                        id: bb.uuid.0,
                        action: BossBarAction::Add {
                            title: Cow::Borrowed(&bb.title.0),
                            health: bb.health.0,
                            color: *bb.color,
                            division: *bb.division,
                            flags: *bb.flags,
                        },
                    });
                }
            }
        }
    }
}

fn update_boss_bar_layer(
    boss_bars: Query<(FullBossBarQuery, &OldLayerId), Changed<LayerId>>,
    mut layers: Query<&mut LayerMessages>,
) {
    for (bb, old_layer) in &boss_bars {
        // Remove from old layer.
        if let Ok(mut msgs) = layers.get_mut(old_layer.get()) {
            msgs.write_packet(&BossBarS2c {
                id: bb.uuid.0,
                action: BossBarAction::Remove,
            })
        }

        // Init in new layer.
        if let Ok(mut msgs) = layers.get_mut(bb.layer.0) {
            msgs.write_packet(&BossBarS2c {
                id: bb.uuid.0,
                action: BossBarAction::Add {
                    title: Cow::Borrowed(&bb.title.0),
                    health: bb.health.0,
                    color: *bb.color,
                    division: *bb.division,
                    flags: *bb.flags,
                },
            });
        }
    }
}

fn update_boss_bar_title(
    boss_bars: Query<(&UniqueId, &LayerId, &BossBarTitle), Changed<BossBarTitle>>,
    mut layers: Query<&mut LayerMessages>,
) {
    for (uuid, layer, title) in &boss_bars {
        if let Ok(mut msgs) = layers.get_mut(layer.0) {
            msgs.write_packet(&BossBarS2c {
                id: uuid.0,
                action: BossBarAction::UpdateTitle(Cow::Borrowed(&title.0)),
            });
        }
    }
}

fn update_boss_bar_health(
    boss_bars: Query<(&UniqueId, &LayerId, &BossBarHealth), Changed<BossBarHealth>>,
    mut layers: Query<&mut LayerMessages>,
) {
    for (uuid, layer, health) in &boss_bars {
        if let Ok(mut msgs) = layers.get_mut(layer.0) {
            msgs.write_packet(&BossBarS2c {
                id: uuid.0,
                action: BossBarAction::UpdateHealth(health.0),
            });
        }
    }
}

fn update_boss_bar_style(
    boss_bars: Query<
        (&UniqueId, &LayerId, &BossBarColor, &BossBarDivision),
        Or<(Changed<BossBarColor>, Changed<BossBarDivision>)>,
    >,
    mut layers: Query<&mut LayerMessages>,
) {
    for (uuid, layer, color, division) in &boss_bars {
        if let Ok(mut msgs) = layers.get_mut(layer.0) {
            msgs.write_packet(&BossBarS2c {
                id: uuid.0,
                action: BossBarAction::UpdateStyle(*color, *division),
            });
        }
    }
}

fn update_boss_bar_flags(
    boss_bars: Query<(&UniqueId, &LayerId, &BossBarFlags), Changed<BossBarFlags>>,
    mut layers: Query<&mut LayerMessages>,
) {
    for (uuid, layer, flags) in &boss_bars {
        if let Ok(mut msgs) = layers.get_mut(layer.0) {
            msgs.write_packet(&BossBarS2c {
                id: uuid.0,
                action: BossBarAction::UpdateFlags(*flags),
            });
        }
    }
}

fn despawn_boss_bar(
    boss_bars: Query<(&UniqueId, &LayerId), With<Despawned>>,
    mut layers: Query<&mut LayerMessages>,
) {
    for (uuid, layer_id) in &boss_bars {
        if let Ok(mut msgs) = layers.get_mut(layer_id.0) {
            msgs.write_packet(&BossBarS2c {
                id: uuid.0,
                action: BossBarAction::Remove,
            });
        }
    }
}
