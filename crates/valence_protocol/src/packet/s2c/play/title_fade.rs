use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct TitleFadeS2c {
    /// Ticks to spend fading in.
    pub fade_in: i32,
    /// Ticks to keep the title displayed.
    pub stay: i32,
    /// Ticks to spend fading out.
    pub fade_out: i32,
}
