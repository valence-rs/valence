use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct EntityEventS2c {
    pub entity_id: i32,
    pub entity_status: u8,
}
