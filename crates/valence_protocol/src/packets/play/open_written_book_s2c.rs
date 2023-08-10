use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::OPEN_WRITTEN_BOOK_S2C)]
pub struct OpenWrittenBookS2c {
    pub hand: Hand,
}
