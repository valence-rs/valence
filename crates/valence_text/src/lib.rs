//! Formatted text.

use std::borrow::Cow;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::{fmt, ops};

use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize};
use uuid::Uuid;
use valence_ident::Ident;
use valence_nbt::Value;

pub mod color;
mod into_text;
#[cfg(test)]
mod tests;

pub use color::Color;
pub use into_text::IntoText;

/// Represents formatted text in Minecraft's JSON text format.
///
/// Text is used in various places such as chat, window titles,
/// disconnect messages, written books, signs, and more.
///
/// For more information, see the relevant [Minecraft Wiki article].
///
/// [Minecraft Wiki article]: https://minecraft.fandom.com/wiki/Raw_JSON_text_format
///
/// # Examples
///
/// With [`IntoText`] in scope, you can write the following:
/// ```
/// use valence_text::{Color, IntoText, Text};
///
/// let txt = "The text is ".into_text()
///     + "Red".color(Color::RED)
///     + ", "
///     + "Green".color(Color::GREEN)
///     + ", and also "
///     + "Blue".color(Color::BLUE)
///     + "! And maybe even "
///     + "Italic".italic()
///     + ".";
///
/// assert_eq!(
///     txt.to_string(),
///     r#"{"text":"The text is ","extra":[{"text":"Red","color":"red"},{"text":", "},{"text":"Green","color":"green"},{"text":", and also "},{"text":"Blue","color":"blue"},{"text":"! And maybe even "},{"text":"Italic","italic":true},{"text":"."}]}"#
/// );
/// ```
#[derive(Clone, PartialEq, Default, Serialize)]
#[serde(transparent)]
pub struct Text(Box<TextInner>);

/// Text data and formatting.
#[derive(Clone, PartialEq, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextInner {
    #[serde(flatten)]
    pub content: TextContent,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<Font>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlined: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obfuscated: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insertion: Option<Cow<'static, str>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub click_event: Option<ClickEvent>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_event: Option<HoverEvent>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Text>,
}

/// The text content of a Text object.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextContent {
    /// Normal text
    Text { text: Cow<'static, str> },
    /// A piece of text that will be translated on the client based on the
    /// client language. If no corresponding translation can be found, the
    /// identifier itself is used as the translated text.
    Translate {
        /// A translation identifier, corresponding to the identifiers found in
        /// loaded language files.
        translate: Cow<'static, str>,
        /// Optional list of text components to be inserted into slots in the
        /// translation text. Ignored if `translate` is not present.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        with: Vec<Text>,
    },
    /// Displays a score holder's current score in an objective.
    ScoreboardValue { score: ScoreboardValueContent },
    /// Displays the name of one or more entities found by a [`selector`].
    ///
    /// [`selector`]: https://minecraft.fandom.com/wiki/Target_selectors
    EntityNames {
        /// A string containing a [`selector`].
        ///
        /// [`selector`]: https://minecraft.fandom.com/wiki/Target_selectors
        selector: Cow<'static, str>,
        /// An optional custom separator used when the selector returns multiple
        /// entities. Defaults to the ", " text with gray color.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        separator: Option<Text>,
    },
    /// Displays the name of the button that is currently bound to a certain
    /// configurable control on the client.
    Keybind {
        /// A [`keybind identifier`], to be displayed as the name of the button
        /// that is currently bound to that action.
        ///
        /// [`keybind identifier`]: https://minecraft.fandom.com/wiki/Controls#Configurable_controls
        keybind: Cow<'static, str>,
    },
    /// Displays NBT values from block entities.
    BlockNbt {
        block: Cow<'static, str>,
        nbt: Cow<'static, str>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpret: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        separator: Option<Text>,
    },
    /// Displays NBT values from entities.
    EntityNbt {
        entity: Cow<'static, str>,
        nbt: Cow<'static, str>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpret: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        separator: Option<Text>,
    },
    /// Displays NBT values from command storage.
    StorageNbt {
        storage: Ident<Cow<'static, str>>,
        nbt: Cow<'static, str>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpret: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        separator: Option<Text>,
    },
}

