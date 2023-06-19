#![doc = include_str!("../README.md")]
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

pub mod fixed_tickstep;
#[allow(clippy::module_inception)]
mod tick;

use fixed_tickstep::{run_fixed_update_schedule, FixedTick};
pub use tick::*;

use bevy_ecs::system::ResMut;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Adds tick functionality to Apps.
#[derive(Default)]
pub struct TickPlugin;

#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
/// Updates the elapsed ticks. Any system that interacts with [Tick] component should run after
/// this.
pub struct TickSystem;

impl Plugin for TickSystem {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tick>()
            .register_type::<Tick>()
            .init_resource::<FixedTick>()
            .configure_set(TickSystem.in_base_set(CoreSet::First))
            .add_system(tick_system.in_set(TickSystem))
            .add_system(run_fixed_update_schedule.in_base_set(CoreSet::FixedUpdate));
    }
}

/// The system used to update the [`Tick`] used by app logic.
fn tick_system(mut tick: ResMut<Tick>) {
    tick.update();
}
