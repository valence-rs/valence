//! Provides the [`IntoText`] trait and implementations.

use std::borrow::Cow;

use super::{ClickEvent, Color, Font, HoverEvent, Text};

/// Trait for any data that can be converted to a [`Text`] object.
///
/// Also conveniently provides many useful methods for modifying a [`Text`]
/// object.
///
/// # Usage
///
/// ```
/// # use valence_text::{IntoText, color::NamedColor};
/// let mut my_text = "".into_text();
/// my_text = my_text.color(NamedColor::Red).bold();
/// my_text = my_text.add_child("CRABBBBB".obfuscated());
pub trait IntoText<'a>: Sized {
    /// Converts to a [`Text`] object, either owned or borrowed.
    fn into_cow_text(self) -> Cow<'a, Text>;

    /// Converts to an owned [`Text`] object.
    fn into_text(self) -> Text {
        self.into_cow_text().into_owned()
    }

    /// Sets the color of the text.
    fn color(self, color: impl Into<Color>) -> Text {
        let mut value = self.into_text();
        value.color = Some(color.into());
        value
    }
    /// Clears the color of the text. Color of parent [`Text`] object will be
    /// used.
    fn clear_color(self) -> Text {
        let mut value = self.into_text();
        value.color = None;
        value
    }

    /// Sets the font of the text.
    fn font(self, font: Font) -> Text {
        let mut value = self.into_text();
        value.font = Some(font);
        value
    }
    /// Clears the font of the text. Font of parent [`Text`] object will be
    /// used.
    fn clear_font(self) -> Text {
        let mut value = self.into_text();
        value.font = None;
        value
    }

    /// Makes the text bold.
    fn bold(self) -> Text {
        let mut value = self.into_text();
        value.bold = Some(true);
        value
    }
    /// Makes the text not bold.
    fn not_bold(self) -> Text {
        let mut value = self.into_text();
        value.bold = Some(false);
        value
    }
    /// Clears the `bold` property of the text. Property of the parent [`Text`]
    /// object will be used.
    fn clear_bold(self) -> Text {
        let mut value = self.into_text();
        value.bold = None;
        value
    }

    /// Makes the text italic.
    fn italic(self) -> Text {
        let mut value = self.into_text();
        value.italic = Some(true);
        value
    }
    /// Makes the text not italic.
    fn not_italic(self) -> Text {
        let mut value = self.into_text();
        value.italic = Some(false);
        value
    }
    /// Clears the `italic` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_italic(self) -> Text {
        let mut value = self.into_text();
        value.italic = None;
        value
    }

    /// Makes the text underlined.
    fn underlined(self) -> Text {
        let mut value = self.into_text();
        value.underlined = Some(true);
        value
    }
    /// Makes the text not underlined.
    fn not_underlined(self) -> Text {
        let mut value = self.into_text();
        value.underlined = Some(false);
        value
    }
    /// Clears the `underlined` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_underlined(self) -> Text {
        let mut value = self.into_text();
        value.underlined = None;
        value
    }

    /// Adds a strikethrough effect to the text.
    fn strikethrough(self) -> Text {
        let mut value = self.into_text();
        value.strikethrough = Some(true);
        value
    }
    /// Removes the strikethrough effect from the text.
    fn not_strikethrough(self) -> Text {
        let mut value = self.into_text();
        value.strikethrough = Some(false);
        value
    }
    /// Clears the `strikethrough` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_strikethrough(self) -> Text {
        let mut value = self.into_text();
        value.strikethrough = None;
        value
    }

    /// Makes the text obfuscated.
    fn obfuscated(self) -> Text {
        let mut value = self.into_text();
        value.obfuscated = Some(true);
        value
    }
    /// Makes the text not obfuscated.
    fn not_obfuscated(self) -> Text {
        let mut value = self.into_text();
        value.obfuscated = Some(false);
        value
    }
    /// Clears the `obfuscated` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_obfuscated(self) -> Text {
        let mut value = self.into_text();
        value.obfuscated = None;
        value
    }

    /// Adds an `insertion` property to the text. When shift-clicked, the given
    /// text will be inserted into chat box for the client.
    fn insertion(self, insertion: impl Into<Cow<'static, str>>) -> Text {
        let mut value = self.into_text();
        value.insertion = Some(insertion.into());
        value
    }
    /// Clears the `insertion` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_insertion(self) -> Text {
        let mut value = self.into_text();
        value.insertion = None;
        value
    }

    /// On click, opens the given URL. Has to be `http` or `https` protocol.
    fn on_click_open_url(self, url: impl Into<Cow<'static, str>>) -> Text {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::OpenUrl(url.into()));
        value
    }
    /// On click, sends a command. Doesn't actually have to be a command, can be
    /// a simple chat message.
    fn on_click_run_command(self, command: impl Into<Cow<'static, str>>) -> Text {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::RunCommand(command.into()));
        value
    }
    /// On click, copies the given text to the chat box.
    fn on_click_suggest_command(self, command: impl Into<Cow<'static, str>>) -> Text {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::SuggestCommand(command.into()));
        value
    }
    /// On click, turns the page of the opened book to the given number.
    /// Indexing starts at `1`.
    fn on_click_change_page(self, page: impl Into<i32>) -> Text {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::ChangePage(page.into()));
        value
    }
    /// On click, copies the given text to clipboard.
    fn on_click_copy_to_clipboard(self, text: impl Into<Cow<'static, str>>) -> Text {
        let mut value = self.into_text();
        value.click_event = Some(ClickEvent::CopyToClipboard(text.into()));
        value
    }
    /// Clears the `click_event` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_click_event(self) -> Text {
        let mut value = self.into_text();
        value.click_event = None;
        value
    }

    /// On mouse hover, shows the given text in a tooltip.
    fn on_hover_show_text(self, text: impl IntoText<'static>) -> Text {
        let mut value = self.into_text();
        value.hover_event = Some(HoverEvent::ShowText(text.into_text()));
        value
    }
    /// Clears the `hover_event` property of the text. Property of the parent
    /// [`Text`] object will be used.
    fn clear_hover_event(self) -> Text {
        let mut value = self.into_text();
        value.hover_event = None;
        value
    }

    /// Adds a child [`Text`] object.
    fn add_child(self, text: impl IntoText<'static>) -> Text {
        let mut value = self.into_text();
        value.extra.push(text.into_text());
        value
    }
}

