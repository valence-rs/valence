//! Provides the [`IntoText`] trait and implementations.

use std::borrow::Cow;

use super::Text;

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

impl<'a> IntoText<'a> for Cow<'a, Text> {
    fn into_cow_text(self) -> Cow<'a, Text> {
        self
    }
}
impl<'a, 'b> IntoText<'a> for &'a Cow<'b, Text> {
    fn into_cow_text(self) -> Cow<'a, Text> {
        self.clone()
    }
}

impl IntoText<'static> for String {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(self))
    }
}
impl<'a> IntoText<'static> for &'a String {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(self.clone()))
    }
}

impl IntoText<'static> for Cow<'static, str> {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(self))
    }
}
impl<'a> IntoText<'static> for &'a Cow<'static, str> {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(self.clone()))
    }
}

impl IntoText<'static> for &'static str {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(self))
    }
}
impl<'a> IntoText<'static> for &'a &'static str {
    fn into_cow_text(self) -> Cow<'static, Text> {
        Cow::Owned(Text::text(*self))
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
            impl<'a> IntoText<'static> for &'a $primitive {
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
        assert!(!is_borrowed(&"this has to be owned too"));
        assert!(!is_borrowed("this should be owned"));
        assert!(!is_borrowed(465));
        assert!(!is_borrowed(false));
    }
}
