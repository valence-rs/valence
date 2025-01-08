use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PickItemFromEntityC2s {
    pub entity_id: i32,
    pub include_data: bool,
}
