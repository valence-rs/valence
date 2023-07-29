//! [`Color`] and related data structures.

use std::fmt;
use std::hash::Hash;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Text color
#[derive(Default, Debug, PartialOrd, Eq, Ord, Clone, Copy)]
pub enum Color {
    /// The default color for the text will be used, which varies by context
    /// (in some cases, it's white; in others, it's black; in still others, it
    /// is a shade of gray that isn't normally used on text).
    #[default]
    Reset,
    /// RGB Color
    Rgb(RgbColor),
    /// One of the 16 named Minecraft colors
    Named(NamedColor),
}

/// RGB Color
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RgbColor {
    /// Red channel
    pub r: u8,
    /// Green channel
    pub g: u8,
    /// Blue channel
    pub b: u8,
}

/// Named Minecraft color
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum NamedColor {
    /// Hex digit: `0`, name: `black`
    Black = 0,
    /// Hex digit: `1`, name: `dark_blue`
    DarkBlue,
    /// Hex digit: `2`, name: `dark_green`
    DarkGreen,
    /// Hex digit: `3`, name: `dark_aqua`
    DarkAqua,
    /// Hex digit: `4`, name: `dark_red`
    DarkRed,
    /// Hex digit: `5`, name: `dark_purple`
    DarkPurple,
    /// Hex digit: `6`, name: `gold`
    Gold,
    /// Hex digit: `7`, name: `gray`
    Gray,
    /// Hex digit: `8`, name: `dark_gray`
    DarkGray,
    /// Hex digit: `9`, name: `blue`
    Blue,
    /// Hex digit: `a`, name: `green`
    Green,
    /// Hex digit: `b`, name: `aqua`
    Aqua,
    /// Hex digit: `c`, name: `red`
    Red,
    /// Hex digit: `d`, name: `light_purple`
    LightPurple,
    /// Hex digit: `e`, name: `yellow`
    Yellow,
    /// Hex digit: `f`, name: `white`
    White,
}

/// Color parsing error
#[derive(Debug, Error, PartialEq, PartialOrd, Clone, Copy, Hash, Eq, Ord)]
#[error("invalid color name or hex code")]
pub struct ColorError;

impl Color {
    pub const RESET: Self = Self::Reset;
    pub const AQUA: Self = Self::Named(NamedColor::Aqua);
    pub const BLACK: Self = Self::Named(NamedColor::Black);
    pub const BLUE: Self = Self::Named(NamedColor::Blue);
    pub const DARK_AQUA: Self = Self::Named(NamedColor::DarkAqua);
    pub const DARK_BLUE: Self = Self::Named(NamedColor::DarkBlue);
    pub const DARK_GRAY: Self = Self::Named(NamedColor::DarkGray);
    pub const DARK_GREEN: Self = Self::Named(NamedColor::DarkGreen);
    pub const DARK_PURPLE: Self = Self::Named(NamedColor::DarkPurple);
    pub const DARK_RED: Self = Self::Named(NamedColor::DarkRed);
    pub const GOLD: Self = Self::Named(NamedColor::Gold);
    pub const GRAY: Self = Self::Named(NamedColor::Gray);
    pub const GREEN: Self = Self::Named(NamedColor::Green);
    pub const LIGHT_PURPLE: Self = Self::Named(NamedColor::LightPurple);
    pub const RED: Self = Self::Named(NamedColor::Red);
    pub const WHITE: Self = Self::Named(NamedColor::White);
    pub const YELLOW: Self = Self::Named(NamedColor::Yellow);

    /// Constructs a new RGB color
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb(RgbColor::new(r, g, b))
    }
}

impl RgbColor {
    /// Constructs a new color from red, green, and blue components.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
    /// Converts the RGB color to the closest [`NamedColor`] equivalent (lossy).
    pub fn to_named_lossy(self) -> NamedColor {
        // calculates the squared distance between 2 colors
        fn squared_distance(c1: RgbColor, c2: RgbColor) -> i32 {
            (c1.r as i32 - c2.r as i32).pow(2)
                + (c1.g as i32 - c2.g as i32).pow(2)
                + (c1.b as i32 - c2.b as i32).pow(2)
        }

        [
            NamedColor::Aqua,
            NamedColor::Black,
            NamedColor::Blue,
            NamedColor::DarkAqua,
            NamedColor::DarkBlue,
            NamedColor::DarkGray,
            NamedColor::DarkGreen,
            NamedColor::DarkPurple,
            NamedColor::DarkRed,
            NamedColor::Gold,
            NamedColor::Gray,
            NamedColor::Green,
            NamedColor::LightPurple,
            NamedColor::Red,
            NamedColor::White,
            NamedColor::Yellow,
        ]
        .into_iter()
        .min_by_key(|&named| squared_distance(named.into(), self))
        .unwrap()
    }
}

impl NamedColor {
    /// Returns the corresponding hex digit of the color.
    pub const fn hex_digit(self) -> char {
        b"0123456789abcdef"[self as usize] as char
    }
    /// Returns the identifier of the color.
    pub const fn name(self) -> &'static str {
        [
            "black",
            "dark_blue",
            "dark_green",
            "dark_aqua",
            "dark_red",
            "dark_purple",
            "gold",
            "gray",
            "dark_gray",
            "blue",
            "green",
            "aqua",
            "red",
            "light_purple",
            "yellow",
            "white",
        ][self as usize]
    }
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Self::Reset, Self::Reset) => true,
            (Self::Rgb(rgb1), Self::Rgb(rgb2)) => rgb1 == rgb2,
            (Self::Named(normal1), Self::Named(normal2)) => normal1 == normal2,
            (Self::Rgb(rgb), Self::Named(normal)) | (Self::Named(normal), Self::Rgb(rgb)) => {
                rgb == RgbColor::from(normal)
            }
            (Self::Reset, _) | (_, Self::Reset) => false,
        }
    }
}

