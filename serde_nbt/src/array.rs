use serde::{Serialize, Serializer};
use serde::ser::SerializeTupleStruct;

use super::{BYTE_ARRAY_MAGIC, INT_ARRAY_MAGIC, LONG_ARRAY_MAGIC};

macro_rules! def {
    ($name:ident, $magic:ident) => {
        pub fn $name<T, S>(array: T, serializer: S) -> Result<S::Ok, S::Error>
        where
            T: IntoIterator,
            T::IntoIter: ExactSizeIterator,
            T::Item: Serialize,
            S: Serializer,
        {
            let it = array.into_iter();
            let mut sts = serializer.serialize_tuple_struct($magic, it.len())?;

            for item in it {
                sts.serialize_field(&item)?;
            }

            sts.end()
        }
    }
}

def!(byte_array, BYTE_ARRAY_MAGIC);
def!(int_array, INT_ARRAY_MAGIC);
def!(long_array, LONG_ARRAY_MAGIC);
