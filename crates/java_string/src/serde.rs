use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{JavaStr, JavaString};

impl Serialize for JavaString {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_str_lossy().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JavaString {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(JavaString::from)
    }
}

impl Serialize for JavaStr {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_str_lossy().serialize(serializer)
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a JavaStr {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        <&'a str>::deserialize(deserializer).map(JavaStr::from_str)
    }
}
