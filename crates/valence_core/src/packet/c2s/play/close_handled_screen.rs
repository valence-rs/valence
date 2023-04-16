use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CloseHandledScreenC2s {
    pub window_id: i8,
}
