use serde::Deserialize;

use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SYNCHRONIZE_TAGS_S2C)]
pub struct SynchronizeTagsS2c<'a> {
    pub registries: Cow<'a, [Registry]>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Registry {
    pub registry: Ident<String>,
    pub tags: Vec<TagEntry>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Encode, Decode)]
pub struct TagEntry {
    pub name: Ident<String>,
    pub entries: Vec<VarInt>,
}
