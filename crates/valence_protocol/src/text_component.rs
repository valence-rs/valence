use serde::{Deserialize, Serialize};
use valence_text::Text;
#[derive(Clone, Debug, Serialize, Deserialize)]
/// A wrapper around `Text` that encodes and decodes as an NBT string.
pub struct NbtText(Text);

impl NbtText {
    pub fn new(text: Text) -> Self {
        Self(text)
    }

    pub fn into_inner(self) -> Text {
        self.0
    }
}
