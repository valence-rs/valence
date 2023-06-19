use bevy_ecs::{reflect::ReflectResource, system::Resource};
use bevy_reflect::{FromReflect, Reflect};

/// A counter that tracks how many ticks has advanced
#[derive(Resource, Default, Reflect, FromReflect, Debug, Clone)]
#[reflect(Resource)]
pub struct Tick {
    elapsed: usize,
}

impl Tick {
    /// Constructs a new `Tick` instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the internal tick measurements.
    pub fn update(&mut self) {
        self.update_with_tick(1);
    }

    pub fn update_with_tick(&mut self, tick: usize) {
        self.elapsed += tick;
    }

    /// Returns how many tick have advanced since [`startup`](#method.startup), as [`usize`].
    #[inline]
    pub fn elapsed(&self) -> usize {
        self.elapsed
    }
}
