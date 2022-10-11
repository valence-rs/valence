//! Resource identifiers.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::io::Write;
use std::str::FromStr;
use std::{fmt, hash};

use ascii::{AsAsciiStr, AsciiChar, AsciiStr, IntoAsciiString};
use hash::Hash;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::nbt;
use crate::protocol::{Decode, Encode};

/// A resource identifier is a string divided into a "namespace" part and a
/// "path" part. For instance `minecraft:apple` and `valence:frobnicator` are
/// both valid identifiers.
///
/// If the namespace part is left off (the part before and including the colon)
/// the namespace is considered to be "minecraft" for the purposes of equality,
/// ordering, and hashing.
///
/// A string must match the regex `^([a-z0-9_-]+:)?[a-z0-9_\/.-]+$` to be a
/// valid identifier.
#[derive(Clone, Eq)]
pub struct Ident<'a> {
    string: Cow<'a, AsciiStr>,
    /// The index of the ':' character in the string.
    /// If there is no namespace then it is `usize::MAX`.
    ///
    /// Since the string only contains ASCII characters, we can slice it
    /// in O(1) time.
    colon_idx: usize,
}

/// The error type created when an [`Ident`] cannot be parsed from a
/// string. Contains the offending string.
#[derive(Clone, Debug, Error)]
#[error("invalid resource identifier \"{0}\"")]
pub struct IdentParseError<'a>(pub Cow<'a, str>);

impl<'a> Ident<'a> {
    /// Parses a new identifier from a string.
    ///
    /// An error is returned containing the input string if it is not a valid
    /// resource identifier.
    pub fn new(string: impl Into<Cow<'a, str>>) -> Result<Ident<'a>, IdentParseError<'a>> {
        #![allow(bindings_with_variant_name)]

        let cow = match string.into() {
            Cow::Borrowed(s) => {
                Cow::Borrowed(s.as_ascii_str().map_err(|_| IdentParseError(s.into()))?)
            }
            Cow::Owned(s) => Cow::Owned(
                s.into_ascii_string()
                    .map_err(|e| IdentParseError(e.into_source().into()))?,
            ),
        };

        let str = cow.as_ref();

        let check_namespace = |s: &AsciiStr| {
            !s.is_empty()
                && s.chars()
                    .all(|c| matches!(c.as_char(), 'a'..='z' | '0'..='9' | '_' | '-'))
        };
        let check_path = |s: &AsciiStr| {
            !s.is_empty()
                && s.chars()
                    .all(|c| matches!(c.as_char(), 'a'..='z' | '0'..='9' | '_' | '/' | '.' | '-'))
        };

        match str.chars().position(|c| c == AsciiChar::Colon) {
            Some(colon_idx)
                if check_namespace(&str[..colon_idx]) && check_path(&str[colon_idx + 1..]) =>
            {
                Ok(Self {
                    string: cow,
                    colon_idx,
                })
            }
            None if check_path(str) => Ok(Self {
                string: cow,
                colon_idx: usize::MAX,
            }),
            _ => Err(IdentParseError(ascii_cow_to_str_cow(cow))),
        }
    }

    /// Returns the namespace part of this resource identifier.
    ///
    /// If this identifier was constructed from a string without a namespace,
    /// then "minecraft" is returned.
    pub fn namespace(&self) -> &str {
        if self.colon_idx != usize::MAX {
            self.string[..self.colon_idx].as_str()
        } else {
            "minecraft"
        }
    }

    /// Returns the path part of this resource identifier.
    pub fn path(&self) -> &str {
        if self.colon_idx == usize::MAX {
            self.string.as_str()
        } else {
            self.string[self.colon_idx + 1..].as_str()
        }
    }

    /// Returns the underlying string as a `str`.
    pub fn as_str(&self) -> &str {
        self.string.as_str()
    }

    /// Consumes the identifier and returns the underlying string.
    pub fn into_inner(self) -> Cow<'a, str> {
        ascii_cow_to_str_cow(self.string)
    }

    /// Used as the argument to `#[serde(deserialize_with = "...")]` when you
    /// don't want to borrow data from the `'de` lifetime.
    pub fn deserialize_to_owned<'de, D>(deserializer: D) -> Result<Ident<'static>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ident::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

fn ascii_cow_to_str_cow(cow: Cow<AsciiStr>) -> Cow<str> {
    match cow {
        Cow::Borrowed(s) => Cow::Borrowed(s.as_str()),
        Cow::Owned(s) => Cow::Owned(s.into()),
    }
}

impl<'a> fmt::Debug for Ident<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ident").field(&self.as_str()).finish()
    }
}

impl<'a> FromStr for Ident<'a> {
    type Err = IdentParseError<'a>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ident::new(s.to_owned())
    }
}

impl<'a> From<Ident<'a>> for String {
    fn from(id: Ident) -> Self {
        id.string.into_owned().into()
    }
}

