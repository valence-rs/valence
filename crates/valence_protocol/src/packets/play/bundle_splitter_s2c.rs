use crate::{PacketSide, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(name = "BUNDLE_SPLITTER", side = PacketSide::Clientbound)]
pub struct BundleSplitterS2c;
