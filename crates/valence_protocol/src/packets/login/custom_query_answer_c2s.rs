use crate::{Bounded, Decode, Encode, Packet, PacketState, RawBytes, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
/// Sent by the client to the server in response to a
/// [`CustomQueryS2c`](crate::packets::login::CustomQueryS2c) packet.
pub struct CustomQueryAnswerC2s<'a> {
    pub message_id: VarInt,
    pub data: Bounded<RawBytes<'a>, 1048576>,
}
