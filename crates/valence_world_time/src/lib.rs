//! # Controlling World Time
//! This module contains Components and Systems needed to update, tick,
//! broadcast information about the time of day and world age of a instance.
//!
//! ## Enable world time
//! To control world time of an instance, simply insert the [`WorldTime`]
//! component. We also need to broadcast world time updates to clients. The
//! [`IntervalTimeBroadcast::default()`] provides configuration to mimic
//! vanilla behavior:
//! ```
//! fn enable(mut commands: Commands, instance: Entity) {
//!     commands
//!         .entity(instance)
//!         .insert((WorldTime::deafault(), IntervalTimeBroadcast::default()));
//! }
//! ```
//!
//! Use [`ChangeTrackingTimeBroadcast`] if you need changes to time apply
//! immediately. It's also useful to use this over [`IntervalTimeBroadcast`] if
//! changes to time happens occasionally. You can also implement your custom
//! time broadcast if the above doesn't meet your requirements.
//!
//! ## Set the time explicitly
//! If you need set the time explicitly, simply modify fields of the
//! [`WorldTime`] components:
//! ```
//! fn into_the_night(mut instances: Query<&mut WorldTime, With<Instance>>) {
//!     for time in instances.iter_mut() {
//!         // Set the value directly
//!         time.time_of_day = time.time_of_day / 24000 * 24000 + 13000;
//!
//!         // Or, use utility method
//!         time.time_of_day
//!             .set_current_day_time(DayPhase::Night.into());
//!     }
//! }
//! ```
//!
//! ## Advacing the world time
//! Time of day and world age can be ticked individually using
//! [`LinearTimeTicking`] and [`LinearWorldAging`] respectively.
//! Note: If these component don't meet your requirements
//! (eg: you need time increment follow a sine wave ~~for some reason~~)
//! You can create your own ticking component by modify the respective
//! fields
//!
//! ## Prevent client from automatically update WorldTime
//! *(mimics `/gamerule doDaylightCycle false`)*
//!
//! By default, client will continue to update world time if the server
//! doesn't send packet to sync time between client and server.
//! Unfortunately, this can be disabled by setting `stop_client_time`
//! of [`WorldTime`] component to true.
//!
//! Here is an example of mimicing `/gamerule doDaylightCycle <value>`:
//! ```
//! #[derive(Component)]
//! pub struct DaylightCycle(pub bool);
//!
//! fn handle_game_rule_daylight_cycle(
//!     mut instances: Query<
//!         (&mut WorldTime, &mut LinearTimeTicking, &DaylightCycle),
//!         Changed<DaylightCycle>,
//!     >,
//! ) {
//!     for (mut time, mut ticking, doCycle) in instances.iter_mut() {
//!         // Stop client from update
//!         time.set_client_time_ticking(!doCycle.0);
//!         ticking.speed = if doCycle.0 { 1 } else { 0 };
//!     }
//! }
//! ```
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

pub mod packet;

use bevy_app::{CoreSet, Plugin};
use packet::WorldTimeUpdateS2c;
use valence_client::{Client, FlushPacketsSet};
use valence_core::protocol::encode::WritePacket;
use valence_core::Server;
use valence_entity::Location;
use valence_instance::{Instance, WriteUpdatePacketsToInstancesSet};
use valence_registry::*;

pub const DAY_LENGTH: i64 = 24000;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct CalculateWorldTimeSet;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct BroadcastWorldTimeSet;

pub struct WorldTimePlugin;

impl Plugin for WorldTimePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.configure_set(
            BroadcastWorldTimeSet
                .in_base_set(CoreSet::PostUpdate)
                .before(WriteUpdatePacketsToInstancesSet),
        )
        .configure_set(
            CalculateWorldTimeSet
                .in_base_set(CoreSet::PostUpdate)
                .before(BroadcastWorldTimeSet),
        )
        .add_systems(
            (
                handle_change_tracking_time_broadcast,
                handle_interval_time_broadcast,
            )
                .in_set(BroadcastWorldTimeSet),
        )
        .add_systems(
            (handle_linear_time_ticking, handle_linear_world_aging).in_set(CalculateWorldTimeSet),
        )
        .add_system(send_time_to_joined_players.before(FlushPacketsSet));
    }
}

/// Base component for storing world time information
#[derive(Component, Default)]
pub struct WorldTime {
    /// The amount of time in game tick the current world has passed
    pub world_age: i64,
    /// The time of day is based on the timestamp modulo 24000.
    /// 0 is sunrise, 6000 is noon, 12000 is sunset, and 18000 is midnight.
    pub time_of_day: i64,
}

impl WorldTime {
    /// This function ensure that adding time will not resulting in
    /// time_of_day flipping sign.
    /// Note: If the resulting calculation set time_of_day to 0, then
    /// the client will start advancing time.
    pub fn add_time(&mut self, amount: i64) {
        let client_ticking = self.client_time_ticking();
        self.time_of_day = self.time_of_day.abs().wrapping_add(amount);
        if self.time_of_day < 0 {
            self.time_of_day = self.time_of_day + i64::MAX + 1;
        }

        self.set_client_time_ticking(client_ticking);
    }

    /// If the client advance world time without server update
    pub fn client_time_ticking(&self) -> bool {
        !self.time_of_day.is_negative()
    }

    /// Set will the client advance world time without server update
    pub fn set_client_time_ticking(&mut self, val: bool) {
        self.time_of_day = if val {
            self.time_of_day.abs()
        } else {
            -self.time_of_day.abs()
        };
    }

