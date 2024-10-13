use crate::{Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
/// Sent by the client to the server as a response to the 
/// [KeepAliveS2c](crate::packets::configuration::KeepAliveS2c) packet.
/// The id is the same as the one sent by the server. if a client does not respond to a `KeepAliveS2c`
/// packet within 15 seconds, the server should disconnect the client.
pub struct KeepAliveC2s(pub i32);
