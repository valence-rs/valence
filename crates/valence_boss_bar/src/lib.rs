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
use valence_client::{Client, FlushPacketsSet, OldVisibleEntityLayers, VisibleEntityLayers};
use valence_core::despawn::Despawned;
use valence_core::uuid::UniqueId;
use valence_packet::packets::play::boss_bar_s2c::{BossBarAction, ToPacketAction};
use valence_packet::packets::play::BossBarS2c;
use valence_packet::protocol::encode::WritePacket;

mod components;
pub use components::*;
use valence_entity::{EntityLayerId, Position};
use valence_layer::{EntityLayer, Layer};

pub struct BossBarPlugin;

impl Plugin for BossBarPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            (
                update_boss_bar::<BossBarTitle>,
                update_boss_bar::<BossBarHealth>,
                update_boss_bar::<BossBarStyle>,
                update_boss_bar::<BossBarFlags>,
                update_boss_bar_view,
                boss_bar_despawn,
            )
                .before(FlushPacketsSet),
        );
    }
}

fn update_boss_bar<T: Component + ToPacketAction>(
    boss_bars_query: Query<(&UniqueId, &T, &EntityLayerId, Option<&Position>), Changed<T>>,
    mut entity_layers_query: Query<&mut EntityLayer>,
) {
    for (id, part, entity_layer_id, pos) in boss_bars_query.iter() {
        if let Ok(mut entity_layer) = entity_layers_query.get_mut(entity_layer_id.0) {
            let packet = BossBarS2c {
                id: id.0,
                action: part.to_packet_action(),
            };
            if let Some(pos) = pos {
                entity_layer
                    .view_writer(pos.to_chunk_pos())
                    .write_packet(&packet);
            } else {
                entity_layer.write_packet(&packet);
            }
        }
    }
}

fn update_boss_bar_view(
    mut clients_query: Query<
        (&mut Client, &VisibleEntityLayers, &OldVisibleEntityLayers),
        Changed<VisibleEntityLayers>,
    >,
    boss_bars_query: Query<(
        &UniqueId,
        &BossBarTitle,
        &BossBarHealth,
        &BossBarStyle,
        &BossBarFlags,
        &EntityLayerId,
    )>,
) {
    for (mut client, visible_entity_layers, old_visible_entity_layers) in clients_query.iter_mut() {
        let old_layers = old_visible_entity_layers.get();
        let current_layers = &visible_entity_layers.0;

        for &added_layer in current_layers.difference(old_layers) {
            if let Some((id, title, health, style, flags, _)) = boss_bars_query
                .iter()
                .find(|(_, _, _, _, _, layer_id)| layer_id.0 == added_layer)
            {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::Add {
                        title: title.0.to_owned(),
                        health: health.0,
                        color: style.color,
                        division: style.division,
                        flags: *flags,
                    },
                });
            };
        }

        for &removed_layer in old_layers.difference(current_layers) {
            if let Some((id, _, _, _, _, _)) = boss_bars_query
                .iter()
                .find(|(_, _, _, _, _, layer_id)| layer_id.0 == removed_layer)
            {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::Remove,
                });
            }
        }
    }
}

fn boss_bar_despawn(
    boss_bars_query: Query<(&UniqueId, &EntityLayerId), Added<Despawned>>,
    mut clients_query: Query<(&mut Client, &VisibleEntityLayers)>,
) {
    for (id, entity_layer_id) in boss_bars_query.iter() {
        for (mut client, visible_layer_id) in clients_query.iter_mut() {
            if visible_layer_id.0.contains(&entity_layer_id.0) {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::Remove,
                });
            }
        }
    }
}
