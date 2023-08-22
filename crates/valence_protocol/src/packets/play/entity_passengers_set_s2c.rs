use std::borrow::Cow;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct EntityPassengersSetS2c<'a> {
    /// Vehicle's entity id
    pub entity_id: VarInt,
    pub passengers: Cow<'a, [VarInt]>,
}
