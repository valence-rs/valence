use bevy_app::Plugin;
use bevy_ecs::{system::Query, query::{Added, Changed}};
use components::{BossBarViewers, BossBarTitle, BossBarHealth, BossBarStyle, BossBarFlags};
use packet::{BossBarS2c, BossBarAction};
use valence_client::Client;
use valence_core::{despawn::Despawned, protocol::encode::WritePacket, uuid::UniqueId};

pub mod packet;
pub mod components;

pub struct BossBarPlugin;

impl Plugin for BossBarPlugin {

    fn build(&self, app: &mut bevy_app::App) {
        app
        .add_system(remove_despawned_boss_bars_from_viewers)
        .add_system(handle_boss_bar_title_update)
        .add_system(handle_boss_bar_health_update)
        .add_system(handle_boss_bar_style_update)
        .add_system(handle_boss_bar_flags_update)
        .add_system(handle_boss_bar_viewers_update);
    }

}

/// System that sends a bossbar update title packet to all viewers of a boss bar that has had its title updated.
fn handle_boss_bar_title_update(mut boss_bars: Query<(&UniqueId, &BossBarTitle, &mut BossBarViewers), Changed<BossBarTitle>>, mut clients: Query<&mut Client>) {
    for (id, title, mut viewers) in boss_bars.iter_mut() {
        for viewer in viewers.current_viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::UpdateTitle(title.0.clone()),
            });
        }
    }
}

/// System that sends a bossbar update health packet to all viewers of a boss bar that has had its health updated.
fn handle_boss_bar_health_update(mut boss_bars: Query<(&UniqueId, &BossBarHealth, &mut BossBarViewers), Changed<BossBarHealth>>, mut clients: Query<&mut Client>) {
    for (id, health, mut viewers) in boss_bars.iter_mut() {
        for viewer in viewers.current_viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::UpdateHealth(health.0),
            });
        }
    }
}

/// System that sends a bossbar update style packet to all viewers of a boss bar that has had its style updated.
fn handle_boss_bar_style_update(mut boss_bars: Query<(&UniqueId, &BossBarStyle, &mut BossBarViewers), Changed<BossBarStyle>>, mut clients: Query<&mut Client>) {
    for (id, style, mut viewers) in boss_bars.iter_mut() {
        for viewer in viewers.current_viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::UpdateStyle(style.color, style.division)
            });
        }
    }
}

/// System that sends a bossbar update flags packet to all viewers of a boss bar that has had its flags updated.
fn handle_boss_bar_flags_update(mut boss_bars: Query<(&UniqueId, &BossBarFlags, &mut BossBarViewers), Changed<BossBarFlags>>, mut clients: Query<&mut Client>) {
    for (id, flags, mut viewers) in boss_bars.iter_mut() {
        for viewer in viewers.current_viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::UpdateFlags(flags.clone()),
            });
        }
    }
}

/// System that sends a bossbar add/remove packet to all viewers of a boss bar that just have been added/removed.
fn handle_boss_bar_viewers_update(mut boss_bars: Query<(&UniqueId, &BossBarTitle, &BossBarHealth, &BossBarStyle, &BossBarFlags, &mut BossBarViewers), Changed<BossBarViewers>>, mut clients: Query<&mut Client>) {
    for (id, title, health, style, flags, mut viewers) in boss_bars.iter_mut() {
        let previous_viewers = &viewers.last_viewers;
        let current_viewers = &viewers.current_viewers;

        let mut added_viewers = Vec::new();
        let mut removed_viewers = Vec::new();

        for viewer in viewers.current_viewers.iter() {
            if !previous_viewers.contains(viewer) {
                added_viewers.push(*viewer);
            }
        }

        for viewer in previous_viewers.iter() {
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
                    flags: flags.clone(),
                },
            });
        }

        for removed_viewer in removed_viewers {
            let mut client = clients.get_mut(removed_viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::Remove,
            });
        }

        viewers.last_viewers = viewers.current_viewers.clone();
    }
}

/// System that sends a bossbar remove packet to all viewers of a boss bar that has been despawned.
fn remove_despawned_boss_bars_from_viewers(mut boss_bars: Query<(&UniqueId, &mut BossBarViewers), Added<Despawned>>, mut clients: Query<&mut Client>) {
    for boss_bar in boss_bars.iter_mut() {
        let (id, mut viewers) = boss_bar;

        for viewer in viewers.current_viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::Remove,
            });
        }
    }
}