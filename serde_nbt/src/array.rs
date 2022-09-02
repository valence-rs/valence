use std::fmt::Formatter;
use std::marker::PhantomData;

use serde::de::value::SeqAccessDeserializer;
use serde::de::{EnumAccess, IgnoredAny, SeqAccess, VariantAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    ARRAY_ENUM_NAME, BYTE_ARRAY_VARIANT_NAME, INT_ARRAY_VARIANT_NAME, LONG_ARRAY_VARIANT_NAME,
};

macro_rules! def_mod {
    ($index:literal, $mod_name:ident, $display_name:literal, $variant_name:ident) => {
        /// Provides (de)serialization support for the NBT type
        #[doc = concat!(" \"", $display_name, "\".")]
        ///
        /// This module is intended to be the target of serde's `#[serde(with =
        /// "module")]` field attribute.
        ///
        /// The target field must serialize and deserialize as a seq.
        ///
        /// # Examples
        ///
        /// ```
        /// use serde::{Deserialize, Serialize};
        /// use serde_nbt::binary::to_writer;
        ///
        /// #[derive(Serialize, Deserialize)]
        /// struct MyStruct {
        ///     #[serde(with = "serde_nbt::int_array")]
        ///     array: Vec<i32>,
        /// }
        ///
        /// let s = MyStruct {
        ///     array: vec![1, 2, 3],
        /// };
        ///
        /// let mut buf = Vec::new();
        /// to_writer(&mut buf, &s).unwrap();
        /// ```
        pub mod $mod_name {
            use super::*;

            pub fn serialize<T, S>(array: &T, serializer: S) -> Result<S::Ok, S::Error>
            where
                T: Serialize,
                S: Serializer,
            {
                serializer.serialize_newtype_variant(ARRAY_ENUM_NAME, $index, $variant_name, array)
            }

            pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
            where
                T: Deserialize<'de>,
                D: Deserializer<'de>,
            {
                struct ArrayVisitor<T>(PhantomData<T>);

                impl<'de, T: Deserialize<'de>> Visitor<'de> for ArrayVisitor<T> {
                    type Value = T;

                    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                        write!(
                            formatter,
                            concat!("an NBT ", $display_name, " encoded as an enum or seq")
                        )
                    }

                    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: SeqAccess<'de>,
                    {
                        T::deserialize(SeqAccessDeserializer::new(seq))
                    }

                    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                    where
                        A: EnumAccess<'de>,
                    {
                        // Ignore the variant name.
                        let (_, variant) = data.variant::<IgnoredAny>()?;

                        variant.newtype_variant()
                    }
                }

                let variants = &[
                    BYTE_ARRAY_VARIANT_NAME,
                    INT_ARRAY_VARIANT_NAME,
                    LONG_ARRAY_VARIANT_NAME,
                ];

                deserializer.deserialize_enum(ARRAY_ENUM_NAME, variants, ArrayVisitor(PhantomData))
            }
        }
    };
}

def_mod!(0, byte_array, "byte array", BYTE_ARRAY_VARIANT_NAME);
def_mod!(1, int_array, "int array", INT_ARRAY_VARIANT_NAME);
def_mod!(2, long_array, "long array", LONG_ARRAY_VARIANT_NAME);
