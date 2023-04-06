use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ClearTitleS2c {
    pub reset: bool,
}
