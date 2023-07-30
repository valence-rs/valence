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
use valence_core::boss_bar::{BossBarColor, BossBarDivision, BossBarFlags};
use valence_core::despawn::Despawned;
use valence_core::protocol::encode::WritePacket;
use valence_core::text::Text;
use valence_core::uuid::UniqueId;
use valence_packet::boss_bar::{BossBarAction, BossBarS2c};

#[derive(Clone, Debug)]
enum BossBarUpdate {
    UpdateHealth(f32),
    UpdateTitle(Text),
    UpdateStyle(BossBarColor, BossBarDivision),
    UpdateFlags(BossBarFlags),
}

#[derive(Clone, Debug, Component)]
pub struct BossBar {
    pub title: Text,
    /// From 0 to 1. Values greater than 1 do not crash a Notchian client, and
    /// start rendering part of a second health bar at around 1.5
    pub health: f32,
    pub color: BossBarColor,
    pub division: BossBarDivision,
    pub flags: BossBarFlags,
    update: Vec<BossBarUpdate>,
}

impl BossBar {
    fn new(
        title: Text,
        health: f32,
        color: BossBarColor,
        division: BossBarDivision,
        flags: BossBarFlags,
    ) -> Self {
        Self {
            title,
            health,
            color,
            division,
            flags,
            update: Vec::new(),
        }
    }

    pub fn update_health(&mut self, health: f32) {
        self.health = health;
        self.update.push(BossBarUpdate::UpdateHealth(health));
    }

    pub fn update_title(&mut self, title: Text) {
        let cloned_title = title.clone();
        self.title = title;
        self.update.push(BossBarUpdate::UpdateTitle(cloned_title));
    }

    pub fn update_style(&mut self, color: Option<BossBarColor>, division: Option<BossBarDivision>) {
        if let Some(color) = color {
            self.color = color;
        }
        if let Some(division) = division {
            self.division = division;
        }
        self.update
            .push(BossBarUpdate::UpdateStyle(self.color, self.division));
    }

    pub fn update_flags(&mut self, flags: BossBarFlags) {
        self.flags = flags;
        self.update.push(BossBarUpdate::UpdateFlags(flags));
    }
}

/// The viewers of a boss bar.
#[derive(Component, Default, Debug, Clone)]
pub struct BossBarViewers {
    /// The current viewers of the boss bar. It is the list that should be
    /// updated.
    pub viewers: BTreeSet<Entity>,
    /// The viewers of the last tick in order to determine which viewers have
    /// been added and removed.
    pub(crate) old_viewers: BTreeSet<Entity>,
}

#[derive(Bundle, Debug)]
pub struct BossBarBundle {
    boss_bar: BossBar,
    unique_id: UniqueId,
    viewers: BossBarViewers,
}

impl BossBarBundle {
    pub fn new(
        title: Text,
        health: f32,
        color: BossBarColor,
        division: BossBarDivision,
        flags: BossBarFlags,
    ) -> Self {
        Self {
            boss_bar: BossBar::new(title, health, color, division, flags),
            unique_id: UniqueId::default(),
            viewers: BossBarViewers::default(),
        }
    }
}

// #[derive(Debug, Component, Default)]
// struct VisibleBossBar(pub BTreeSet<Entity>);

// #[derive(Debug, Component, Default)]
// struct OldVisibleBossBar(BTreeSet<Entity>);

// #[derive(Debug, Bundle, Default)]
// pub struct VisibleBossBarBundle {
//     visible_boss_bar: VisibleBossBar,
//     old_visible_boss_bar: OldVisibleBossBar,
// }

// impl VisibleBossBarBundle {
//     pub fn new(boss_bar_ids: Vec<Entity>) -> Self {
//         Self {
//             visible_boss_bar:
// VisibleBossBar(boss_bar_ids.into_iter().collect()),
// ..Default::default()         }
//     }
// }

pub struct BossBarPlugin;

impl Plugin for BossBarPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            (
                update_boss_bar,
                boss_bar_viewers_update,
                boss_bar_despawn,
                client_disconnection.before(boss_bar_viewers_update),
            )
                .before(FlushPacketsSet),
        );
    }
}

fn update_boss_bar(
    mut boss_bars: Query<(&UniqueId, &mut BossBar, &BossBarViewers), Changed<BossBar>>,
    mut clients: Query<&mut Client>,
) {
    for (id, mut boss_bar, boss_bar_viewers) in boss_bars.iter_mut() {
        for viewer in boss_bar_viewers.viewers.iter() {
            if let Ok(mut client) = clients.get_mut(*viewer) {
                for update in boss_bar.update.clone().into_iter() {
                    match update {
                        BossBarUpdate::UpdateHealth(health) => {
                            client.write_packet(&BossBarS2c {
                                id: id.0,
                                action: BossBarAction::UpdateHealth(health),
                            });
                        }
                        BossBarUpdate::UpdateTitle(title) => {
                            client.write_packet(&BossBarS2c {
                                id: id.0,
                                action: BossBarAction::UpdateTitle(Cow::Borrowed(&title)),
                            });
                        }
                        BossBarUpdate::UpdateStyle(color, division) => {
                            client.write_packet(&BossBarS2c {
                                id: id.0,
                                action: BossBarAction::UpdateStyle(color, division),
                            });
                        }
                        BossBarUpdate::UpdateFlags(flags) => {
                            client.write_packet(&BossBarS2c {
                                id: id.0,
                                action: BossBarAction::UpdateFlags(flags),
                            });
                        }
                    }
                }
            }
        }
        boss_bar.update.clear();
    }
}

/// System that sends a bossbar add/remove packet to all viewers of a boss bar
/// that just have been added/removed.
fn boss_bar_viewers_update(
    mut boss_bars: Query<(&UniqueId, &BossBar, &mut BossBarViewers), Changed<BossBarViewers>>,
    mut clients: Query<&mut Client>,
) {
    for (id, boss_bar, mut boss_bar_viewers) in boss_bars.iter_mut() {
        let old_viewers = &boss_bar_viewers.old_viewers;
        let current_viewers = &boss_bar_viewers.viewers;

        for &added_viewer in current_viewers.difference(old_viewers) {
            if let Ok(mut client) = clients.get_mut(added_viewer) {
                client.write_packet(&BossBarS2c {
                    id: id.0,
                    action: BossBarAction::Add {
                        title: Cow::Borrowed(&boss_bar.title),
                        health: boss_bar.health,
                        color: boss_bar.color,
                        division: boss_bar.division,
                        flags: boss_bar.flags,
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
