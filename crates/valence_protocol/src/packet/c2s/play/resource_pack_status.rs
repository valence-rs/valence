use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x24]
pub enum ResourcePackStatusC2s {
    SuccessfullyLoaded,
    Declined,
    FailedDownload,
    Accepted,
}
