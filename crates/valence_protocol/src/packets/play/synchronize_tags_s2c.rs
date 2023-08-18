use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SYNCHRONIZE_TAGS_S2C)]
pub struct SynchronizeTagsS2c<'a> {
    pub groups: Cow<'a, BTreeMap<Ident<String>, RegistryValue>>,
}

pub type RegistryValue = BTreeMap<Ident<String>, Vec<VarInt>>;
