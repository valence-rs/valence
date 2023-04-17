use std::borrow::Cow;

use crate::ident::Ident;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub enum AdvancementTabC2s<'a> {
    OpenedTab { tab_id: Ident<Cow<'a, str>> },
    ClosedScreen,
}
