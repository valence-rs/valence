//! Provides the [`TextFormat`] trait and implementations.

use std::borrow::Cow;

use super::{ClickEvent, Color, HoverEvent, IntoText, Text};

/// Provides the methods necessary for working with [`Text`] objects.
///
/// This trait exists to allow using [`IntoText`] types without
/// having to first convert the type into [`Text`].
///
/// # Usage
///
/// ```
/// # use valence_core::text::{Text, color::NormalColor, TextFormat};
/// # fn usage_example(my_text: &mut Text) {
/// my_text.color(NormalColor::Red).bold();
/// my_text.add_child("CRABBBBB".obfuscated());
/// # }
pub trait TextFormat {
    type ReturnType: Sized;

    fn into_text(self) -> Self::ReturnType;

    /// Sets the color of the text.
    fn color(self, color: impl Into<Color>) -> Self::ReturnType;
    /// Clears the color of the text. Color of parent [`Text`] object will be
    /// used.
    fn clear_color(self) -> Self::ReturnType;

    /// Sets the font of the text. Possible options: `minecraft:uniform`
    /// (Unicode font), `minecraft:alt` (Enchanting table font), or
    /// `minecraft:default` (Default).
    fn font(self, font: impl Into<Cow<'static, str>>) -> Self::ReturnType;
    /// Clears the font of the text. Font of parent [`Text`] object will be
    /// used.
    fn clear_font(self) -> Self::ReturnType;

    /// Makes the text bold.
    fn bold(self) -> Self::ReturnType;
    /// Makes the text not bold.
    fn not_bold(self) -> Self::ReturnType;
    /// Clears the `bold` property of the text. Property of the parent [`Text`]
    /// object will be used.
    fn clear_bold(self) -> Self::ReturnType;

    /// Makes the text italic.
    fn italic(self) -> Self::ReturnType;
    /// Makes the text not italic.
    fn not_italic(self) -> Self::ReturnType;
    /// Clears the `italic` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_italic(self) -> Self::ReturnType;

    /// Makes the text underlined.
    fn underlined(self) -> Self::ReturnType;
    /// Makes the text not underlined.
    fn not_underlined(self) -> Self::ReturnType;
    /// Clears the `underlined` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_underlined(self) -> Self::ReturnType;

    /// Adds a strikethrough effect to the text.
    fn strikethrough(self) -> Self::ReturnType;
    /// Removes the strikethrough effect from the text.
    fn not_strikethrough(self) -> Self::ReturnType;
    /// Clears the `strikethrough` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_strikethrough(self) -> Self::ReturnType;

    /// Makes the text obfuscated.
    fn obfuscated(self) -> Self::ReturnType;
    /// Makes the text not obfuscated.
    fn not_obfuscated(self) -> Self::ReturnType;
    /// Clears the `obfuscated` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_obfuscated(self) -> Self::ReturnType;

    /// Adds an `insertion` property to the text. When shift-clicked, the given
    /// text will be inserted into chat box for the client.
    fn insertion(self, insertion: impl Into<Cow<'static, str>>) -> Self::ReturnType;
    /// Clears the `insertion` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_insertion(self) -> Self::ReturnType;

    /// On click, opens the given URL. Has to be `http` or `https` protocol.
    fn on_click_open_url(self, url: impl Into<Cow<'static, str>>) -> Self::ReturnType;
    /// On click, sends a command. Doesn't actually have to be a command, can be
    /// a simple chat message.
    fn on_click_run_command(self, command: impl Into<Cow<'static, str>>) -> Self::ReturnType;
    /// On click, copies the given text to the chat box.
    fn on_click_suggest_command(self, command: impl Into<Cow<'static, str>>) -> Self::ReturnType;
    /// On click, turns the page of the opened book to the given number.
    /// Indexing starts at `1`.
    fn on_click_change_page(self, page: impl Into<i32>) -> Self::ReturnType;
    /// On click, copies the given text to clipboard.
    fn on_click_copy_to_clipboard(self, text: impl Into<Cow<'static, str>>) -> Self::ReturnType;
    /// Clears the `click_event` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_click_event(self) -> Self::ReturnType;

    /// On mouse hover, shows the given text in a tooltip.
    fn on_hover_show_text(self, text: impl IntoText<'static>) -> Self::ReturnType;
    /// Clears the `hover_event` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_hover_event(self) -> Self::ReturnType;

    /// Adds a child [`Text`] object.
    fn add_child(self, text: impl IntoText<'static>) -> Self::ReturnType;
}

impl<'a> TextFormat for &'a mut Text {
    type ReturnType = &'a mut Text;

    fn into_text(self) -> Self::ReturnType {
        self
    }

    fn color(self, color: impl Into<Color>) -> Self::ReturnType {
        self.color = Some(color.into());
        self
    }
    fn clear_color(self) -> Self::ReturnType {
        self.color = None;
        self
    }

    fn font(self, font: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        self.font = Some(font.into());
        self
    }
    fn clear_font(self) -> Self::ReturnType {
        self.font = None;
        self
    }

    fn bold(self) -> Self::ReturnType {
        self.bold = Some(true);
        self
    }
    fn not_bold(self) -> Self::ReturnType {
        self.bold = Some(false);
        self
    }
    fn clear_bold(self) -> Self::ReturnType {
        self.bold = None;
        self
    }

    fn italic(self) -> Self::ReturnType {
        self.italic = Some(true);
        self
    }
    fn not_italic(self) -> Self::ReturnType {
        self.italic = Some(false);
        self
    }
    fn clear_italic(self) -> Self::ReturnType {
        self.italic = None;
        self
    }

    fn underlined(self) -> Self::ReturnType {
        self.underlined = Some(true);
        self
    }
    fn not_underlined(self) -> Self::ReturnType {
        self.underlined = Some(false);
        self
    }
    fn clear_underlined(self) -> Self::ReturnType {
        self.underlined = None;
        self
    }

    fn strikethrough(self) -> Self::ReturnType {
        self.strikethrough = Some(true);
        self
    }
    fn not_strikethrough(self) -> Self::ReturnType {
        self.strikethrough = Some(false);
        self
    }
    fn clear_strikethrough(self) -> Self::ReturnType {
        self.strikethrough = None;
        self
    }

    fn obfuscated(self) -> Self::ReturnType {
        self.obfuscated = Some(true);
        self
    }
    fn not_obfuscated(self) -> Self::ReturnType {
        self.obfuscated = Some(false);
        self
    }
    fn clear_obfuscated(self) -> Self::ReturnType {
        self.obfuscated = None;
        self
    }

    fn insertion(self, insertion: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        self.insertion = Some(insertion.into());
        self
    }
    fn clear_insertion(self) -> Self::ReturnType {
        self.insertion = None;
        self
    }

    fn on_click_open_url(self, url: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        self.click_event = Some(ClickEvent::OpenUrl(url.into()));
        self
    }
    fn on_click_run_command(self, command: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        self.click_event = Some(ClickEvent::RunCommand(command.into()));
        self
    }
    fn on_click_suggest_command(self, command: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        self.click_event = Some(ClickEvent::SuggestCommand(command.into()));
        self
    }
    fn on_click_change_page(self, page: impl Into<i32>) -> Self::ReturnType {
        self.click_event = Some(ClickEvent::ChangePage(page.into()));
        self
    }
    fn on_click_copy_to_clipboard(self, text: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        self.click_event = Some(ClickEvent::CopyToClipboard(text.into()));
        self
    }
    fn clear_click_event(self) -> Self::ReturnType {
        self.click_event = None;
        self
    }

    fn on_hover_show_text(self, text: impl IntoText<'static>) -> Self::ReturnType {
        self.hover_event = Some(HoverEvent::ShowText(text.into_text()));
        self
    }

    fn clear_hover_event(self) -> Self::ReturnType {
        self.hover_event = None;
        self
    }

    fn add_child(self, text: impl IntoText<'static>) -> Self::ReturnType {
        self.extra.push(text.into_text());
        self
    }
}

impl<'a, T: IntoText<'a>> TextFormat for T {
    type ReturnType = Text;

    fn into_text(self) -> Self::ReturnType {
        self.into_cow_text().into_owned()
    }

    fn color(self, color: impl Into<Color>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.color = Some(color.into());
        value
    }
    fn clear_color(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.color = None;
        value
    }

    fn font(self, font: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.font = Some(font.into());
        value
    }
    fn clear_font(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.font = None;
        value
    }

    fn bold(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.bold = Some(true);
        value
    }
    fn not_bold(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.bold = Some(false);
        value
    }
    fn clear_bold(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.bold = None;
        value
    }

    fn italic(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.italic = Some(true);
        value
    }
    fn not_italic(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.italic = Some(false);
        value
    }
    fn clear_italic(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.italic = None;
        value
    }

    fn underlined(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.underlined = Some(true);
        value
    }
    fn not_underlined(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.underlined = Some(false);
        value
    }
    fn clear_underlined(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.underlined = None;
        value
    }

    fn strikethrough(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.strikethrough = Some(true);
        value
    }
    fn not_strikethrough(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.strikethrough = Some(false);
        value
    }
    fn clear_strikethrough(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.strikethrough = None;
        value
    }

    fn obfuscated(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.obfuscated = Some(true);
        value
    }
    fn not_obfuscated(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.obfuscated = Some(false);
        value
    }
    fn clear_obfuscated(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.obfuscated = None;
        value
    }

    fn insertion(self, insertion: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.insertion = Some(insertion.into());
        value
    }
    fn clear_insertion(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.insertion = None;
        value
    }

    fn on_click_open_url(self, url: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::OpenUrl(url.into()));
        value
    }
    fn on_click_run_command(self, command: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::RunCommand(command.into()));
        value
    }
    fn on_click_suggest_command(self, command: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::SuggestCommand(command.into()));
        value
    }
    fn on_click_change_page(self, page: impl Into<i32>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::ChangePage(page.into()));
        value
    }
    fn on_click_copy_to_clipboard(self, text: impl Into<Cow<'static, str>>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::CopyToClipboard(text.into()));
        value
    }
    fn clear_click_event(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.click_event = None;
        value
    }

    fn on_hover_show_text(self, text: impl IntoText<'static>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.hover_event = Some(HoverEvent::ShowText(text.into_text()));
        value
    }

    fn clear_hover_event(self) -> Self::ReturnType {
        let mut value = self.into_text();
        value.hover_event = None;
        value
    }

    fn add_child(self, text: impl IntoText<'static>) -> Self::ReturnType {
        let mut value = self.into_text();
        value.extra.push(text.into_text());
        value
    }
}
