use crate::{Decode, Encode, Packet, RawBytes, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct EntityTrackerUpdateS2c<'a> {
    pub entity_id: VarInt,
    pub tracked_values: RawBytes<'a>,
}
