use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SynchronizeTagsS2c<'a> {
    pub groups: Cow<'a, BTreeMap<Ident<String>, RegistryValue>>,
}

pub type RegistryValue = BTreeMap<Ident<String>, Vec<VarInt>>;