impl<'a> IntoText<'a> for Text {
    fn into_cow_text(self) -> Cow<'a, Text> {
        Cow::Owned(self)
    }
}
impl<'a> IntoText<'a> for &'a Text {
    fn into_cow_text(self) -> Cow<'a, Text> {
        Cow::Borrowed(self)
    }
}
impl<'a> From<&'a Text> for Text {
    fn from(value: &'a Text) -> Self {
        value.clone()
    }
}

impl<'a> IntoText<'a> for Cow<'a, Text> {
    fn into_cow_text(self) -> Cow<'a, Text> {
        self
    }
}
impl<'a> From<Cow<'a, Text>> for Text {
    fn from(value: Cow<'a, Text>) -> Self {
        value.into_owned()
    }
}
impl<'a, 'b> IntoText<'a> for &'a Cow<'b, Text> {
    fn into_cow_text(self) -> Cow<'a, Text> {
        self.clone()
    }
}
impl<'a, 'b> From<&'a Cow<'b, Text>> for Text {
    fn from(value: &'a Cow<'b, Text>) -> Self {
        value.clone().into_owned()
    }
}

impl<'a> IntoText<'a> for String {
    fn into_cow_text(self) -> Cow<'a, Text> {
        Cow::Owned(Text::text(self))
    }
}
impl From<String> for Text {
    fn from(value: String) -> Self {
        value.into_text()
    }
}
impl<'a, 'b> IntoText<'b> for &'a String {
    fn into_cow_text(self) -> Cow<'b, Text> {
        Cow::Owned(Text::text(self.clone()))
    }
}
impl<'a> From<&'a String> for Text {
    fn from(value: &'a String) -> Self {
        value.into_text()
    }
}

impl<'a> IntoText<'a> for Cow<'static, str> {
    fn into_cow_text(self) -> Cow<'a, Text> {
        Cow::Owned(Text::text(self))
    }
}
impl From<Cow<'static, str>> for Text {
    fn from(value: Cow<'static, str>) -> Self {
        value.into_text()
    }
}
impl<'a> IntoText<'static> for &'a Cow<'static, str> {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(self.clone()))
    }
}
impl<'a> From<&'a Cow<'static, str>> for Text {
    fn from(value: &'a Cow<'static, str>) -> Self {
        value.into_text()
    }
}

impl<'a> IntoText<'a> for &'static str {
    fn into_cow_text(self) -> Cow<'a, Text> {
        Cow::Owned(Text::text(self))
    }
}
impl From<&'static str> for Text {
    fn from(value: &'static str) -> Self {
        value.into_text()
    }
}

impl<'a, 'b, T: IntoText<'a>, const N: usize> IntoText<'b> for [T; N] {
    fn into_cow_text(self) -> Cow<'b, Text> {
        let mut txt = Text::text("");

        for child in self {
            txt = txt.add_child(child.into_cow_text().into_owned());
        }

        Cow::Owned(txt)
    }
}

impl<'a, 'b, 'c, T: IntoText<'a> + Clone, const N: usize> IntoText<'c> for &'b [T; N] {
    fn into_cow_text(self) -> Cow<'c, Text> {
        let mut txt = Text::text("");

        for child in self {
            txt = txt.add_child(child.clone().into_cow_text().into_owned());
        }

        Cow::Owned(txt)
    }
}

macro_rules! impl_primitives {
    ($($primitive:ty),+) => {
        $(
            impl<'a> IntoText<'a> for $primitive {
                fn into_cow_text(self) -> Cow<'a, Text> {
                    Cow::Owned(Text::text(self.to_string()))
                }
            }
        )+
    };
}
impl_primitives! {char, bool, f32, f64, isize, usize, i8, i16, i32, i64, i128, u8, u16, u32, u64, u128}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::needless_borrows_for_generic_args)]
    fn intotext_trait() {
        fn is_borrowed<'a>(value: impl IntoText<'a>) -> bool {
            matches!(value.into_cow_text(), Cow::Borrowed(..))
        }

        assert!(is_borrowed(&"this should be borrowed".into_text()));
        assert!(is_borrowed(&"this should be borrowed too".bold()));
        assert!(!is_borrowed("this should be owned?".bold()));
        assert!(!is_borrowed("this should be owned"));
        assert!(!is_borrowed(465));
        assert!(!is_borrowed(false));
    }
}
