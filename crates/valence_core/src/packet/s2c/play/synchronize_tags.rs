use std::borrow::Cow;

use crate::ident::Ident;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SynchronizeTagsS2c<'a> {
    pub tags: Vec<TagGroup<'a>>,
}

#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct TagGroup<'a> {
    pub kind: Ident<Cow<'a, str>>,
    pub tags: Vec<Tag<'a>>,
}

#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct Tag<'a> {
    pub name: Ident<Cow<'a, str>>,
    pub entries: Vec<VarInt>,
}
