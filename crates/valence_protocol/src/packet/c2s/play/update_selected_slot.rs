use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateSelectedSlotC2s {
    pub slot: i16,
}
