use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_SUCCESS_S2C, state = PacketState::Login)]
pub struct LoginSuccessS2c<'a> {
    pub uuid: Uuid,
    pub username: &'a str, // TODO: bound this.
    pub properties: Cow<'a, [Property]>,
}
