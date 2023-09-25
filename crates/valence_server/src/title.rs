use valence_protocol::encode::WritePacket;
use valence_protocol::packets::play::{
    ClearTitleS2c, OverlayMessageS2c, SubtitleS2c, TitleFadeS2c, TitleS2c,
};
use valence_protocol::text::IntoText;

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
