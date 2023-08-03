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
use std::collections::BTreeSet;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_client::{Client, FlushPacketsSet};
use valence_core::despawn::Despawned;
use valence_core::text::Text;
use valence_core::uuid::UniqueId;
pub use valence_packet::packets::play::boss_bar_s2c::{
    BossBarAction, BossBarColor, BossBarDivision, BossBarFlags,
};
use valence_packet::packets::play::BossBarS2c;
use valence_packet::protocol::encode::WritePacket;

/// The bundle of components that make up a boss bar.
#[derive(Bundle, Debug, Default)]
pub struct BossBarBundle {
    pub id: UniqueId,
    pub title: BossBarTitle,
    pub health: BossBarHealth,
    pub style: BossBarStyle,
    pub flags: BossBarFlags,
    pub viewers: BossBarViewers,
}

impl BossBarBundle {
    pub fn new(
        title: Text,
        color: BossBarColor,
        division: BossBarDivision,
        flags: BossBarFlags,
    ) -> BossBarBundle {
        BossBarBundle {
            id: UniqueId::default(),
            title: BossBarTitle(title),
            health: BossBarHealth(1.0),
            style: BossBarStyle { color, division },
            flags,
            viewers: BossBarViewers::default(),
        }
    }
}

/// The title of a boss bar.
#[derive(Component, Clone, Debug, Default)]
pub struct BossBarTitle(pub Text);

/// The health of a boss bar.
#[derive(Component, Debug, Default)]
pub struct BossBarHealth(pub f32);

/// The style of a boss bar. This includes the color and division of the boss
/// bar.
#[derive(Component, Debug, Default)]
pub struct BossBarStyle {
    pub color: BossBarColor,
    pub division: BossBarDivision,
}

/// The viewers of a boss bar.
#[derive(Component, Default, Debug)]
pub struct BossBarViewers {
    /// The current viewers of the boss bar. It is the list that should be
    /// updated.
    pub viewers: BTreeSet<Entity>,
    /// The viewers of the last tick in order to determine which viewers have
    /// been added and removed.
    pub(crate) old_viewers: BTreeSet<Entity>,
}

pub struct BossBarPlugin;

impl Plugin for BossBarPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            (
                boss_bar_title_update,
                boss_bar_health_update,
                boss_bar_style_update,
                boss_bar_flags_update,
                boss_bar_viewers_update,
                boss_bar_despawn,
                client_disconnection.before(boss_bar_viewers_update),
            )
                .before(FlushPacketsSet),
        );
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
    disconnected_clients: Query<Entity, (With<Client>, Added<Despawned>)>,
    mut boss_bars_viewers: Query<&mut BossBarViewers>,
) {
    for entity in disconnected_clients.iter() {
        for mut boss_bar_viewers in boss_bars_viewers.iter_mut() {
            boss_bar_viewers.viewers.remove(&entity);
        }
    }
}
