use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_HELLO_C2S, state = PacketState::Login)]
pub struct LoginHelloC2s<'a> {
    pub username: Bounded<&'a str, 16>,
    pub profile_id: Option<Uuid>,
}
