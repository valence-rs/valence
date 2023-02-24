use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityStatusS2c {
    pub entity_id: i32,
    pub entity_status: u8,
}
