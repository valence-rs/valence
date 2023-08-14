use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::FEATURES_S2C)]
pub struct FeaturesS2c<'a> {
    pub features: Cow<'a, BTreeSet<Ident<String>>>,
}
