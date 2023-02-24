use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ClearTitlesS2c {
    pub reset: bool,
}