/// Scoreboard value.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ScoreboardValueContent {
    /// The name of the score holder whose score should be displayed. This
    /// can be a [`selector`] or an explicit name.
    ///
    /// [`selector`]: https://minecraft.fandom.com/wiki/Target_selectors
    pub name: Cow<'static, str>,
    /// The internal name of the objective to display the player's score in.
    pub objective: Cow<'static, str>,
    /// If present, this value is displayed regardless of what the score
    /// would have been.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Cow<'static, str>>,
}

/// Action to take on click of the text.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "snake_case")]
pub enum ClickEvent {
    /// Opens an URL
    OpenUrl(Cow<'static, str>),
    /// Only usable by internal servers for security reasons.
    OpenFile(Cow<'static, str>),
    /// Sends a chat command. Doesn't actually have to be a command, can be a
    /// normal chat message.
    RunCommand(Cow<'static, str>),
    /// Replaces the contents of the chat box with the text, not necessarily a
    /// command.
    SuggestCommand(Cow<'static, str>),
    /// Only usable within written books. Changes the page of the book. Indexing
    /// starts at 1.
    ChangePage(i32),
    /// Copies the given text to clipboard
    CopyToClipboard(Cow<'static, str>),
}

/// Action to take when mouse-hovering on the text.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "contents", rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum HoverEvent {
    /// Displays a tooltip with the given text.
    ShowText(Text),
    /// Shows an item.
    ShowItem {
        /// Resource identifier of the item
        id: Ident<Cow<'static, str>>,
        /// Number of the items in the stack
        count: Option<i32>,
        /// NBT information about the item (sNBT format)
        tag: Cow<'static, str>,
    },
    /// Shows an entity.
    ShowEntity {
        /// The entity's UUID
        id: Uuid,
        /// Resource identifier of the entity
        #[serde(rename = "type")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<Ident<Cow<'static, str>>>,
        /// Optional custom name for the entity
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<Text>,
    },
}

/// The font of the text.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Font {
    /// The default font.
    #[serde(rename = "minecraft:default")]
    Default,
    /// Unicode font.
    #[serde(rename = "minecraft:uniform")]
    Uniform,
    /// Enchanting table font.
    #[serde(rename = "minecraft:alt")]
    Alt,
}

#[allow(clippy::self_named_constructors)]
impl Text {
    /// Constructs a new plain text object.
    pub fn text(plain: impl Into<Cow<'static, str>>) -> Self {
        Self(Box::new(TextInner {
            content: TextContent::Text { text: plain.into() },
            ..Default::default()
        }))
    }

    /// Create translated text based on the given translation key, with extra
    /// text components to be inserted into the slots of the translation text.
    pub fn translate(key: impl Into<Cow<'static, str>>, with: impl Into<Vec<Text>>) -> Self {
        Self(Box::new(TextInner {
            content: TextContent::Translate {
                translate: key.into(),
                with: with.into(),
            },
            ..Default::default()
        }))
    }

    /// Create a score from the scoreboard with an optional custom value.
    pub fn score(
        name: impl Into<Cow<'static, str>>,
        objective: impl Into<Cow<'static, str>>,
        value: Option<Cow<'static, str>>,
    ) -> Self {
        Self(Box::new(TextInner {
            content: TextContent::ScoreboardValue {
                score: ScoreboardValueContent {
                    name: name.into(),
                    objective: objective.into(),
                    value,
                },
            },
            ..Default::default()
        }))
    }

    /// Creates a text component for selecting entity names with an optional
    /// custom separator.
    pub fn selector(selector: impl Into<Cow<'static, str>>, separator: Option<Text>) -> Self {
        Self(Box::new(TextInner {
            content: TextContent::EntityNames {
                selector: selector.into(),
                separator,
            },
            ..Default::default()
        }))
    }

    /// Creates a text component for a keybind. The keybind should be a valid
    /// [`keybind identifier`].
    ///
    /// [`keybind identifier`]: https://minecraft.fandom.com/wiki/Controls#Configurable_controls
    pub fn keybind(keybind: impl Into<Cow<'static, str>>) -> Self {
        Self(Box::new(TextInner {
            content: TextContent::Keybind {
                keybind: keybind.into(),
            },
            ..Default::default()
        }))
    }

    /// Creates a text component for a block NBT tag.
    pub fn block_nbt(
        block: impl Into<Cow<'static, str>>,
        nbt: impl Into<Cow<'static, str>>,
        interpret: Option<bool>,
        separator: Option<Text>,
    ) -> Self {
        Self(Box::new(TextInner {
            content: TextContent::BlockNbt {
                block: block.into(),
                nbt: nbt.into(),
                interpret,
                separator,
            },
            ..Default::default()
        }))
    }

    /// Creates a text component for an entity NBT tag.
    pub fn entity_nbt(
        entity: impl Into<Cow<'static, str>>,
        nbt: impl Into<Cow<'static, str>>,
        interpret: Option<bool>,
        separator: Option<Text>,
    ) -> Self {
        Self(Box::new(TextInner {
            content: TextContent::EntityNbt {
                entity: entity.into(),
                nbt: nbt.into(),
                interpret,
                separator,
            },
            ..Default::default()
        }))
    }

    /// Creates a text component for a command storage NBT tag.
    pub fn storage_nbt(
        storage: impl Into<Ident<Cow<'static, str>>>,
        nbt: impl Into<Cow<'static, str>>,
        interpret: Option<bool>,
        separator: Option<Text>,
    ) -> Self {
        Self(Box::new(TextInner {
            content: TextContent::StorageNbt {
                storage: storage.into(),
                nbt: nbt.into(),
                interpret,
                separator,
            },
            ..Default::default()
        }))
    }

    /// Returns `true` if the text contains no characters. Returns `false`
    /// otherwise.
    pub fn is_empty(&self) -> bool {
        for extra in &self.0.extra {
            if !extra.is_empty() {
                return false;
            }
        }

        match &self.0.content {
            TextContent::Text { text } => text.is_empty(),
            TextContent::Translate { translate, .. } => translate.is_empty(),
            TextContent::ScoreboardValue { score } => {
                let ScoreboardValueContent {
                    name, objective, ..
                } = score;

                name.is_empty() || objective.is_empty()
            }
            TextContent::EntityNames { selector, .. } => selector.is_empty(),
            TextContent::Keybind { keybind } => keybind.is_empty(),
            TextContent::BlockNbt { nbt, .. } => nbt.is_empty(),
            TextContent::EntityNbt { nbt, .. } => nbt.is_empty(),
            TextContent::StorageNbt { nbt, .. } => nbt.is_empty(),
        }
    }

    /// Converts the [`Text`] object to a plain string with the [legacy formatting (`§` and format codes)](https://wiki.vg/Chat#Old_system)
    ///
    /// Removes everything that can't be represented with a `§` and a modifier.
    /// Any colors not on the [the legacy color list](https://wiki.vg/Chat#Colors) will be replaced with their closest equivalent.
    pub fn to_legacy_lossy(&self) -> String {
        // For keeping track of the currently active modifiers
        #[derive(Default, Clone)]
        struct Modifiers {
            obfuscated: Option<bool>,
            bold: Option<bool>,
            strikethrough: Option<bool>,
            underlined: Option<bool>,
            italic: Option<bool>,
            color: Option<Color>,
        }

        impl Modifiers {
            // Writes all active modifiers to a String as `§<mod>`
            fn write(&self, output: &mut String) {
                if let Some(color) = self.color {
                    let code = match color {
                        Color::Rgb(rgb) => rgb.to_named_lossy().hex_digit(),
                        Color::Named(normal) => normal.hex_digit(),
                        Color::Reset => return,
                    };

                    output.push('§');
                    output.push(code);
                }
                if let Some(true) = self.obfuscated {
                    output.push_str("§k");
                }
                if let Some(true) = self.bold {
                    output.push_str("§l");
                }
                if let Some(true) = self.strikethrough {
                    output.push_str("§m");
                }
                if let Some(true) = self.underlined {
                    output.push_str("§n");
                }
                if let Some(true) = self.italic {
                    output.push_str("§o");
                }
            }
            // Merges 2 Modifiers. The result is what you would get if you applied them both
            // sequentially.
            fn add(&self, other: &Self) -> Self {
                Self {
                    obfuscated: other.obfuscated.or(self.obfuscated),
                    bold: other.bold.or(self.bold),
                    strikethrough: other.strikethrough.or(self.strikethrough),
                    underlined: other.underlined.or(self.underlined),
                    italic: other.italic.or(self.italic),
                    color: other.color.or(self.color),
                }
            }
        }

        fn to_legacy_inner(this: &Text, result: &mut String, mods: &mut Modifiers) {
            let new_mods = Modifiers {
                obfuscated: this.0.obfuscated,
                bold: this.0.bold,
                strikethrough: this.0.strikethrough,
                underlined: this.0.underlined,
                italic: this.0.italic,
                color: this.0.color,
            };

            // If any modifiers were removed
            if [
                this.0.obfuscated,
                this.0.bold,
                this.0.strikethrough,
                this.0.underlined,
                this.0.italic,
            ]
            .iter()
            .any(|m| *m == Some(false))
                || this.0.color == Some(Color::Reset)
            {
                // Reset and print sum of old and new modifiers
                result.push_str("§r");
                mods.add(&new_mods).write(result);
            } else {
                // Print only new modifiers
                new_mods.write(result);
            }

            *mods = mods.add(&new_mods);

            if let TextContent::Text { text } = &this.0.content {
                result.push_str(text);
            }

            for child in &this.0.extra {
                to_legacy_inner(child, result, mods);
            }
        }

        let mut result = String::new();
        let mut mods = Modifiers::default();
        to_legacy_inner(self, &mut result, &mut mods);

        result
    }
}

impl Deref for Text {
    type Target = TextInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Text {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: IntoText<'static>> ops::Add<T> for Text {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        self.add_child(rhs)
    }
}

impl<T: IntoText<'static>> ops::AddAssign<T> for Text {
    fn add_assign(&mut self, rhs: T) {
        self.extra.push(rhs.into_text());
    }
}

impl<'a> From<Text> for Cow<'a, Text> {
    fn from(value: Text) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a Text> for Cow<'a, Text> {
    fn from(value: &'a Text) -> Self {
        Cow::Borrowed(value)
    }
}

impl FromStr for Text {
    type Err = serde_json::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Text::default())
        } else {
            serde_json::from_str(s)
        }
    }
}

impl From<Text> for String {
    fn from(value: Text) -> Self {
        format!("{value}")
    }
}

impl From<Text> for Value {
    fn from(value: Text) -> Self {
        Value::String(value.into())
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string = if f.alternate() {
            serde_json::to_string_pretty(self)
        } else {
            serde_json::to_string(self)
        }
        .map_err(|_| fmt::Error)?;

        f.write_str(&string)
    }
}

impl Default for TextContent {
    fn default() -> Self {
        Self::Text { text: "".into() }
    }
}

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TextVisitor;

        impl<'de> Visitor<'de> for TextVisitor {
            type Value = Text;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a text component data type")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(Text::text(v.to_string()))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Text::text(v.to_string()))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Text::text(v.to_string()))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(Text::text(v.to_string()))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Text::text(v.to_string()))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(Text::text(v))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let Some(mut res) = seq.next_element()? else {
                    return Ok(Text::default());
                };

                while let Some(child) = seq.next_element::<Text>()? {
                    res += child;
                }

                Ok(res)
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                use de::value::MapAccessDeserializer;

                Ok(Text(Box::new(TextInner::deserialize(
                    MapAccessDeserializer::new(map),
                )?)))
            }
        }

        deserializer.deserialize_any(TextVisitor)
    }
}
