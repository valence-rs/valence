use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateSelectedSlotS2c {
    pub slot: u8,
}
