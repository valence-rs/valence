use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Handshaking)]
pub struct HandshakeC2s<'a> {
    pub protocol_version: VarInt,
    pub server_address: Bounded<&'a str, 255>,
    pub server_port: u16,
    pub next_state: HandshakeNextState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum HandshakeNextState {
    #[packet(tag = 1)]
    Status,
    #[packet(tag = 2)]
    Login,
}
