use std::io::Write;

use byteorder::WriteBytesExt;
use serde::{ser, Serialize};

use crate::binary::ser::payload::PayloadSerializer;
use crate::{Error, Tag};

pub struct SerializeStruct<'w, W: ?Sized> {
    pub(super) writer: &'w mut W,
}

impl<W: Write + ?Sized> ser::SerializeStruct for SerializeStruct<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value
            .serialize(&mut PayloadSerializer::named(self.writer, key))
            .map_err(|e| e.field(key))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer.write_u8(Tag::End as u8)?)
    }
}
