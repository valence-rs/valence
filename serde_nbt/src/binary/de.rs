use std::io::Read;

pub use root::RootDeserializer as Deserializer;
use serde::de::DeserializeOwned;

use crate::Error;

mod array;
mod compound;
mod list;
mod payload;
mod root;

/// Reads uncompressed NBT binary data from the provided reader.
///
/// The name of the root compound is discarded. If you need access to it, see
/// [`Deserializer`].
pub fn from_reader<R, T>(reader: R) -> Result<T, Error>
where
    R: Read,
    T: DeserializeOwned,
{
    T::deserialize(&mut Deserializer::new(reader, false))
}
