use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::RENAME_ITEM_C2S)]
pub struct RenameItemC2s<'a> {
    // Surprisingly, this is not bounded as of 1.20.1.
    pub item_name: &'a str,
}
