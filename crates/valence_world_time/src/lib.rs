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

pub mod extra;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;
use bevy_ecs::query::Changed;
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::system::{Query, Res};
use derive_more::{Deref, DerefMut};
use valence_server::client::{Client, FlushPacketsSet, VisibleChunkLayer};
use valence_server::protocol::packets::play::WorldTimeUpdateS2c;
use valence_server::protocol::WritePacket;
use valence_server::{ChunkLayer, Server};

pub struct WorldTimePlugin;

impl Plugin for WorldTimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                handle_interval_broadcast,
                handle_linear_time_ticking,
                handle_linear_world_aging,
            )
                .before(FlushPacketsSet)
                .before(handle_layer_time_broadcastcast),
        )
        .add_systems(
            PostUpdate,
            handle_layer_time_broadcastcast.before(init_time_for_new_clients),
        )
        .add_systems(PostUpdate, init_time_for_new_clients);
    }
}

#[derive(Bundle, Default, Debug)]
pub struct WorldTimeBundle {
    pub world_time: WorldTime,
    pub broadcast: WorldTimeBroadcast,
    pub interval: IntervalBroadcast,
    pub linear_ticker: LinearTimeTicking,
    pub linear_ticker_timestamp: LinearTimeTickerTimestamp,
    pub linear_world_age: LinearWorldAging,
    pub linear_world_age_timestamp: LinearWorldAgingTimestamp,
}

/// The base component to store time in a layer.
/// Tip: If you are looking to modify time in a layer, use
/// [`SetTimeQuery`] to also broadcast time immediately
#[derive(Component, Default, PartialEq, Clone, Copy, Debug)]
pub struct WorldTime {
    /// The age of the world in 1/20ths of a second.
    pub world_age: i64,
    /// The current time of day in 1/20ths of a second.
    /// The value should be in the range \[0, 24000].
    /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
    pub time_of_day: i64,
}

/// Store information about the last broadcasted time. You shouldn't
/// mutate this component directly.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct WorldTimeBroadcast {
    pub last_broadcasted: WorldTime,
    pub timestamp: i64,
    pub will_broadcast_this_tick: bool,
}

/// This component will signal [`WorldTimeBroadcast`] to send
/// [`WorldTimeUpdateS2c`] packet on an interval. Note that
/// it compares the last broadcasted timestamp with the
/// current server tick to determine if an update should be sent.
#[derive(Component, Deref, DerefMut, Clone, Copy, Debug)]
pub struct IntervalBroadcast(pub i64);

impl Default for IntervalBroadcast {
    fn default() -> Self {
        Self(20)
    }
}

/// Use this struct to set time and broadcast it immediately at
/// this tick
#[derive(Debug)]
pub struct SetTimeQuery {
    time: &'static mut WorldTime,
    broadcast: &'static mut WorldTimeBroadcast,
}

impl Deref for SetTimeQuery {
    type Target = WorldTime;

    fn deref(&self) -> &Self::Target {
        self.time
    }
}

impl DerefMut for SetTimeQuery {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.broadcast.will_broadcast_this_tick = true;
        self.time
    }
}

/// This component is responsible for managing time in a
/// linear fashion. It is commonly used to handle day-night cycles and
/// similar time-dependent processes. This component employs both an interval
/// and a rate to control time progression.
#[derive(Component, Clone, Copy, Debug)]
pub struct LinearTimeTicking {
    /// The time interval (in server tick) between each time tick.
    pub interval: i64,

    /// The rate at which time advances. A rate of 1 corresponds to real-time,
    /// while values less than 1 make time progress slower than the server tick
    /// rate.
    pub rate: i64,
}

impl Default for LinearTimeTicking {
    fn default() -> Self {
        Self {
            interval: 1,
            rate: 1,
        }
    }
}

#[derive(Component, Default, Deref, DerefMut, Clone, Copy, Debug)]
pub struct LinearTimeTickerTimestamp(pub i64);

/// Similar to [`LinearTimeTicking`] but for world age
#[derive(Component, Clone, Copy, Debug)]
pub struct LinearWorldAging {
    /// The time interval (in server tick) between each time tick.
    pub interval: i64,

    /// The rate at which world age advances. A rate of 1 corresponds to
    /// real-time, while values less than 1 make time progress slower than
    /// the server tick rate.
    pub rate: i64,
}

impl Default for LinearWorldAging {
    fn default() -> Self {
        Self {
            interval: 1,
            rate: 1,
        }
    }
}

#[derive(Component, Default, Deref, DerefMut, Clone, Copy, Debug)]
pub struct LinearWorldAgingTimestamp(pub i64);

fn init_time_for_new_clients(
    mut clients: Query<(&mut Client, &VisibleChunkLayer), Changed<VisibleChunkLayer>>,
    layers: Query<&WorldTime>,
) {
    for (mut client, layer_ref) in &mut clients {
        if let Ok(time) = layers.get(layer_ref.0) {
            client.write_packet(&WorldTimeUpdateS2c {
                time_of_day: time.time_of_day,
                world_age: time.world_age,
            })
        }
    }
}

fn handle_layer_time_broadcastcast(
    mut layers: Query<(&mut ChunkLayer, &WorldTime, &mut WorldTimeBroadcast)>,
    server: Res<Server>,
) {
    for (mut layer, time, mut broadcast) in &mut layers {
        if broadcast.will_broadcast_this_tick {
            layer.write_packet(&WorldTimeUpdateS2c {
                time_of_day: time.time_of_day,
                world_age: time.world_age,
            });

            broadcast.will_broadcast_this_tick = false;
            broadcast.timestamp = server.current_tick();
        }
    }
}

fn handle_interval_broadcast(
    mut time: Query<(&IntervalBroadcast, &mut WorldTimeBroadcast)>,
    server: Res<Server>,
) {
    for (interval, mut broadcast) in &mut time {
        if server.current_tick() - broadcast.timestamp >= interval.0 {
            broadcast.will_broadcast_this_tick = true;
        }
    }
}

fn handle_linear_time_ticking(
    mut ticker: Query<(
        &LinearTimeTicking,
        &mut LinearTimeTickerTimestamp,
        &mut WorldTime,
    )>,
    server: Res<Server>,
) {
    for (info, mut ts, mut time) in &mut ticker {
        let ct = server.current_tick();
        if ct - ts.0 >= info.interval {
            time.time_of_day += info.rate;
            ts.0 = ct;
        }
    }
}

fn handle_linear_world_aging(
    mut ticker: Query<(
        &LinearWorldAging,
        &mut LinearWorldAgingTimestamp,
        &mut WorldTime,
    )>,
    server: Res<Server>,
) {
    for (info, mut ts, mut time) in &mut ticker {
        let ct = server.current_tick();
        if ct - ts.0 >= info.interval {
            time.world_age += info.rate;
            ts.0 = ct;
        }
    }
}
