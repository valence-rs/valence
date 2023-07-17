//! Provides the [`IntoText`] trait and implementations.

use std::borrow::Cow;

use super::Text;

/// Trait for any data that can be converted to a [`Text`] object.
pub trait IntoText: Sized {
    /// Converts to a [`Text`] object.
    fn into_text(self) -> Text;
}

impl<'a, T: IntoText + Clone> IntoText for &'a T {
    fn into_text(self) -> Text {
        (*self).clone().into_text()
    }
}

impl IntoText for Text {
    fn into_text(self) -> Text {
        self
    }
}

impl IntoText for String {
    fn into_text(self) -> Text {
        Text::text(self)
    }
}

impl IntoText for Cow<'static, str> {
    fn into_text(self) -> Text {
        Text::text(self)
    }
}

impl IntoText for &'static str {
    fn into_text(self) -> Text {
        Text::text(self)
    }
}

macro_rules! impl_primitives {
    ($($primitive:ty),+) => {
        $(
            impl IntoText for $primitive {
                fn into_text(self) -> Text {
                    Text::text(self.to_string())
                }
            }
        )+
    };
}

impl_primitives! {char, bool, f32, f64, isize, usize, i8, i16, i32, i64, i128, u8, u16, u32, u64, u128}
