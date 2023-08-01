use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_HELLO_S2C, state = PacketState::Login)]
pub struct LoginHelloS2c<'a> {
    pub server_id: &'a str,
    pub public_key: &'a [u8],
    pub verify_token: &'a [u8],
}
