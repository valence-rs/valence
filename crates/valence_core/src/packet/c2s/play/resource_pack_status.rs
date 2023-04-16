use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub enum ResourcePackStatusC2s {
    SuccessfullyLoaded,
    Declined,
    FailedDownload,
    Accepted,
}
