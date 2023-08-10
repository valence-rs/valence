use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::RENAME_ITEM_C2S)]
pub struct RenameItemC2s<'a> {
    pub item_name: &'a str,
}
