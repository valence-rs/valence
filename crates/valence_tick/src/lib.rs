// Adapted from https://github.com/bevyengine/bevy/blob/v0.10.1/crates/bevy_time/src/lib.rs

pub mod fixed_tickstep;
#[allow(clippy::module_inception)]
mod tick;

use fixed_tickstep::{run_fixed_update_schedule, FixedTick};
pub use tick::*;

use bevy_ecs::system::ResMut;
use flume::{Receiver, Sender};

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{fixed_tickstep::FixedTick, Tick}; // , Time, Timer, TimerMode
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Adds time functionality to Apps.
#[derive(Default)]
pub struct TickPlugin;

#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
/// Updates the elapsed time. Any system that interacts with [Time] component should run after
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

/// Channel resource used to receive time from render world
#[derive(Resource)]
pub struct TickReceiver(pub Receiver<usize>);

/// Channel resource used to send time from render world
#[derive(Resource)]
pub struct TickSender(pub Sender<usize>);

/// Creates channels used for sending time between render world and app world
pub fn create_time_channels() -> (TickSender, TickReceiver) {
    // bound the channel to 2 since when pipelined the render phase can finish before
    // the time system runs.
    let (s, r) = flume::bounded::<usize>(2);
    (TickSender(s), TickReceiver(r))
}

/// The system used to update the [`Time`] used by app logic. If there is a render world the time is sent from
/// there to this system through channels. Otherwise the time is updated in this system.
fn tick_system(mut tick: ResMut<Tick>) {
    tick.update();
}
