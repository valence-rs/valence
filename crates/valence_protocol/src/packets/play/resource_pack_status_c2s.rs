use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::RESOURCE_PACK_STATUS_C2S)]
pub enum ResourcePackStatusC2s {
    /// The client has successfully loaded the server's resource pack.
    SuccessfullyLoaded,
    /// The client has declined the server's resource pack.
    Declined,
    /// The client has failed to download the server's resource pack.
    FailedDownload,
    /// The client has accepted the server's resource pack.
    Accepted,
}
