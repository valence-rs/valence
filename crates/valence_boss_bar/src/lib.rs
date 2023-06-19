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

use bevy_app::Plugin;
use bevy_ecs::query::{Added, Changed};
use bevy_ecs::removal_detection::RemovedComponents;
use bevy_ecs::system::Query;
use packet::{BossBarAction, BossBarS2c};
use valence_client::Client;
use valence_core::despawn::Despawned;
use valence_core::protocol::encode::WritePacket;
use valence_core::uuid::UniqueId;

mod components;
pub use components::*;

pub mod packet;

pub struct BossBarPlugin;

impl Plugin for BossBarPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_system(boss_bar_title_update)
            .add_system(boss_bar_health_update)
            .add_system(boss_bar_style_update)
            .add_system(boss_bar_flags_update)
            .add_system(boss_bar_viewers_update)
            .add_system(boss_bar_despawn)
            .add_system(client_disconnection);
    }
}

/// System that sends a bossbar update title packet to all viewers of a boss bar
/// that has had its title updated.
fn boss_bar_title_update(
    boss_bars: Query<(&UniqueId, &BossBarTitle, &BossBarViewers), Changed<BossBarTitle>>,
    mut clients: Query<&mut Client>,
) {
    for (id, title, boss_bar_viewers) in boss_bars.iter() {
        for viewer in boss_bar_viewers.viewers.iter() {
            if let Ok(mut client) = clients.get_mut(*viewer) {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::UpdateTitle(Cow::Borrowed(&title.0)),
                });
            }
        }
    }
}

/// System that sends a bossbar update health packet to all viewers of a boss
/// bar that has had its health updated.
fn boss_bar_health_update(
    boss_bars: Query<(&UniqueId, &BossBarHealth, &BossBarViewers), Changed<BossBarHealth>>,
    mut clients: Query<&mut Client>,
) {
    for (id, health, boss_bar_viewers) in boss_bars.iter() {
        for viewer in boss_bar_viewers.viewers.iter() {
            if let Ok(mut client) = clients.get_mut(*viewer) {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::UpdateHealth(health.0),
                });
            }
        }
    }
}

/// System that sends a bossbar update style packet to all viewers of a boss bar
/// that has had its style updated.
fn boss_bar_style_update(
    boss_bars: Query<(&UniqueId, &BossBarStyle, &BossBarViewers), Changed<BossBarStyle>>,
    mut clients: Query<&mut Client>,
) {
    for (id, style, boss_bar_viewers) in boss_bars.iter() {
        for viewer in boss_bar_viewers.viewers.iter() {
            if let Ok(mut client) = clients.get_mut(*viewer) {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::UpdateStyle(style.color, style.division),
                });
            }
        }
    }
}

/// System that sends a bossbar update flags packet to all viewers of a boss bar
/// that has had its flags updated.
fn boss_bar_flags_update(
    boss_bars: Query<(&UniqueId, &BossBarFlags, &BossBarViewers), Changed<BossBarFlags>>,
    mut clients: Query<&mut Client>,
) {
    for (id, flags, boss_bar_viewers) in boss_bars.iter() {
        for viewer in boss_bar_viewers.viewers.iter() {
            if let Ok(mut client) = clients.get_mut(*viewer) {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::UpdateFlags(*flags),
                });
            }
        }
    }
}

/// System that sends a bossbar add/remove packet to all viewers of a boss bar
/// that just have been added/removed.
fn boss_bar_viewers_update(
    mut boss_bars: Query<
        (
            &UniqueId,
            &BossBarTitle,
            &BossBarHealth,
            &BossBarStyle,
            &BossBarFlags,
            &mut BossBarViewers,
        ),
        Changed<BossBarViewers>,
    >,
    mut clients: Query<&mut Client>,
) {
    for (id, title, health, style, flags, mut boss_bar_viewers) in boss_bars.iter_mut() {
        let old_viewers = &boss_bar_viewers.old_viewers;
        let current_viewers = &boss_bar_viewers.viewers;

        for &added_viewer in current_viewers.difference(old_viewers) {
            if let Ok(mut client) = clients.get_mut(added_viewer) {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::Add {
                        title: Cow::Borrowed(&title.0),
                        health: health.0,
                        color: style.color,
                        division: style.division,
                        flags: *flags,
                    },
                });
            }
        }

        for &removed_viewer in old_viewers.difference(current_viewers) {
            if let Ok(mut client) = clients.get_mut(removed_viewer) {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::Remove,
                });
            }
        }

        boss_bar_viewers.old_viewers = boss_bar_viewers.viewers.clone();
    }
}

/// System that sends a bossbar remove packet to all viewers of a boss bar that
/// has been despawned.
fn boss_bar_despawn(
    mut boss_bars: Query<(&UniqueId, &BossBarViewers), Added<Despawned>>,
    mut clients: Query<&mut Client>,
) {
    for (id, viewers) in boss_bars.iter_mut() {
        for viewer in viewers.viewers.iter() {
            if let Ok(mut client) = clients.get_mut(*viewer) {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::Remove,
                });
            }
        }
    }
}

/// System that removes a client from the viewers of its boss bars when it
/// disconnects.
fn client_disconnection(
    mut disconnected_clients: RemovedComponents<Client>,
    mut boss_bars_viewers: Query<&mut BossBarViewers>,
) {
    for entity in disconnected_clients.iter() {
        for mut boss_bar_viewers in boss_bars_viewers.iter_mut() {
            boss_bar_viewers.viewers.retain(|viewer| *viewer != entity);
        }
    }
}
