use crate::glm::{self, Number, RealNumber, TVec};

/// An Axis-aligned bounding box in an arbitrary dimension.
#[derive(Clone, Copy, Debug)] // TODO: impl PartialEq, Eq, PartialOrd, Ord, Hash
pub struct Aabb<T, const D: usize> {
    min: TVec<T, D>,
    max: TVec<T, D>,
}

impl<T: Number, const D: usize> Aabb<T, D> {
    pub fn new(p0: TVec<T, D>, p1: TVec<T, D>) -> Self {
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
