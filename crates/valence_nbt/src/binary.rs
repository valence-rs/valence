//! Support for serializing and deserializing compounds in Java edition's binary
//! format.
//!
//! # Examples
//!
//! ```
//! use valence_nbt::{compound, Compound, List};
//!
//! let c = compound! {
//!     "byte" => 5_i8,
//!     "string" => "hello",
//!     "list_of_float" => List::Float(vec![
//!         3.1415,
//!         2.7182,
//!         1.4142
//!     ]),
//! };
//!
//! let mut buf = vec![];
//!
//! c.to_binary(&mut buf, "").unwrap();
//! ```
//!
//! Decode NBT data from its binary form.
//!
//! ```
//! use valence_nbt::{compound, Compound};
//!
//! let some_bytes = [10, 0, 0, 3, 0, 3, 105, 110, 116, 0, 0, 222, 173, 0];
//!
//! let expected_value = compound! {
//!     "int" => 0xdead
//! };
//!
//! let (nbt, root_name) = Compound::from_binary(&mut some_bytes.as_slice()).unwrap();
//!
//! assert_eq!(nbt, expected_value);
//! assert_eq!(root_name, "");
//! ```

mod encode;
mod decode;
mod error;
mod modified_utf8;
#[cfg(test)]
mod tests;

pub use error::*;
