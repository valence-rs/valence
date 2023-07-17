use std::fmt;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Text color
#[derive(Default, Debug, PartialOrd, Eq, Ord, Clone, Copy, Hash)]
pub enum Color {
    /// The default color for the text will be used, which varies by context
    /// (in some cases, it's white; in others, it's black; in still others, it
    /// is a shade of gray that isn't normally used on text).
    #[default]
    Reset,
    /// RGB Color
    Rgb(RgbColor),
    /// One of the 16 normal Minecraft colors
    Normal(NormalColor),
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

/// Normal Minecraft colors
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum NormalColor {
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

#[derive(Debug, Error, PartialEq, PartialOrd, Clone, Copy, Hash, Eq, Ord)]
#[error("invalid color name or hex code")]
pub struct ColorError;

impl Color {
    pub const RESET: Self = Self::Reset;
    pub const AQUA: Self = Self::Normal(NormalColor::Aqua);
    pub const BLACK: Self = Self::Normal(NormalColor::Black);
    pub const BLUE: Self = Self::Normal(NormalColor::Blue);
    pub const DARK_AQUA: Self = Self::Normal(NormalColor::DarkAqua);
    pub const DARK_BLUE: Self = Self::Normal(NormalColor::DarkBlue);
    pub const DARK_GRAY: Self = Self::Normal(NormalColor::DarkGray);
    pub const DARK_GREEN: Self = Self::Normal(NormalColor::DarkGreen);
    pub const DARK_PURPLE: Self = Self::Normal(NormalColor::DarkPurple);
    pub const DARK_RED: Self = Self::Normal(NormalColor::DarkRed);
    pub const GOLD: Self = Self::Normal(NormalColor::Gold);
    pub const GRAY: Self = Self::Normal(NormalColor::Gray);
    pub const GREEN: Self = Self::Normal(NormalColor::Green);
    pub const LIGHT_PURPLE: Self = Self::Normal(NormalColor::LightPurple);
    pub const RED: Self = Self::Normal(NormalColor::Red);
    pub const WHITE: Self = Self::Normal(NormalColor::White);
    pub const YELLOW: Self = Self::Normal(NormalColor::Yellow);

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
}

impl NormalColor {
    pub const fn as_hex_digit(self) -> char {
        b"0123456789abcdef"[self as usize] as char
    }
    pub const fn as_name(self) -> &'static str {
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
            (Self::Normal(normal1), Self::Normal(normal2)) => normal1 == normal2,
            (Self::Rgb(rgb), Self::Normal(normal)) | (Self::Normal(normal), Self::Rgb(rgb)) => {
                rgb == RgbColor::from(normal)
            }
            (Self::Reset, _) | (_, Self::Reset) => false,
        }
    }
}

impl From<NormalColor> for RgbColor {
    fn from(value: NormalColor) -> Self {
        match value {
            NormalColor::Aqua => Self::new(85, 255, 255),
            NormalColor::Black => Self::new(0, 0, 0),
            NormalColor::Blue => Self::new(85, 85, 255),
            NormalColor::DarkAqua => Self::new(0, 170, 170),
            NormalColor::DarkBlue => Self::new(0, 0, 170),
            NormalColor::DarkGray => Self::new(85, 85, 85),
            NormalColor::DarkGreen => Self::new(0, 170, 0),
            NormalColor::DarkPurple => Self::new(170, 0, 170),
            NormalColor::DarkRed => Self::new(170, 0, 0),
            NormalColor::Gold => Self::new(255, 170, 0),
            NormalColor::Gray => Self::new(170, 170, 170),
            NormalColor::Green => Self::new(85, 255, 85),
            NormalColor::LightPurple => Self::new(255, 85, 255),
            NormalColor::Red => Self::new(255, 85, 85),
            NormalColor::White => Self::new(255, 255, 255),
            NormalColor::Yellow => Self::new(255, 255, 85),
        }
    }
}

impl From<RgbColor> for NormalColor {
    fn from(value: RgbColor) -> Self {
        // calculates the squared distance between 2 colors
        fn squared_distance(c1: RgbColor, c2: RgbColor) -> i32 {
            (c1.r as i32 - c2.r as i32).pow(2)
                + (c1.g as i32 - c2.g as i32).pow(2)
                + (c1.b as i32 - c2.b as i32).pow(2)
        }

        [
            NormalColor::Aqua,
            NormalColor::Black,
            NormalColor::Blue,
            NormalColor::DarkAqua,
            NormalColor::DarkBlue,
            NormalColor::DarkGray,
            NormalColor::DarkGreen,
            NormalColor::DarkPurple,
            NormalColor::DarkRed,
            NormalColor::Gold,
            NormalColor::Gray,
            NormalColor::Green,
            NormalColor::LightPurple,
            NormalColor::Red,
            NormalColor::White,
            NormalColor::Yellow,
        ]
        .into_iter()
        .min_by_key(|&normal| squared_distance(normal.into(), value))
        .unwrap()
    }
}

impl From<RgbColor> for Color {
    fn from(value: RgbColor) -> Self {
        Self::Rgb(value)
    }
}

impl From<NormalColor> for Color {
    fn from(value: NormalColor) -> Self {
        Self::Normal(value)
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

        Ok(Self::Normal(NormalColor::try_from(value)?))
    }
}

impl TryFrom<&str> for NormalColor {
    type Error = ColorError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "black" => Ok(NormalColor::Black),
            "dark_blue" => Ok(NormalColor::DarkBlue),
            "dark_green" => Ok(NormalColor::DarkGreen),
            "dark_aqua" => Ok(NormalColor::DarkAqua),
            "dark_red" => Ok(NormalColor::DarkRed),
            "dark_purple" => Ok(NormalColor::DarkPurple),
            "gold" => Ok(NormalColor::Gold),
            "gray" => Ok(NormalColor::Gray),
            "dark_gray" => Ok(NormalColor::DarkGray),
            "blue" => Ok(NormalColor::Blue),
            "green" => Ok(NormalColor::Green),
            "aqua" => Ok(NormalColor::Aqua),
            "red" => Ok(NormalColor::Red),
            "light_purple" => Ok(NormalColor::LightPurple),
            "yellow" => Ok(NormalColor::Yellow),
            "white" => Ok(NormalColor::White),
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
            Color::Normal(normal) => normal.fmt(f),
        }
    }
}

impl fmt::Display for RgbColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl fmt::Display for NormalColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_name())
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
        assert_eq!(Color::try_from("#000000"), Ok(NormalColor::Black.into()));
        assert_eq!(Color::try_from("red"), Ok(NormalColor::Red.into()));
        assert_eq!(Color::try_from("blue"), Ok(NormalColor::Blue.into()));
        assert!(Color::try_from("#ffTf00").is_err());
        assert!(Color::try_from("#ffš00").is_err());
        assert!(Color::try_from("#00000000").is_err());
        assert!(Color::try_from("#").is_err());
    }
}
