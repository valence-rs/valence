use crate::{Bounded, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct RenameItemC2s<'a> {
    // In the notican server: The item name may be no longer than 50 characters long, and if it is
    // longer than that, then the rename is silently ignored.
    pub item_name: Bounded<&'a str, 32767>,
}
