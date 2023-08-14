use std::borrow::Borrow;

use derive_more::{AsRef, Deref, DerefMut, From};

/// A newtype wrapper for `T` which modifies the [`Encode`](crate::Encode) and
/// [`Decode`](crate::Decode) impls to be bounded by some upper limit `MAX`.
/// Implementations are expected to error eagerly if the limit is exceeded.
///
/// What exactly `MAX` represents depends on the type `T`. Here are some
/// instances:
/// - **
/// - **strings**: The maximum number of _characters_ in the string.
#[derive(
    Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deref, DerefMut, AsRef, From,
)]
pub struct Bounded<T, const MAX: usize>(pub T);

impl<T, const MAX: usize> Bounded<T, MAX> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Bounded<U, MAX> {
        Bounded(f(self.0))
    }

    pub fn map_into<U: From<T>>(self) -> Bounded<U, MAX> {
        Bounded(self.0.into())
    }
}

impl<T, const MAX: usize> Borrow<T> for Bounded<T, MAX> {
    fn borrow(&self) -> &T {
        &self.0
    }
}
