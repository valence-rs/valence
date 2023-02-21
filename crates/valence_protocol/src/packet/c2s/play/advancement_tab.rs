use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub enum AdvancementTabC2s<'a> {
    OpenedTab { tab_id: Ident<&'a str> },
    ClosedScreen,
}
