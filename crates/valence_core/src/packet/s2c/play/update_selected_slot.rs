use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateSelectedSlotS2c {
    pub slot: u8,
}