    /// Get the time part of `time_of_day`
    pub fn current_day_time(&self) -> i64 {
        self.time_of_day % DAY_LENGTH
    }

    /// Set the time part of `time_of_day`
    pub fn set_current_day_time(&mut self, time: i64) {
        self.time_of_day = self.day() * DAY_LENGTH + time % DAY_LENGTH;
    }

    /// Get the current day part of `time_of_day`
    pub fn day(&self) -> i64 {
        self.time_of_day / DAY_LENGTH
    }

    /// Set the current day `time_of_day`
    pub fn set_day(&mut self, day: i64) {
        self.time_of_day = day * DAY_LENGTH + self.current_day_time();
    }

    /// Set the time_of_day to the next specified [`DayPhase`]
    pub fn warp_to_next_day_phase(&mut self, phase: DayPhase) {
        let phase_num: i64 = phase.into();
        if self.current_day_time() >= phase_num {
            self.set_day(self.day() + 1);
        }

        self.set_current_day_time(phase_num)
    }

    /// Set the time_of_day to the next specified [`MoonPhase`]
    pub fn wrap_to_next_moon_phase(&mut self, phase: MoonPhase) {
        let phase_no: i64 = phase.into();
        if self.day() % 8 >= phase_no {
            self.set_day(self.day() + 8 - (self.day() % 8))
        }

        self.set_day(self.day() + phase_no - self.day() % 8);
        self.set_current_day_time(DayPhase::Night.into());
    }
}

/// Notable events of a 24-hour Minecraft day
pub enum DayPhase {
    Day = 0,
    Noon = 6000,
    Sunset = 12000,
    Night = 13000,
    Midnight = 18000,
    Sunrise = 23000,
}

impl From<DayPhase> for i64 {
    fn from(value: DayPhase) -> Self {
        value as Self
    }
}

/// Reference: <https://minecraft.fandom.com/wiki/Daylight_cycle#Moon_phases>
pub enum MoonPhase {
    FullMoon = 0,
    WaningGibbous = 1,
    ThirdQuarter = 2,
    WaningCrescent = 3,
    NewMoon = 4,
    WaxingCrescent = 5,
    FirstQuarter = 6,
    WaxingGibbous = 7,
}

impl From<MoonPhase> for i64 {
    fn from(value: MoonPhase) -> Self {
        value as Self
    }
}

/// This component will advance the `time_of_day` field of [`WorldTime`] `speed`
/// per tick
#[derive(Component)]
pub struct LinearTimeTicking {
    pub speed: i64,
}

impl Default for LinearTimeTicking {
    fn default() -> Self {
        Self { speed: 1 }
    }
}

/// This component will advance the `world_age` field of [`WorldTime`] `speed`
/// per tick
#[derive(Component)]
pub struct LinearWorldAging {
    pub speed: i64,
}

impl Default for LinearWorldAging {
    fn default() -> Self {
        Self { speed: 1 }
    }
}

/// This component will broadcast world time information every `broadcast_rate`
/// ticks
#[derive(Component)]
pub struct IntervalTimeBroadcast {
    pub broadcast_rate: i64,
    last_broadcast: i64,
}

impl IntervalTimeBroadcast {
    pub fn new(broadcast_rate: i64) -> Self {
        Self {
            broadcast_rate,
            last_broadcast: 0,
        }
    }

    pub fn last_broadcast(&self) -> i64 {
        self.last_broadcast
    }
}

impl Default for IntervalTimeBroadcast {
    fn default() -> Self {
        Self::new(20)
    }
}

#[derive(Component)]
pub struct ChangeTrackingTimeBroadcast;

fn send_time_to_joined_players(
    mut clients: Query<(&mut Client, &Location), Changed<Location>>,
    instances: Query<&WorldTime, With<Instance>>,
) {
    for (mut client, loc) in clients.iter_mut() {
        let Ok(time) = instances.get(loc.0) else {
            continue;
        };
        client.write_packet(&WorldTimeUpdateS2c {
            time_of_day: time.time_of_day,
            world_age: time.world_age,
        });
    }
}

fn handle_change_tracking_time_broadcast(
    mut instances: Query<
        (&mut Instance, &WorldTime),
        (With<ChangeTrackingTimeBroadcast>, Changed<WorldTime>),
    >,
) {
    for (mut ins, time) in instances.iter_mut() {
        ins.write_packet(&WorldTimeUpdateS2c {
            time_of_day: time.time_of_day,
            world_age: time.world_age,
        })
    }
}

fn handle_interval_time_broadcast(
    mut instances: Query<(&mut Instance, &WorldTime, &mut IntervalTimeBroadcast)>,
    server: Res<Server>,
) {
    for (mut ins, time, mut interval) in instances.iter_mut() {
        let currect_tick = server.current_tick();
        if currect_tick - interval.last_broadcast >= interval.broadcast_rate {
            ins.write_packet(&WorldTimeUpdateS2c {
                time_of_day: time.time_of_day,
                world_age: time.world_age,
            });

            interval.last_broadcast = currect_tick;
        }
    }
}

fn handle_linear_time_ticking(
    mut instances: Query<(&mut WorldTime, &LinearTimeTicking), With<Instance>>,
) {
    for (mut time, ltr) in instances.iter_mut() {
        time.add_time(ltr.speed);
    }
}

fn handle_linear_world_aging(
    mut instances: Query<(&mut WorldTime, &LinearWorldAging), With<Instance>>,
) {
    for (mut time, lwa) in instances.iter_mut() {
        time.world_age += lwa.speed;
    }
}
