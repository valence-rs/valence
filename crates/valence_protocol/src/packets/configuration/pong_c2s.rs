use crate::{Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
// Response to the [PingS2c](crate::packets::configuration::PingS2c) packet.
pub struct PongC2s(pub i32);
