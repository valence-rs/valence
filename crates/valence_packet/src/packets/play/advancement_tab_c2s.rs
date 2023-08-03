use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ADVANCEMENT_TAB_C2S)]
pub enum AdvancementTabC2s<'a> {
    OpenedTab { tab_id: Ident<Cow<'a, str>> },
    ClosedScreen,
}
