pub mod fixed_tickstep;
#[allow(clippy::module_inception)]
mod tick;

use fixed_tickstep::{run_fixed_update_schedule, FixedTick};
pub use tick::*;

use bevy_ecs::system::ResMut;

pub mod prelude {
    //! The Valence Tick Prelude.
    #[doc(hidden)]
    pub use crate::{fixed_tickstep::FixedTick, Tick, TickSystem};
}

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
