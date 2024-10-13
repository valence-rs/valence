use crate::{Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
/// Sent by the client to the server to finish the configuration process. This packet is sent after
/// [the server sends a `FinishConfigurationS2c` packet](crate::packets::configuration::FinishConfigurationS2c).
/// We should move to the play state after this packet is received.
pub struct FinishConfigurationC2s;