impl<'a> From<Ident<'a>> for Cow<'a, str> {
    fn from(id: Ident<'a>) -> Self {
        ascii_cow_to_str_cow(id.string)
    }
}

impl<'a> AsRef<str> for Ident<'a> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a, 'b> PartialEq<Ident<'b>> for Ident<'a> {
    fn eq(&self, other: &Ident<'b>) -> bool {
        (self.namespace(), self.path()) == (other.namespace(), other.path())
    }
}

impl<'a, 'b> PartialOrd<Ident<'b>> for Ident<'a> {
    fn partial_cmp(&self, other: &Ident<'b>) -> Option<Ordering> {
        (self.namespace(), self.path()).partial_cmp(&(other.namespace(), other.path()))
    }
}

impl<'a> Hash for Ident<'a> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.namespace().hash(state);
        self.path().hash(state);
    }
}

impl<'a> fmt::Display for Ident<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.namespace(), self.path())
    }
}

impl<'a> TryFrom<String> for Ident<'a> {
    type Error = IdentParseError<'a>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ident::new(value)
    }
}

impl<'a> TryFrom<&'a str> for Ident<'a> {
    type Error = IdentParseError<'a>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Ident::new(value)
    }
}

impl<'a> From<Ident<'a>> for nbt::Value {
    fn from(id: Ident<'a>) -> Self {
        String::from(id).into()
    }
}

impl<'a> Encode for Ident<'a> {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        self.as_str().encode(w)
    }
}

impl<'a> Decode for Ident<'a> {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(Ident::new(String::decode(r)?)?)
    }
}

impl<'a> Serialize for Ident<'a> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_str().serialize(serializer)
    }
}

/// This uses borrowed data from the `'de` lifetime. If you just want owned
/// data, see [`Ident::deserialize_to_owned`].
impl<'de> Deserialize<'de> for Ident<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_string(IdentVisitor)
    }
}

struct IdentVisitor;

impl<'de> Visitor<'de> for IdentVisitor {
    type Value = Ident<'de>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a valid Minecraft resource identifier")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        dbg!("foo");

        Ident::from_str(s).map_err(E::custom)
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        dbg!("bar");

        Ident::new(v).map_err(E::custom)
    }

    fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
        dbg!("baz");

        Ident::new(s).map_err(E::custom)
    }
}

/// Convenience macro for constructing an [`Ident`] from a format string.
///
/// The arguments to this macro are forwarded to [`std::format_args`].
///
/// # Panics
///
/// The macro will cause a panic if the formatted string is not a valid
/// identifier. See [`Ident`] for more information.
///
/// # Examples
///
/// ```
/// use valence::ident;
///
/// let namespace = "my_namespace";
/// let path = ident!("{namespace}:my_path");
///
/// assert_eq!(path.namespace(), "my_namespace");
/// assert_eq!(path.path(), "my_path");
/// ```
#[macro_export]
macro_rules! ident {
    ($($arg:tt)*) => {{
        let errmsg = "invalid resource identifier in `ident` macro";
        #[allow(clippy::redundant_closure_call)]
        (|args: ::std::fmt::Arguments| match args.as_str() {
            Some(s) => $crate::ident::Ident::new(s).expect(errmsg),
            None => $crate::ident::Ident::new(args.to_string()).expect(errmsg),
        })(format_args!($($arg)*))
    }}
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    use super::*;

    #[test]
    fn parse_valid() {
        ident!("minecraft:whatever");
        ident!("_what-ever55_:.whatever/whatever123456789_");
        ident!("valence:frobnicator");
    }

    #[test]
    #[should_panic]
    fn parse_invalid_0() {
        ident!("");
    }

    #[test]
    #[should_panic]
    fn parse_invalid_1() {
        ident!(":");
    }

    #[test]
    #[should_panic]
    fn parse_invalid_2() {
        ident!("foo:bar:baz");
    }

    #[test]
    fn equality() {
        assert_eq!(ident!("minecraft:my.identifier"), ident!("my.identifier"));
    }

    #[test]
    fn equal_hash() {
        let mut h1 = DefaultHasher::new();
        ident!("minecraft:my.identifier").hash(&mut h1);

        let mut h2 = DefaultHasher::new();
        ident!("my.identifier").hash(&mut h2);

        assert_eq!(h1.finish(), h2.finish());
    }

    fn check_borrowed(id: Ident) {
        if let Cow::Owned(_) = id.into_inner() {
            panic!("not borrowed!");
        }
    }

    #[test]
    fn literal_is_borrowed() {
        check_borrowed(ident!("akjghsjkhebf"));
    }

    #[test]
    fn visit_borrowed_str_works() {
        let data = String::from("valence:frobnicator");

        check_borrowed(
            IdentVisitor
                .visit_borrowed_str::<de::value::Error>(data.as_ref())
                .unwrap(),
        );
    }
}
