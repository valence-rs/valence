//! Formatted text.

use std::borrow::Cow;
use std::io::Write;
use std::{fmt, ops};

use anyhow::Context;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;
use valence_nbt::Value;

use crate::{Decode, Encode, Ident, Result};

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
/// With [`TextFormat`] in scope, you can write the following:
/// ```
/// use valence_protocol::text::{Color, Text, TextFormat};
///
/// let txt = "The text is ".into_text()
///     + "Red".color(Color::RED)
///     + ", "
///     + "Green".color(Color::GREEN)
///     + ", and also "
///     + "Blue".color(Color::BLUE)
///     + "!\nAnd maybe even "
///     + "Italic".italic()
///     + ".";
///
/// assert_eq!(
///     txt.to_string(),
///     "The text is Red, Green, and also Blue!\nAnd maybe even Italic."
/// );
/// ```
#[derive(Clone, PartialEq, Default, Serialize)]
#[serde(transparent)]
pub struct Text(Box<TextInner>);

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
                    return Ok(Text::default())
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

#[derive(Clone, PartialEq, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextInner {
    #[serde(flatten)]
    content: TextContent,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<Color>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    font: Option<Cow<'static, str>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    bold: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    italic: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    underlined: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    strikethrough: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    obfuscated: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    insertion: Option<Cow<'static, str>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    click_event: Option<ClickEvent>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    hover_event: Option<HoverEvent>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    extra: Vec<Text>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum TextContent {
    Text {
        text: Cow<'static, str>,
    },
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
    ScoreboardValue {
        score: ScoreboardValueContent,
    },
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
        storage: Ident<String>,
        nbt: Cow<'static, str>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpret: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        separator: Option<Text>,
    },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
struct ScoreboardValueContent {
    /// The name of the score holder whose score should be displayed. This
    /// can be a [`selector`] or an explicit name.
    ///
    /// [`selector`]: https://minecraft.fandom.com/wiki/Target_selectors
    name: Cow<'static, str>,
    /// The internal name of the objective to display the player's score in.
    objective: Cow<'static, str>,
    /// If present, this value is displayed regardless of what the score
    /// would have been.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<Cow<'static, str>>,
}

/// Text color
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Color {
    /// Red channel
    pub r: u8,
    /// Green channel
    pub g: u8,
    /// Blue channel
    pub b: u8,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "snake_case")]
