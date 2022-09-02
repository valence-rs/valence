use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};
use cesu8::to_java_cesu8;
pub use root::RootSerializer as Serializer;
use serde::{ser, Serialize};

use crate::{Error, Result};

mod map;
mod payload;
mod root;
mod seq;
mod structs;

/// Writes uncompressed NBT binary data to the provided writer.
///
/// Note that serialization will fail if the provided value does not serialize
/// as a compound (a map or struct). This is because the NBT format requires the
/// root value to be a named compound.
///
/// The name of the root compound will be `""`. If you want to use a different
/// name, see [`Serializer`].
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize + ?Sized,
{
    value.serialize(&mut Serializer::new(writer, ""))
}

type Impossible = ser::Impossible<(), Error>;

fn write_string(mut writer: impl Write, string: &str) -> Result<()> {
    let data = to_java_cesu8(string);
    match data.len().try_into() {
        Ok(len) => writer.write_u16::<BigEndian>(len)?,
        Err(_) => return Err(Error::new_static("string byte length exceeds u16::MAX")),
    };

    writer.write_all(&data)?;
    Ok(())
}
