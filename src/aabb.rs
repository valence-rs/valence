use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use num::cast::AsPrimitive;

use crate::glm::{self, Number, RealNumber, TVec, TVec3};

/// An Axis-aligned bounding box in an arbitrary dimension, defined by its
/// minimum and maximum corners.
///
/// This type maintains the invariant that `min <= max` componentwise.
///
/// The generic type `T` can be an integer, which is useful for AABBs on grids.
#[derive(Clone, Copy, Debug)]
pub struct Aabb<T, const D: usize> {
    min: TVec<T, D>,
    max: TVec<T, D>,
}

impl<T: Number, const D: usize> Aabb<T, D> {
    pub fn new(p0: impl Into<TVec<T, D>>, p1: impl Into<TVec<T, D>>) -> Self {
        let p0 = p0.into();
        let p1 = p1.into();
        Self {
            min: glm::min2(&p0, &p1),
            max: glm::max2(&p0, &p1),
        }
    }

    pub fn point(pos: impl Into<TVec<T, D>>) -> Self {
        let pos = pos.into();
        Self { min: pos, max: pos }
    }

    pub fn min(&self) -> TVec<T, D> {
        self.min
    }

    pub fn max(&self) -> TVec<T, D> {
        self.max
    }

    pub fn dimensions(&self) -> TVec<T, D> {
        self.max - self.min
    }

    /// Moves this AABB by some vector.
    pub fn translate(&self, v: impl Into<TVec<T, D>>) -> Self {
        let v = v.into();
        Self {
            min: self.min + v,
            max: self.max + v,
        }
    }

    /// Calculates the AABB union, which is the smallest AABB completely
    /// encompassing both AABBs.
    pub fn union(&self, other: Self) -> Self {
        Self {
            min: glm::min2(&self.min, &other.min),
            max: glm::max2(&self.max, &other.max),
        }
    }

    pub fn collides_with_aabb(&self, other: &Self) -> bool {
        let l = glm::less_than_equal(&self.min, &other.max);
        let r = glm::greater_than_equal(&self.max, &other.min);
        glm::all(&l.zip_map(&r, |l, r| l && r))
    }
}

impl<T: Number, const D: usize> Aabb<T, D>
where
    i32: AsPrimitive<T>,
{
    /// Returns the center (centroid) of this AABB.
    pub fn center(&self) -> TVec<T, D> {
        (self.min + self.max).map(|c| c / 2.as_())
    }
}

impl<T: RealNumber, const D: usize> Aabb<T, D> {
    /// Construct an AABB from a center (centroid) and the dimensions of the box
    /// along each axis.
    pub fn from_center_and_dimensions(
        center: impl Into<TVec<T, D>>,
        dims: impl Into<TVec<T, D>>,
    ) -> Self {
        let half = dims.into() * T::from_subset(&0.5);
        let center = center.into();
        Self {
            min: center - half,
            max: center + half,
        }
    }

    pub fn collides_with_sphere(
        &self,
        center: impl Into<TVec<T, D>>,
        radius: impl Into<T>,
    ) -> bool {
        self.distance_to_point(center.into()) <= radius.into()
    }

    pub fn distance_to_point(&self, p: impl Into<TVec<T, D>>) -> T {
        let p = p.into();
        glm::distance(&p, &glm::clamp_vec(&p, &self.min, &self.max))
    }
}

impl<T: Number> Aabb<T, 3>
where
    i32: AsPrimitive<T>,
{
    pub fn surface_area(&self) -> T {
        let d = self.dimensions();
        (d.x * d.y + d.y * d.z + d.z * d.x) * 2.as_()
    }
}

impl<T: RealNumber> Aabb<T, 3> {
    /// Constructs an AABB from a position and the dimensions of the box along
    /// each axis. The position is the center of the bottom face of the AABB.
    pub fn from_bottom_and_dimensions(
        bottom: impl Into<TVec3<T>>,
        dims: impl Into<TVec3<T>>,
    ) -> Self {
        let dims = dims.into();
        Self::from_center_and_dimensions(bottom, dims).translate([
            T::from_subset(&0.0),
            dims.y * T::from_subset(&0.5),
            T::from_subset(&0.0),
        ])
    }
}

impl<T: Number + Default, const D: usize> Default for Aabb<T, D> {
    fn default() -> Self {
        let d = T::default();
        Self::new([d; D], [d; D])
    }
}

impl<T: Number, const D: usize> PartialEq for Aabb<T, D> {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

impl<T: Number + Eq, const D: usize> Eq for Aabb<T, D> {}

impl<T: Number, const D: usize> PartialOrd for Aabb<T, D> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.min.partial_cmp(&other.min) {
            Some(Ordering::Equal) => self.max.partial_cmp(&other.max),
            ord => return ord,
        }
    }
}

impl<T: Number + Hash, const D: usize> Hash for Aabb<T, D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.min.hash(state);
        self.max.hash(state);
    }
}

// TODO: impl Ord for Aabb
//impl<T: Number + Ord, const D: usize> Ord for Aabb<T, D> {
//    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//        match self.min.cmp(&other.min) {
//            Ordering::Equal => self.max.cmp(&other.max),
//            ord => ord,
//        }
//    }
//}
