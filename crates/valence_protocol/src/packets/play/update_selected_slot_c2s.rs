use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_SELECTED_SLOT_C2S)]
pub struct UpdateSelectedSlotC2s {
    pub slot: u16,
}
