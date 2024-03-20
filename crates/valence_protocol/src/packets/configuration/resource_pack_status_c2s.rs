use crate::{Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub enum ResourcePackStatusC2s {
    /// The client has successfully doawnloaded the server's resource pack.
    SuccessfullyDownloaded,
    /// The client has declined the server's resource pack.
    Declined,
    /// The client has failed to download the server's resource pack.
    FailedDownload,
    /// The client has accepted the server's resource pack.
    Accepted,
    /// ??
    Downloaded,
    /// ??
    InvalidURL,
    /// ??
    FailedToReload,
    /// ??
    Discarded,
}
