use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct DebugSampleSubscriptionC2s {
    pub sample_type: DebugSampleType,
}
#[derive(Clone, Debug, Encode, Decode)]
pub enum DebugSampleType {
    TickTime,
}
