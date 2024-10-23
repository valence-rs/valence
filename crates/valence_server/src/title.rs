use valence_protocol::encode::WritePacket;
use valence_protocol::packets::play::{
    ClearTitlesS2c, SetActionBarTextS2c, SetSubtitleTextS2c, SetTitleTextS2c, SetTitlesAnimationS2c,
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
        self.write_packet(&SetTitleTextS2c {
            title_text: text.into_cow_text(),
        });
    }

    fn set_subtitle<'a>(&mut self, text: impl IntoText<'a>) {
        self.write_packet(&SetSubtitleTextS2c {
            subtitle_text: text.into_cow_text(),
        });
    }

    fn set_action_bar<'a>(&mut self, text: impl IntoText<'a>) {
        self.write_packet(&SetActionBarTextS2c {
            action_bar_text: text.into_cow_text(),
        });
    }

    fn set_title_times(&mut self, fade_in: i32, stay: i32, fade_out: i32) {
        self.write_packet(&SetTitlesAnimationS2c {
            fade_in,
            stay,
            fade_out,
        });
    }

    fn clear_title(&mut self) {
        self.write_packet(&ClearTitlesS2c { reset: false });
    }

    fn reset_title(&mut self) {
        self.write_packet(&ClearTitlesS2c { reset: true });
    }
}
