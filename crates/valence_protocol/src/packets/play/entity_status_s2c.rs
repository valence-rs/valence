use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct EntityStatusS2c {
    pub entity_id: i32,
    pub entity_status: u8,
}
