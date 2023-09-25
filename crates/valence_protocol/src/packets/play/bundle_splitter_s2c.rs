use crate::{packet_id, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BUNDLE_SPLITTER)]
pub struct BundleSplitterS2c;
