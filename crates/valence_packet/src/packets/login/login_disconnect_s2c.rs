use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_DISCONNECT_S2C, state = PacketState::Login)]
pub struct LoginDisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
