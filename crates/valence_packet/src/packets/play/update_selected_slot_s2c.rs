use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_SELECTED_SLOT_S2C)]
pub struct UpdateSelectedSlotS2c {
    pub slot: u8,
}
