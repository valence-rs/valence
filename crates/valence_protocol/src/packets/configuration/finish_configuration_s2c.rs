use crate::{Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
/// Sent by the server to the client to finish the configuration process. The client should send a
/// [`FinishConfigurationC2s`](crate::packets::configuration::FinishConfigurationC2s) packet after
/// receiving this packet.
pub struct FinishConfigurationS2c;
