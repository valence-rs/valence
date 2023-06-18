#![allow(clippy::type_complexity)]

use bevy_app::Plugin;
use bevy_ecs::query::{Added, Changed};
use bevy_ecs::removal_detection::RemovedComponents;
use bevy_ecs::system::Query;
use components::{BossBarFlags, BossBarHealth, BossBarStyle, BossBarTitle, BossBarViewers};
use packet::{BossBarAction, BossBarS2c};
use valence_client::Client;
use valence_core::despawn::Despawned;
use valence_core::protocol::encode::WritePacket;
use valence_core::uuid::UniqueId;

pub mod components;
pub mod packet;

pub struct BossBarPlugin;

impl Plugin for BossBarPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_system(handle_boss_bar_title_update)
            .add_system(handle_boss_bar_health_update)
            .add_system(handle_boss_bar_style_update)
            .add_system(handle_boss_bar_flags_update)
            .add_system(handle_boss_bar_viewers_update)
            .add_system(handle_boss_bar_despawn)
            .add_system(handle_client_disconnection);
    }
}

/// System that sends a bossbar update title packet to all viewers of a boss bar
/// that has had its title updated.
fn handle_boss_bar_title_update(
    mut boss_bars: Query<(&UniqueId, &BossBarTitle, &mut BossBarViewers), Changed<BossBarTitle>>,
    mut clients: Query<&mut Client>,
) {
    for (id, title, mut boss_bar_viewers) in boss_bars.iter_mut() {
        for viewer in boss_bar_viewers.viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::UpdateTitle(title.0.clone()),
            });
        }
    }
}

/// System that sends a bossbar update health packet to all viewers of a boss
/// bar that has had its health updated.
fn handle_boss_bar_health_update(
    mut boss_bars: Query<(&UniqueId, &BossBarHealth, &mut BossBarViewers), Changed<BossBarHealth>>,
    mut clients: Query<&mut Client>,
) {
    for (id, health, mut boss_bar_viewers) in boss_bars.iter_mut() {
        for viewer in boss_bar_viewers.viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::UpdateHealth(health.0),
            });
        }
    }
}

/// System that sends a bossbar update style packet to all viewers of a boss bar
/// that has had its style updated.
fn handle_boss_bar_style_update(
    mut boss_bars: Query<(&UniqueId, &BossBarStyle, &mut BossBarViewers), Changed<BossBarStyle>>,
    mut clients: Query<&mut Client>,
) {
    for (id, style, mut boss_bar_viewers) in boss_bars.iter_mut() {
        for viewer in boss_bar_viewers.viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::UpdateStyle(style.color, style.division),
            });
        }
    }
}

/// System that sends a bossbar update flags packet to all viewers of a boss bar
/// that has had its flags updated.
fn handle_boss_bar_flags_update(
    mut boss_bars: Query<(&UniqueId, &BossBarFlags, &mut BossBarViewers), Changed<BossBarFlags>>,
    mut clients: Query<&mut Client>,
) {
    for (id, flags, mut boss_bar_viewers) in boss_bars.iter_mut() {
        for viewer in boss_bar_viewers.viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::UpdateFlags(*flags),
            });
        }
    }
}

/// System that sends a bossbar add/remove packet to all viewers of a boss bar
/// that just have been added/removed.
fn handle_boss_bar_viewers_update(
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

        let mut added_viewers = Vec::new();
        let mut removed_viewers = Vec::new();

        for viewer in current_viewers.iter() {
            if !old_viewers.contains(viewer) {
                added_viewers.push(*viewer);
            }
        }

        for viewer in old_viewers.iter() {
            if !current_viewers.contains(viewer) {
                removed_viewers.push(*viewer);
            }
        }

        for added_viewer in added_viewers {
            let mut client = clients.get_mut(added_viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::Add {
                    title: title.0.clone(),
                    health: health.0,
                    color: style.color,
                    division: style.division,
                    flags: *flags,
                },
            });
        }

        for removed_viewer in removed_viewers {
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
fn handle_boss_bar_despawn(
    mut boss_bars: Query<(&UniqueId, &mut BossBarViewers), Added<Despawned>>,
    mut clients: Query<&mut Client>,
) {
    for boss_bar in boss_bars.iter_mut() {
        let (id, mut viewers) = boss_bar;

        for viewer in viewers.viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::Remove,
            });
        }
    }
}

/// System that removes a client from the viewers of its boss bars when it
/// disconnects.
fn handle_client_disconnection(
    mut disconnected_clients: RemovedComponents<Client>,
    mut boss_bars_viewers: Query<&mut BossBarViewers>,
) {
    for entity in disconnected_clients.iter() {
        for mut boss_bar_viewers in boss_bars_viewers.iter_mut() {
            boss_bar_viewers.viewers.retain(|viewer| *viewer != entity);
        }
    }
}
