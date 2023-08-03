use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::OVERLAY_MESSAGE_S2C)]
pub struct OverlayMessageS2c<'a> {
    pub action_bar_text: Cow<'a, Text>,
}
