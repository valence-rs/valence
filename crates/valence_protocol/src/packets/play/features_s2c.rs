use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct FeaturesS2c<'a> {
    pub features: Cow<'a, BTreeSet<Ident<String>>>,
}
