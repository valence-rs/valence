use valence_core::protocol::{packet_id, Decode, Encode};
use valence_core::text::IntoText;

use super::*;

pub trait SetTitle {
    /// Displays a title to a client.
    ///
    /// A title is a large piece of text displayed in the center of the screen
    /// which may also include a subtitle underneath it. The title can be
    /// configured to fade in and out using
    /// [`set_title_times`](Self::set_title_times).
    fn set_title<'a>(&mut self, text: impl IntoText<'a>);

    fn set_subtitle<'a>(&mut self, text: impl IntoText<'a>);

    fn set_action_bar<'a>(&mut self, text: impl IntoText<'a>);

    /// - `fade_in`: Ticks to spend fading in.
    /// - `stay`: Ticks to keep the title displayed.
    /// - `fade_out`: Ticks to spend fading out.
    fn set_title_times(&mut self, fade_in: i32, stay: i32, fade_out: i32);

    fn clear_title(&mut self);

    fn reset_title(&mut self);
}

impl<T: WritePacket> SetTitle for T {
    fn set_title<'a>(&mut self, text: impl IntoText<'a>) {
        self.write_packet(&TitleS2c {
            title_text: text.into_cow_text(),
        });
    }

    fn set_subtitle<'a>(&mut self, text: impl IntoText<'a>) {
        self.write_packet(&SubtitleS2c {
            subtitle_text: text.into_cow_text(),
        });
    }

    fn set_action_bar<'a>(&mut self, text: impl IntoText<'a>) {
        self.write_packet(&OverlayMessageS2c {
            action_bar_text: text.into_cow_text(),
        });
    }

    fn set_title_times(&mut self, fade_in: i32, stay: i32, fade_out: i32) {
        self.write_packet(&TitleFadeS2c {
            fade_in,
            stay,
            fade_out,
        });
    }

    fn clear_title(&mut self) {
        self.write_packet(&ClearTitleS2c { reset: false });
    }

    fn reset_title(&mut self) {
        self.write_packet(&ClearTitleS2c { reset: true });
    }
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::TITLE_S2C)]
pub struct TitleS2c<'a> {
    pub title_text: Cow<'a, Text>,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SUBTITLE_S2C)]
pub struct SubtitleS2c<'a> {
    pub subtitle_text: Cow<'a, Text>,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::OVERLAY_MESSAGE_S2C)]
pub struct OverlayMessageS2c<'a> {
    pub action_bar_text: Cow<'a, Text>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::TITLE_FADE_S2C)]
pub struct TitleFadeS2c {
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLEAR_TITLE_S2C)]
pub struct ClearTitleS2c {
    pub reset: bool,
}
