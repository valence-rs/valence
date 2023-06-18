//! Tools to run systems at a regular interval.
//! This can be extremely useful for steady, frame-rate independent gameplay logic and physics.
//!
//! To run a system on a fixed tickstep, add it to the [`CoreSchedule::FixedUpdate`] [`Schedule`](bevy_ecs::schedule::Schedule).
//! This schedules is run in the [`CoreSet::FixedUpdate`](bevy_app::CoreSet::FixedUpdate) near the start of each frame,
//! via the [`run_fixed_update_schedule`] exclusive system.
//!
//! This schedule will be run a number of ticks each frame,
//! equal to the accumulated divided by the period resource, rounded down,
//! as tracked in the [`FixedTick`] resource.
//! Unused tick will be carried over.
//!
//! This does not guarantee that the tick elapsed between executions is exact,
//! and systems in this schedule can run 0, 1 or more ticks on any given frame.
//!
//! For example, a system with a fixed tickstep run criteria of 120 ticks per second will run
//! two ticks during a ~16.667ms frame, once during a ~8.333ms frame, and once every two frames
//! with ~4.167ms frames. However, the same criteria may not result in exactly 8.333ms passing
//! between each execution.
//!
//! When using fixed tick steps, it is advised not to rely on [`Tick::delta`] or any of it's
//! variants for game simulation, but rather use the value of [`FixedTick`] instead.

use bevy_app::CoreSchedule;
use bevy_ecs::{system::Resource, world::World};
use thiserror::Error;

/// The amount of tick that must pass before the fixed tickstep schedule is run again.
#[derive(Resource, Debug)]
pub struct FixedTick {
    accumulated: usize,
    period: usize,
}

impl FixedTick {
    /// Creates a new [`FixedTick`] struct
    pub fn new(period: usize) -> Self {
        FixedTick {
            accumulated: 0,
            period,
        }
    }

    /// Adds 1 to the accumulated tick so far.
    pub fn tick(&mut self) {
        self.accumulated += 1;
    }

    /// Returns the current amount of accumulated tick
    pub fn accumulated(&self) -> usize {
        self.accumulated
    }

    /// Expends one `period` of accumulated tick.
    ///
    /// [`Err(FixedUpdateError`)] will be returned
    pub fn expend(&mut self) -> Result<(), FixedUpdateError> {
        if let Some(new_value) = self.accumulated.checked_sub(self.period) {
            self.accumulated = new_value;
            Ok(())
        } else {
            Err(FixedUpdateError::NotEnoughTick {
                accumulated: self.accumulated,
                period: self.period,
            })
        }
    }
}

impl Default for FixedTick {
    fn default() -> Self {
        FixedTick {
            accumulated: 0,
            period: 1,
        }
    }
}

/// An error returned when working with [`FixedTick`].
#[derive(Debug, Error)]
pub enum FixedUpdateError {
    #[error("At least one period worth of ticks must be accumulated.")]
    NotEnoughTick { accumulated: usize, period: usize },
}

/// Ticks the [`FixedTick`] resource then runs the [`CoreSchedule::FixedUpdate`].
pub fn run_fixed_update_schedule(world: &mut World) {
    // Tick the time
    let mut fixed_time = world.resource_mut::<FixedTick>();
    fixed_time.tick();

    // Run the schedule until we run out of accumulated time
    let mut check_again = true;
    while check_again {
        let mut fixed_time = world.resource_mut::<FixedTick>();
        let fixed_time_run = fixed_time.expend().is_ok();
        if fixed_time_run {
            world.run_schedule(CoreSchedule::FixedUpdate);
        } else {
            check_again = false;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fixed_time_starts_at_zero() {
        let new_time = FixedTick::new(42);
        assert_eq!(new_time.accumulated(), 0);

        let default_time = FixedTick::default();
        assert_eq!(default_time.accumulated(), 0);
    }

    #[test]
    fn fixed_time_ticks_up() {
        let mut fixed_time = FixedTick::default();
        fixed_time.tick();
        assert_eq!(fixed_time.accumulated(), 1);
    }

    #[test]
    fn enough_accumulated_time_is_required() {
        let mut fixed_time = FixedTick::new(2);
        fixed_time.tick();
        assert!(fixed_time.expend().is_err());
        assert_eq!(fixed_time.accumulated(), 1);

        fixed_time.tick();
        assert!(fixed_time.expend().is_ok());
        assert_eq!(fixed_time.accumulated(), 0);
    }

    #[test]
    fn repeatedly_expending_time() {
        let mut fixed_time = FixedTick::new(1);
        fixed_time.tick();
        assert!(fixed_time.expend().is_ok());
        assert!(fixed_time.expend().is_err());
    }
}
