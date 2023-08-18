use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateSelectedSlotC2s {
    pub slot: u16,
}
