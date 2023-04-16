use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct TitleS2c<'a> {
    pub title_text: Cow<'a, Text>,
}
