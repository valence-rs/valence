//! Provides the [`IntoText`] trait and implementations.

use std::borrow::Cow;

use super::{Text, TextFormat};

/// Trait for any data that can be converted to a [`Text`] object.
pub trait IntoText<'a>: Sized {
    /// Converts to a [`Text`] object, either owned or borrowed.
    fn into_cow_text(self) -> Cow<'a, Text>;
}

impl IntoText<'static> for Text {
    fn into_cow_text(self) -> Cow<'static, Text> {
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

impl IntoText<'static> for String {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(self))
    }
}
impl From<String> for Text {
    fn from(value: String) -> Self {
        value.into_text()
    }
}
impl<'a> IntoText<'static> for &'a String {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(self.clone()))
    }
}
impl<'a> From<&'a String> for Text {
    fn from(value: &'a String) -> Self {
        value.into_text()
    }
}

impl IntoText<'static> for Cow<'static, str> {
    fn into_cow_text(self) -> Cow<'static, Text> {
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

impl IntoText<'static> for &'static str {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(self))
    }
}
impl From<&'static str> for Text {
    fn from(value: &'static str) -> Self {
        value.into_text()
    }
}

impl<'a, T: IntoText<'a>, const N: usize> IntoText<'static> for [T; N] {
    fn into_cow_text(self) -> Cow<'static, Text> {
        let mut txt = Text::text("");

        for child in self {
            txt = txt.add_child(child.into_cow_text().into_owned());
        }

        Cow::Owned(txt)
    }
}

impl<'a, 'b, T: IntoText<'a> + Clone, const N: usize> IntoText<'static> for &'b [T; N] {
    fn into_cow_text(self) -> Cow<'static, Text> {
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
            impl IntoText<'static> for $primitive {
                fn into_cow_text(self) -> Cow<'static, Text> {
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
    use crate::text::TextFormat;

    #[test]
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
