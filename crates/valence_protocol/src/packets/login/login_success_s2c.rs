use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
pub struct LoginSuccessS2c<'a> {
    pub uuid: Uuid,
    pub username: Bounded<&'a str, 16>,
    pub properties: Cow<'a, [PropertyValue]>,
}
