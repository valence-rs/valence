use uuid::Uuid;

use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
pub struct ResourcePackC2s {
    pub uuid: Uuid,
    pub result: ResourcePackStatus,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum ResourcePackStatus {
    /// The client has successfully loaded the server's resource pack.
    SuccessfullyLoaded,
    /// The client has declined the server's resource pack.
    Declined,
    /// The client has failed to download the server's resource pack.
    FailedDownload,
    /// The client has accepted the server's resource pack.
    Accepted,
    Downloaded,
    InvalidUrl,
    FailedToReload,
    Discarded,
}
