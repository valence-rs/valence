pub use glam::*;

/// An axis-aligned bounding box.
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Aabb {
    // Invariant: min <= max componentwise.
    min: DVec3,
    max: DVec3,
}

impl Aabb {
    pub fn new(p0: impl Into<DVec3>, p1: impl Into<DVec3>) -> Self {
        let p0 = p0.into();
        let p1 = p1.into();
        Self {
            min: p0.min(p1),
            max: p0.max(p1),
        }
    }

    pub(crate) fn new_unchecked(min: DVec3, max: DVec3) -> Self {
        Self { min, max }
    }

    pub(crate) fn from_bottom_size(bottom: impl Into<DVec3>, size: impl Into<DVec3>) -> Self {
        let bottom = bottom.into();
        let size = size.into();

        Self {
            min: DVec3 {
                x: bottom.x - size.x / 2.0,
                y: bottom.y,
                z: bottom.z - size.z / 2.0,
            },
            max: DVec3 {
                x: bottom.x + size.x / 2.0,
                y: bottom.y + size.y,
                z: bottom.z + size.z / 2.0,
            },
        }
    }

    /// The minimum corner of the bounding box.
    pub const fn min(self) -> DVec3 {
        self.min
    }

    /// The maximum corner of the bounding box.
    pub const fn max(self) -> DVec3 {
        self.max
    }
}
