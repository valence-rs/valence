use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub enum AdvancementTabC2s<'a> {
    OpenedTab { tab_id: Ident<Cow<'a, str>> },
    ClosedScreen,
}
