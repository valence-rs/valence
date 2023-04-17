use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RenameItemC2s<'a> {
    pub item_name: &'a str,
}