impl Hash for Color {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Reset => state.write_u8(0),
            Self::Rgb(rgb) => {
                state.write_u8(1);
                rgb.hash(state);
            }
            Self::Named(normal) => {
                state.write_u8(1);
                RgbColor::from(*normal).hash(state);
            }
        }
    }
}

impl From<NamedColor> for RgbColor {
    fn from(value: NamedColor) -> Self {
        match value {
            NamedColor::Aqua => Self::new(85, 255, 255),
            NamedColor::Black => Self::new(0, 0, 0),
            NamedColor::Blue => Self::new(85, 85, 255),
            NamedColor::DarkAqua => Self::new(0, 170, 170),
            NamedColor::DarkBlue => Self::new(0, 0, 170),
            NamedColor::DarkGray => Self::new(85, 85, 85),
            NamedColor::DarkGreen => Self::new(0, 170, 0),
            NamedColor::DarkPurple => Self::new(170, 0, 170),
            NamedColor::DarkRed => Self::new(170, 0, 0),
            NamedColor::Gold => Self::new(255, 170, 0),
            NamedColor::Gray => Self::new(170, 170, 170),
            NamedColor::Green => Self::new(85, 255, 85),
            NamedColor::LightPurple => Self::new(255, 85, 255),
            NamedColor::Red => Self::new(255, 85, 85),
            NamedColor::White => Self::new(255, 255, 255),
            NamedColor::Yellow => Self::new(255, 255, 85),
        }
    }
}

impl From<RgbColor> for Color {
    fn from(value: RgbColor) -> Self {
        Self::Rgb(value)
    }
}

impl From<NamedColor> for Color {
    fn from(value: NamedColor) -> Self {
        Self::Named(value)
    }
}

impl TryFrom<&str> for Color {
    type Error = ColorError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.starts_with('#') {
            return Ok(Self::Rgb(RgbColor::try_from(value)?));
        }

        if value == "reset" {
            return Ok(Self::Reset);
        }

        Ok(Self::Named(NamedColor::try_from(value)?))
    }
}

impl TryFrom<&str> for NamedColor {
    type Error = ColorError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "black" => Ok(NamedColor::Black),
            "dark_blue" => Ok(NamedColor::DarkBlue),
            "dark_green" => Ok(NamedColor::DarkGreen),
            "dark_aqua" => Ok(NamedColor::DarkAqua),
            "dark_red" => Ok(NamedColor::DarkRed),
            "dark_purple" => Ok(NamedColor::DarkPurple),
            "gold" => Ok(NamedColor::Gold),
            "gray" => Ok(NamedColor::Gray),
            "dark_gray" => Ok(NamedColor::DarkGray),
            "blue" => Ok(NamedColor::Blue),
            "green" => Ok(NamedColor::Green),
            "aqua" => Ok(NamedColor::Aqua),
            "red" => Ok(NamedColor::Red),
            "light_purple" => Ok(NamedColor::LightPurple),
            "yellow" => Ok(NamedColor::Yellow),
            "white" => Ok(NamedColor::White),
            _ => Err(ColorError),
        }
    }
}

impl TryFrom<&str> for RgbColor {
    type Error = ColorError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let to_num = |d| match d {
            b'0'..=b'9' => Ok(d - b'0'),
            b'a'..=b'f' => Ok(d - b'a' + 0xa),
            b'A'..=b'F' => Ok(d - b'A' + 0xa),
            _ => Err(ColorError),
        };

        if let &[b'#', r0, r1, g0, g1, b0, b1] = value.as_bytes() {
            Ok(RgbColor {
                r: to_num(r0)? << 4 | to_num(r1)?,
                g: to_num(g0)? << 4 | to_num(g1)?,
                b: to_num(b0)? << 4 | to_num(b1)?,
            })
        } else {
            Err(ColorError)
        }
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        format!("{}", self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(ColorVisitor)
    }
}

struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a hex color (#rrggbb), a normal color or 'reset'")
    }

    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
        Color::try_from(s).map_err(|_| E::custom("invalid color"))
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Color::Reset => write!(f, "reset"),
            Color::Rgb(rgb) => rgb.fmt(f),
            Color::Named(normal) => normal.fmt(f),
        }
    }
}

impl fmt::Display for RgbColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl fmt::Display for NamedColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(
            Color::try_from("#aBcDeF"),
            Ok(RgbColor::new(0xab, 0xcd, 0xef).into())
        );
        assert_eq!(
            Color::try_from("#fFfFfF"),
            Ok(RgbColor::new(255, 255, 255).into())
        );
        assert_eq!(Color::try_from("#000000"), Ok(NamedColor::Black.into()));
        assert_eq!(Color::try_from("red"), Ok(NamedColor::Red.into()));
        assert_eq!(Color::try_from("blue"), Ok(NamedColor::Blue.into()));
        assert!(Color::try_from("#ffTf00").is_err());
        assert!(Color::try_from("#ffš00").is_err());
        assert!(Color::try_from("#00000000").is_err());
        assert!(Color::try_from("#").is_err());
    }
}
