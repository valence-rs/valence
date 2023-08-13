use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_SUCCESS_S2C, state = PacketState::Login)]
pub struct LoginSuccessS2c<'a> {
    pub uuid: Uuid,
    pub username: Bounded<&'a str, 16>,
    pub properties: Cow<'a, [PropertyValue]>,
}
