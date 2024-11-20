use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct BundleItemSelectedC2s {
    pub slot_of_bundle: VarInt,
    pub slot_in_bundle: VarInt,
}
