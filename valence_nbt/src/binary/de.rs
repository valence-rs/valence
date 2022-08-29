use std::io::Read;

pub use root::RootDeserializer as Deserializer;
use serde::de::DeserializeOwned;

use crate::Error;

mod array;
mod compound;
mod list;
mod payload;
mod root;

pub fn from_reader<R, T>(reader: R) -> Result<T, Error>
where
    R: Read,
    T: DeserializeOwned,
{
    T::deserialize(&mut Deserializer::new(reader, false))
}
