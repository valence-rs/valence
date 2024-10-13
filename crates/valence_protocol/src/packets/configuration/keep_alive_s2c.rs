use crate::{Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
/// Should be sent frequently by the server to the client to keep the connection alive. The client
/// should respond with a [`KeepAliveC2s`](crate::packets::configuration::KeepAliveC2s) packet with
/// the same id. If the client does not receive a `KeepAliveS2c` packet within 20 seconds, it should
/// disconnect.
pub struct KeepAliveS2c(pub i32);