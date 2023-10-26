#![doc = include_str!("../README.md")]
#![allow(unused_imports)]

mod cesu8;
mod char;
mod error;
mod iter;
mod owned;
mod pattern;
#[cfg(feature = "serde")]
mod serde;
mod slice;
pub(crate) mod validations;

pub use char::*;
pub use error::*;
pub use iter::*;
pub use owned::*;
pub use pattern::*;
pub use slice::*;

#[macro_export]
macro_rules! format_java {
    ($($arg:tt)*) => {
        $crate::JavaString::from(::std::format!($($arg)*))
    }
}
