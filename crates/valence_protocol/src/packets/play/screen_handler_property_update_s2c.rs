use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ScreenHandlerPropertyUpdateS2c {
    pub window_id: u8,
    pub property: i16,
    pub value: i16,
}