enum ClickEvent {
    OpenUrl(Cow<'static, str>),
    /// Only usable by internal servers for security reasons.
    OpenFile(Cow<'static, str>),
    RunCommand(Cow<'static, str>),
    SuggestCommand(Cow<'static, str>),
    ChangePage(i32),
    CopyToClipboard(Cow<'static, str>),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "contents", rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
enum HoverEvent {
    ShowText(Text),
    ShowItem {
        id: Ident<String>,
        count: Option<i32>,
        // TODO: tag
    },
    ShowEntity {
        name: Text,
        #[serde(rename = "type")]
        kind: Ident<String>,
        id: Uuid,
    },
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
        storage: impl Into<Ident<String>>,
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

    /// Writes the string representation of this text object to the provided
    /// writer.
    pub fn write_string(&self, mut w: impl fmt::Write) -> fmt::Result {
        fn write_string_inner(this: &Text, w: &mut impl fmt::Write) -> fmt::Result {
            match &this.0.content {
                TextContent::Text { text } => w.write_str(text.as_ref())?,
                TextContent::Translate { translate, with } => {
                    w.write_str(translate.as_ref())?;

                    if !with.is_empty() {
                        w.write_char('[')?;
                        for (i, slot) in with.iter().enumerate() {
                            if i > 0 {
                                w.write_str(", ")?;
                            }
                            w.write_char(char::from_digit((i + 1) as u32, 10).unwrap_or('?'))?;
                            w.write_char('=')?;
                            write_string_inner(slot, w)?;
                        }
                        w.write_char(']')?;
                    }
                }
                TextContent::ScoreboardValue { score } => {
                    let ScoreboardValueContent {
                        name,
                        objective,
                        value,
                    } = score;

                    write!(w, "scoreboard_value[name={name}, objective={objective}")?;

                    if let Some(value) = value {
                        if !value.is_empty() {
                            w.write_str(", value=")?;
                            w.write_str(value)?;
                        }
                    }

                    w.write_char(']')?;
                }
                TextContent::EntityNames {
                    selector,
                    separator,
                } => {
                    write!(w, "entity_names[selector={selector}")?;

                    if let Some(separator) = separator {
                        if !separator.is_empty() {
                            w.write_str(", separator={separator}")?;
                        }
                    }

                    w.write_char(']')?;
                }
                TextContent::Keybind { keybind } => write!(w, "keybind[{keybind}]")?,
                TextContent::BlockNbt {
                    block,
                    nbt,
                    interpret,
                    separator,
                } => {
                    write!(w, "block_nbt[nbt={nbt}")?;

                    if let Some(interpret) = interpret {
                        write!(w, ", interpret={interpret}")?;
                    }

                    if let Some(separator) = separator {
                        if !separator.is_empty() {
                            write!(w, "separator={separator}")?;
                        }
                    }

                    write!(w, "block={block}")?;

                    w.write_char(']')?;
                }
                TextContent::EntityNbt {
                    entity,
                    nbt,
                    interpret,
                    separator,
                } => {
                    write!(w, "entity_nbt[nbt={nbt}")?;

                    if let Some(interpret) = interpret {
                        write!(w, ", interpret={interpret}")?;
                    }

                    if let Some(separator) = separator {
                        if !separator.is_empty() {
                            write!(w, "separator={separator}")?;
                        }
                    }

                    write!(w, ", entity={entity}")?;

                    w.write_char(']')?;
                }
                TextContent::StorageNbt {
                    storage,
                    nbt,
                    interpret,
                    separator,
                } => {
                    write!(w, "storage_nbt[nbt={nbt}")?;

                    if let Some(interpret) = interpret {
                        write!(w, ", interpret={interpret}")?;
                    }

                    if let Some(separator) = separator {
                        if !separator.is_empty() {
                            write!(w, "separator=")?;
                            write_string_inner(separator, w)?;
                        }
                    }

                    write!(w, ", storage={storage}")?;

                    w.write_char(']')?;
                }
            }

            for child in &this.0.extra {
                write_string_inner(child, w)?;
            }

            Ok(())
        }

        write_string_inner(self, &mut w)
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
}

/// Provides the methods necessary for working with [`Text`] objects.
///
/// This trait exists to allow using `Into<Text>` types without having to first
/// convert the type into [`Text`]. A blanket implementation exists for all
/// `Into<Text>` types, including [`Text`] itself.
pub trait TextFormat: Into<Text> {
    /// Converts this type into a [`Text`] object.
    fn into_text(self) -> Text {
        self.into()
    }

    fn color(self, color: Color) -> Text {
        let mut t = self.into();
        t.0.color = Some(color);
        t
    }

    fn clear_color(self) -> Text {
        let mut t = self.into();
        t.0.color = None;
        t
    }

    fn font(self, font: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.0.font = Some(font.into());
        t
    }

    fn clear_font(self) -> Text {
        let mut t = self.into();
        t.0.font = None;
        t
    }

    fn bold(self) -> Text {
        let mut t = self.into();
        t.0.bold = Some(true);
        t
    }

    fn not_bold(self) -> Text {
        let mut t = self.into();
        t.0.bold = Some(false);
        t
    }

    fn clear_bold(self) -> Text {
        let mut t = self.into();
        t.0.bold = None;
        t
    }

    fn italic(self) -> Text {
        let mut t = self.into();
        t.0.italic = Some(true);
        t
    }

    fn not_italic(self) -> Text {
        let mut t = self.into();
        t.0.italic = Some(false);
        t
    }

    fn clear_italic(self) -> Text {
        let mut t = self.into();
        t.0.italic = None;
        t
    }

    fn underlined(self) -> Text {
        let mut t = self.into();
        t.0.underlined = Some(true);
        t
    }

    fn not_underlined(self) -> Text {
        let mut t = self.into();
        t.0.underlined = Some(false);
        t
    }

    fn clear_underlined(self) -> Text {
        let mut t = self.into();
        t.0.underlined = None;
        t
    }

    fn strikethrough(self) -> Text {
        let mut t = self.into();
        t.0.strikethrough = Some(true);
        t
    }

    fn not_strikethrough(self) -> Text {
        let mut t = self.into();
        t.0.strikethrough = Some(false);
        t
    }

    fn clear_strikethrough(self) -> Text {
        let mut t = self.into();
        t.0.strikethrough = None;
        t
    }

    fn obfuscated(self) -> Text {
        let mut t = self.into();
        t.0.obfuscated = Some(true);
        t
    }

    fn not_obfuscated(self) -> Text {
        let mut t = self.into();
        t.0.obfuscated = Some(false);
        t
    }

    fn clear_obfuscated(self) -> Text {
        let mut t = self.into();
        t.0.obfuscated = None;
        t
    }

    fn insertion(self, insertion: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.0.insertion = Some(insertion.into());
        t
    }

    fn clear_insertion(self) -> Text {
        let mut t = self.into();
        t.0.insertion = None;
        t
    }

    fn on_click_open_url(self, url: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.0.click_event = Some(ClickEvent::OpenUrl(url.into()));
        t
    }

    fn on_click_run_command(self, command: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.0.click_event = Some(ClickEvent::RunCommand(command.into()));
        t
    }

    fn on_click_suggest_command(self, command: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.0.click_event = Some(ClickEvent::SuggestCommand(command.into()));
        t
    }

    fn on_click_change_page(self, page: impl Into<i32>) -> Text {
        let mut t = self.into();
        t.0.click_event = Some(ClickEvent::ChangePage(page.into()));
        t
    }

    fn on_click_copy_to_clipboard(self, text: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.0.click_event = Some(ClickEvent::CopyToClipboard(text.into()));
        t
    }

    fn clear_click_event(self) -> Text {
        let mut t = self.into();
        t.0.click_event = None;
        t
    }

    fn on_hover_show_text(self, text: impl Into<Text>) -> Text {
        let mut t = self.into();
        t.0.hover_event = Some(HoverEvent::ShowText(text.into()));
        t
    }

    fn clear_hover_event(self) -> Text {
        let mut t = self.into();
        t.0.hover_event = None;
        t
    }

    fn add_child(self, text: impl Into<Text>) -> Text {
        let mut t = self.into();
        t.0.extra.push(text.into());
        t
    }
}

impl<T: Into<Text>> TextFormat for T {}

impl<T: Into<Text>> ops::Add<T> for Text {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        self.add_child(rhs)
    }
}

impl<T: Into<Text>> ops::AddAssign<T> for Text {
    fn add_assign(&mut self, rhs: T) {
        self.0.extra.push(rhs.into());
    }
}

impl From<char> for Text {
    fn from(c: char) -> Self {
        Text::text(String::from(c))
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Text::text(s)
    }
}

impl From<&'static str> for Text {
    fn from(s: &'static str) -> Self {
        Text::text(s)
    }
}

impl From<Cow<'static, str>> for Text {
    fn from(s: Cow<'static, str>) -> Self {
        Text::text(s)
    }
}

impl From<i32> for Text {
    fn from(value: i32) -> Self {
        Text::text(value.to_string())
    }
}

impl From<i64> for Text {
    fn from(value: i64) -> Self {
        Text::text(value.to_string())
    }
}

impl From<u64> for Text {
    fn from(value: u64) -> Self {
        Text::text(value.to_string())
    }
}

impl From<f64> for Text {
    fn from(value: f64) -> Self {
        Text::text(value.to_string())
    }
}

impl From<bool> for Text {
    fn from(value: bool) -> Self {
        Text::text(value.to_string())
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

impl From<Text> for Value {
    fn from(value: Text) -> Self {
        Value::String(
            serde_json::to_string(&value)
                .unwrap_or_else(|err| panic!("failed to jsonify text {value:?}\n{err}")),
        )
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.write_string(f)
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.write_string(f)
    }
}

impl Encode for Text {
    fn encode(&self, w: impl Write) -> Result<()> {
        serde_json::to_string(self)?.encode(w)
    }
}

impl Decode<'_> for Text {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        let string = <&str>::decode(r)?;
        if string.is_empty() {
            Ok(Self::default())
        } else {
            serde_json::from_str(string).context("decoding text JSON")
        }
    }
}

impl Default for TextContent {
    fn default() -> Self {
        Self::Text { text: "".into() }
    }
}

impl Color {
    pub const AQUA: Color = Color::new(85, 255, 255);
    pub const BLACK: Color = Color::new(0, 0, 0);
    pub const BLUE: Color = Color::new(85, 85, 255);
    pub const DARK_AQUA: Color = Color::new(0, 170, 170);
    pub const DARK_BLUE: Color = Color::new(0, 0, 170);
    pub const DARK_GRAY: Color = Color::new(85, 85, 85);
    pub const DARK_GREEN: Color = Color::new(0, 170, 0);
    pub const DARK_PURPLE: Color = Color::new(170, 0, 170);
    pub const DARK_RED: Color = Color::new(170, 0, 0);
    pub const GOLD: Color = Color::new(255, 170, 0);
    pub const GRAY: Color = Color::new(170, 170, 170);
    pub const GREEN: Color = Color::new(85, 255, 85);
    pub const LIGHT_PURPLE: Color = Color::new(255, 85, 255);
    pub const RED: Color = Color::new(255, 85, 85);
    pub const WHITE: Color = Color::new(255, 255, 255);
    pub const YELLOW: Color = Color::new(255, 255, 85);

    /// Constructs a new color from red, green, and blue components.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b).serialize(serializer)
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
        write!(f, "a hex color of the form #rrggbb")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        color_from_str(s).ok_or_else(|| E::custom("invalid hex color"))
    }
}

fn color_from_str(s: &str) -> Option<Color> {
    let to_num = |d| match d {
        b'0'..=b'9' => Some(d - b'0'),
        b'a'..=b'f' => Some(d - b'a' + 0xa),
        b'A'..=b'F' => Some(d - b'A' + 0xa),
        _ => None,
    };

    match s.as_bytes() {
        [b'#', r0, r1, g0, g1, b0, b1] => Some(Color {
            r: to_num(*r0)? << 4 | to_num(*r1)?,
            g: to_num(*g0)? << 4 | to_num(*g1)?,
            b: to_num(*b0)? << 4 | to_num(*b1)?,
        }),
        _ => match s {
            "aqua" => Some(Color::AQUA),
            "black" => Some(Color::BLACK),
            "blue" => Some(Color::BLUE),
            "dark_aqua" => Some(Color::DARK_AQUA),
            "dark_blue" => Some(Color::DARK_BLUE),
            "dark_gray" => Some(Color::DARK_GRAY),
            "dark_green" => Some(Color::DARK_GREEN),
            "dark_purple" => Some(Color::DARK_PURPLE),
            "dark_red" => Some(Color::DARK_RED),
            "gold" => Some(Color::GOLD),
            "gray" => Some(Color::GRAY),
            "green" => Some(Color::GREEN),
            "light_purple" => Some(Color::LIGHT_PURPLE),
            "red" => Some(Color::RED),
            "white" => Some(Color::WHITE),
            "yellow" => Some(Color::YELLOW),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ident, translation_key};

    #[test]
    fn text_round_trip() {
        let before = "foo".color(Color::RED).bold()
            + ("bar".obfuscated().color(Color::YELLOW)
                + "baz".underlined().not_bold().italic().color(Color::BLACK));

        assert_eq!(before.to_string(), "foobarbaz");

        let json = serde_json::to_string_pretty(&before).unwrap();

        let after: Text = serde_json::from_str(&json).unwrap();

        println!("==== Before ====\n");
        println!("{before:#?}");
        println!("==== After ====\n");
        println!("{after:#?}");

        assert_eq!(before, after);
        assert_eq!(before.to_string(), after.to_string());
    }

    #[test]
    fn text_color() {
        assert_eq!(
            color_from_str("#aBcDeF"),
            Some(Color::new(0xab, 0xcd, 0xef))
        );
        assert_eq!(color_from_str("#fFfFfF"), Some(Color::new(255, 255, 255)));
        assert_eq!(color_from_str("#00000000"), None);
        assert_eq!(color_from_str("#000000"), Some(Color::BLACK));
        assert_eq!(color_from_str("#"), None);
        assert_eq!(color_from_str("red"), Some(Color::RED));
        assert_eq!(color_from_str("blue"), Some(Color::BLUE));
    }

    #[test]
    fn non_object_data_types() {
        let input = r#"["foo", true, false, 1.9E10, 9999]"#;
        let txt: Text = serde_json::from_str(input).unwrap();

        assert_eq!(txt, "foo".into_text() + true + false + 1.9E10 + 9999);
    }

    #[test]
    fn translate() {
        let txt = Text::translate(
            translation_key::CHAT_TYPE_ADVANCEMENT_TASK,
            ["arg1".into(), "arg2".into()],
        );
        let serialized = serde_json::to_string(&txt).unwrap();
        let deserialized: Text = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            serialized,
            r#"{"translate":"chat.type.advancement.task","with":[{"text":"arg1"},{"text":"arg2"}]}"#
        );
        assert_eq!(txt, deserialized);
    }

    #[test]
    fn score() {
        let txt = Text::score("foo", "bar", Some(Cow::from("baz")));
        let serialized = serde_json::to_string(&txt).unwrap();
        let deserialized: Text = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            serialized,
            r#"{"score":{"name":"foo","objective":"bar","value":"baz"}}"#
        );
        assert_eq!(txt, deserialized);
    }

    #[test]
    fn selector() {
        let separator = Text::text("bar").color(Color::RED).bold();
        let txt = Text::selector("foo", Some(separator));
        let serialized = serde_json::to_string(&txt).unwrap();
        let deserialized: Text = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            serialized,
            r##"{"selector":"foo","separator":{"text":"bar","color":"#ff5555","bold":true}}"##
        );
        assert_eq!(txt, deserialized);
    }

    #[test]
    fn keybind() {
        let txt = Text::keybind("foo");
        let serialized = serde_json::to_string(&txt).unwrap();
        let deserialized: Text = serde_json::from_str(&serialized).unwrap();
        assert_eq!(serialized, r#"{"keybind":"foo"}"#);
        assert_eq!(txt, deserialized);
    }

    #[test]
    fn block_nbt() {
        let txt = Text::block_nbt("foo", "bar", Some(true), Some("baz".into()));
        let serialized = serde_json::to_string(&txt).unwrap();
        let deserialized: Text = serde_json::from_str(&serialized).unwrap();
        let expected = r#"{"block":"foo","nbt":"bar","interpret":true,"separator":{"text":"baz"}}"#;
        assert_eq!(serialized, expected);
        assert_eq!(txt, deserialized);
    }

    #[test]
    fn entity_nbt() {
        let txt = Text::entity_nbt("foo", "bar", Some(true), Some("baz".into()));
        let serialized = serde_json::to_string(&txt).unwrap();
        let deserialized: Text = serde_json::from_str(&serialized).unwrap();
        let expected =
            r#"{"entity":"foo","nbt":"bar","interpret":true,"separator":{"text":"baz"}}"#;
        assert_eq!(serialized, expected);
        assert_eq!(txt, deserialized);
    }

    #[test]
    fn storage_nbt() {
        let txt = Text::storage_nbt(ident!("foo"), "bar", Some(true), Some("baz".into()));
        let serialized = serde_json::to_string(&txt).unwrap();
        let deserialized: Text = serde_json::from_str(&serialized).unwrap();
        let expected =
            r#"{"storage":"foo","nbt":"bar","interpret":true,"separator":{"text":"baz"}}"#;
        assert_eq!(serialized, expected);
        assert_eq!(txt, deserialized);
    }
}
