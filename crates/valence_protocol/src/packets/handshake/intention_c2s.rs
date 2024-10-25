use crate::{Bounded, Decode, Encode, Packet, PacketState, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Handshake)]
/// Sent by the client to the server to indicate its intention to switch to the
/// given state. Either the `Status` or `Login` state will be selected.
pub struct IntentionC2s<'a> {
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
