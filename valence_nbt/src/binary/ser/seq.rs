use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};
use serde::{ser, Serialize};

use crate::binary::ser::payload::PayloadSerializer;
use crate::{Error, Tag};

pub struct SerializeSeq<'w, W: ?Sized> {
    writer: &'w mut W,
    element_tag: Tag,
    remaining: i32,
    list_or_array: ListOrArray,
}

#[derive(Copy, Clone)]
enum ListOrArray {
    List,
    Array,
}

impl ListOrArray {
    pub const fn name(self) -> &'static str {
        match self {
            ListOrArray::List => "list",
            ListOrArray::Array => "array",
        }
    }
}

impl<'w, W: Write + ?Sized> SerializeSeq<'w, W> {
    pub(super) fn list(writer: &'w mut W, length: i32) -> Self {
        Self {
            writer,
            element_tag: Tag::End,
            remaining: length,
            list_or_array: ListOrArray::List,
        }
    }

    pub(super) fn array(writer: &'w mut W, element_tag: Tag, length: i32) -> Self {
        Self {
            writer,
            element_tag,
            remaining: length,
            list_or_array: ListOrArray::Array,
        }
    }
}

impl<W: Write + ?Sized> ser::SerializeSeq for SerializeSeq<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if self.remaining <= 0 {
            return Err(Error::new_owned(format!(
                "attempt to serialize more {} elements than specified",
                self.list_or_array.name()
            )));
        }

        match self.list_or_array {
            ListOrArray::List => {
                if self.element_tag == Tag::End {
                    let mut ser =
                        PayloadSerializer::first_list_element(self.writer, self.remaining);

                    value.serialize(&mut ser)?;

                    self.element_tag = ser.written_tag().expect("tag must have been written");
                } else {
                    value.serialize(&mut PayloadSerializer::seq_element(
                        self.writer,
                        self.element_tag,
                    ))?;
                }
            }
            ListOrArray::Array => {
                value.serialize(&mut PayloadSerializer::seq_element(
                    self.writer,
                    self.element_tag,
                ))?;
            }
        }

        self.remaining -= 1;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.remaining > 0 {
            return Err(Error::new_owned(format!(
                "{} {} element(s) left to serialize",
                self.remaining,
                self.list_or_array.name()
            )));
        }

        match self.list_or_array {
            ListOrArray::List => {
                // Were any elements written?
                if self.element_tag == Tag::End {
                    // Element type
                    self.writer.write_u8(Tag::End as u8)?;
                    // List length.
                    self.writer.write_i32::<BigEndian>(0)?;
                }
            }
            ListOrArray::Array => {
                // Array length should be written by the serializer already.
            }
        }

        Ok(())
    }
}
