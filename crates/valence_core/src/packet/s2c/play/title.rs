use std::borrow::Cow;

use crate::packet::{Decode, Encode};
use crate::text::Text;

#[derive(Clone, Debug, Encode, Decode)]
pub struct TitleS2c<'a> {
    pub title_text: Cow<'a, Text>,
}
