#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

pub use compound::Compound;
pub use tag::Tag;
pub use value::{List, Value};

#[cfg(feature = "binary")]
pub mod binary;
pub mod compound;
#[cfg(feature = "serde")]
pub mod serde;
#[cfg(feature = "snbt")]
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
#[macro_export]
macro_rules! compound {
    ($($key:expr => $value:expr),* $(,)?) => {
        <$crate::Compound as ::std::iter::FromIterator<(::std::string::String, $crate::Value)>>::from_iter([
            $(
                (
                    ::std::convert::Into::<::std::string::String>::into($key),
                    ::std::convert::Into::<$crate::Value>::into($value)
                ),
            )*
        ])
    }
}
