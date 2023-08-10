use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SCREEN_HANDLER_PROPERTY_UPDATE_S2C)]
pub struct ScreenHandlerPropertyUpdateS2c {
    pub window_id: u8,
    pub property: i16,
    pub value: i16,
}
