use std::io::Write;

use anyhow::anyhow;
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
        Err(_) => return Err(Error(anyhow!("string byte length exceeds u16::MAX"))),
    };

    writer.write_all(&data)?;
    Ok(())
}
