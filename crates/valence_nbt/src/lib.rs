#![doc = include_str!("../README.md")]
// Run locally with `RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features --open`
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "binary")]
#[cfg_attr(docsrs, doc(cfg(feature = "binary")))]
pub use binary::{from_binary, to_binary};
pub use compound::Compound;
pub use error::*;
pub use list::List;
pub use tag::*;
pub use value::Value;

#[cfg(feature = "binary")]
#[cfg_attr(docsrs, doc(cfg(feature = "binary")))]
pub mod binary;
pub mod compound;
pub mod conv;
mod error;
pub mod list;
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub mod serde;
#[cfg(feature = "snbt")]
#[cfg_attr(docsrs, doc(cfg(feature = "snbt")))]
pub mod snbt;
mod tag;
pub mod value;

/// A convenience macro for constructing [`Compound`]s.
///
/// Key expressions must implement `Into<String>` while value expressions must
/// implement `Into<Value>`.
///
/// # Examples
///
/// ```
/// use valence_nbt::{compound, List};
///
/// let c = compound! {
///     "byte" => 123_i8,
///     "list_of_int" => List::Int(vec![3, -7, 5]),
///     "list_of_string" => List::String(vec![
///         "foo".to_owned(),
///         "bar".to_owned(),
///         "baz".to_owned()
///     ]),
///     "string" => "aé日",
///     "compound" => compound! {
///         "foo" => 1,
///         "bar" => 2,
///         "baz" => 3,
///     },
///     "int_array" => vec![5, -9, i32::MIN, 0, i32::MAX],
///     "byte_array" => vec![0_i8, 2, 3],
///     "long_array" => vec![123_i64, 456, 789],
/// };
///
/// println!("{c:?}");
/// ```
///
/// It is also possible to specify a custom string type like this:
/// ```
/// # use std::borrow::Cow;
///
/// use valence_nbt::compound;
///
/// let c = compound! { <Cow<str>>
///     "foo" => 123_i8,
/// };
///
/// println!("{c:?}");
/// ```
#[macro_export]
macro_rules! compound {
    (<$string_type:ty> $($key:expr => $value:expr),* $(,)?) => {
        <$crate::Compound<$string_type> as ::std::iter::FromIterator<($string_type, $crate::Value<$string_type>)>>::from_iter([
            $(
                (
                    ::std::convert::Into::<$string_type>::into($key),
                    ::std::convert::Into::<$crate::Value<$string_type>>::into($value)
                ),
            )*
        ])
    };

    ($($key:expr => $value:expr),* $(,)?) => {
        compound!(<::std::string::String> $($key => $value),*)
    };
}

/// A convenience macro for constructing [`Compound`]`<`[`JavaString`]`>`s
///
/// [`JavaString`]: java_string::JavaString
#[cfg(feature = "java_string")]
#[cfg_attr(docsrs, doc(cfg(feature = "java_string")))]
#[macro_export]
macro_rules! jcompound {
    ($($key:expr => $value:expr),* $(,)?) => {
        compound!(<::java_string::JavaString> $($key => $value),*)
    }
}
