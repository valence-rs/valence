use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct RenameItemC2s<'a> {
    // Surprisingly, this is not bounded as of 1.20.1.
    pub item_name: &'a str,
}
