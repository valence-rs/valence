use bevy_ecs::{reflect::ReflectResource, system::Resource};
use bevy_reflect::{FromReflect, Reflect};

/// A clock that tracks how much it has advanced (and how much real tick has elapsed) since
/// its previous update and since its creation.
#[derive(Resource, Reflect, FromReflect, Debug, Clone)]
#[reflect(Resource)]
pub struct Tick {
    elapsed: usize,
}

impl Default for Tick {
    fn default() -> Self {
        Self { elapsed: 0 }
    }
}

impl Tick {
    /// Constructs a new `Tick` instance with a specific startup `Tick`.
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Updates the internal tick measurements.
    ///
    /// Calling this method as part of your app will most likely result in inaccurate tickkeeping,
    /// as the `Tick` resource is ordinarily managed by the [`TickPlugin`](crate::TickPlugin).
    pub fn update(&mut self) {
        self.update_with_tick(1);
    }

    pub fn update_with_tick(&mut self, tick: usize) {
        self.elapsed += tick;
    }

    /// Returns how much tick has advanced since [`startup`](#method.startup), as [`Duration`].
    #[inline]
    pub fn elapsed(&self) -> usize {
        self.elapsed
    }
}
