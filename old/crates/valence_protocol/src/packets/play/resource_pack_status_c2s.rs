use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
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
