use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use crate::glm::{self, Number, RealNumber, TVec};

/// An Axis-aligned bounding box in an arbitrary dimension.
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

    pub fn point(pos: TVec<T, D>) -> Self {
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

    pub fn collides_with_aabb(&self, other: &Self) -> bool {
        let l = glm::less_than_equal(&self.min, &other.max);
        let r = glm::greater_than_equal(&self.max, &other.min);
        glm::all(&l.zip_map(&r, |l, r| l && r))
    }
}

impl<T: RealNumber, const D: usize> Aabb<T, D> {
    /// Construct an AABB from a center (centroid) and the dimensions of the box
    /// along each axis.
    pub fn from_center_and_dimensions(center: TVec<T, D>, dims: TVec<T, D>) -> Self {
        let half = dims * T::from_subset(&0.5);
        Self {
            min: center - half,
            max: center + half,
        }
    }

    pub fn center(&self) -> TVec<T, D> {
        // TODO: distribute multiplication to avoid intermediate overflow?
        (self.min + self.max) * T::from_subset(&0.5)
    }

    pub fn collides_with_sphere(&self, center: TVec<T, D>, radius: T) -> bool {
        self.distance_to_point(center) <= radius
    }

    pub fn distance_to_point(&self, p: TVec<T, D>) -> T {
        glm::distance(&p, &glm::clamp_vec(&p, &self.min, &self.max))
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
