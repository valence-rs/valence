use crate::{Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
/// The client should respond with a [`PongC2s`](crate::packets::configuration::PongC2s) packet with
/// the same id.
pub struct PingS2c(pub i32);