use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_KEY_C2S, state = PacketState::Login)]
pub struct LoginKeyC2s<'a> {
    pub shared_secret: &'a [u8],
    pub verify_token: &'a [u8],
}
