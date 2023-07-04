use std::sync::atomic::{AtomicU32, Ordering};

use bevy_ecs::prelude::*;
use tracing::warn;

#[derive(Component, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Layer(u32);

impl Layer {
    pub const DEFAULT: Self = Self(0);

    pub fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(1); // Skip default layer.

        let val = NEXT.fetch_add(1, Ordering::Relaxed);

        if val == 0 {
            warn!("layer counter overflowed!");
        }

        Self(val)
    }
}

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OldLayer(Layer);

impl OldLayer {
    pub fn get(&self) -> Layer {
        self.0
    }
}
