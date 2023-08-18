use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Status)]
pub struct QueryResponseS2c<'a> {
    pub json: &'a str,
}
