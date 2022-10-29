//! Formatted text.

use std::borrow::Cow;
use std::fmt;
use std::io::Write;

use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::ident::Ident;
use crate::protocol::{BoundedString, Decode, Encode};

/// Represents formatted text in Minecraft's JSON text format.
///
/// Text is used in various places such as chat, window titles,
/// disconnect messages, written books, signs, and more.
///
/// For more information, see the relevant [Minecraft Wiki article].
///
/// Note that the current [`Deserialize`] implementation on this type recognizes
/// only a subset of the full JSON chat component format.
///
/// [Minecraft Wiki article]: https://minecraft.fandom.com/wiki/Raw_JSON_text_format
///
/// # Examples
///
/// With [`TextFormat`] in scope, you can write the following:
/// ```
/// use valence::text::{Color, Text, TextFormat};
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
///     txt.to_plain(),
///     "The text is Red, Green, and also Blue!\nAnd maybe even Italic."
/// );
/// ```
#[derive(Clone, PartialEq, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Text {
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

#[allow(clippy::self_named_constructors)]
impl Text {
    /// Constructs a new plain text object.
    pub fn text(plain: impl Into<Cow<'static, str>>) -> Self {
        Self {
            content: TextContent::Text { text: plain.into() },
            ..Self::default()
        }
    }

    /// Create translated text based on the given translation key.
    pub fn translate(key: impl Into<Cow<'static, str>>) -> Self {
        Self::translate_with_slots(key, Vec::default())
    }

    /// Create translated text based on the given translation key, with extra
    /// text components to be inserted into the slots in the translation text.
    pub fn translate_with_slots(
        key: impl Into<Cow<'static, str>>,
        with: impl Into<Vec<Text>>,
    ) -> Self {
        Self {
            content: TextContent::Translate {
                translate: key.into(),
                with: with.into(),
            },
            ..Self::default()
        }
    }

    /// Create a score from the scoreboard.
    pub fn score(
        name: impl Into<Cow<'static, str>>,
        objective: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::score_with_value(name, objective, None::<Cow<'static, str>>)
    }

    /// Create a score from the scoreboard with a custom value.
    pub fn score_with_value(
        name: impl Into<Cow<'static, str>>,
        objective: impl Into<Cow<'static, str>>,
        value: Option<impl Into<Cow<'static, str>>>,
    ) -> Self {
        Self {
            content: TextContent::ScoreboardValue {
                score: ScoreboardValueContent {
                    name: name.into(),
                    objective: objective.into(),
                    value: value.map(|val| val.into()),
                },
            },
            ..Self::default()
        }
    }

    /// Creates a text component for selecting entity names.
    pub fn entity_names(selector: impl Into<Cow<'static, str>>) -> Self {
        Self::entity_names_with_separator(selector, None::<Text>)
    }

    /// Creates a text component for selecting entity names, with a custom
    /// separator.
    pub fn entity_names_with_separator(
        selector: impl Into<Cow<'static, str>>,
        separator: Option<impl Into<Text>>,
    ) -> Self {
        Self {
            content: TextContent::EntityNames {
                selector: selector.into(),
                separator: separator.map(|v| Box::new(v.into())),
            },
            ..Self::default()
        }
    }

    /// Creates a text component for a keybind. The keybind should be a valid
    /// [`keybind identifier`].
    ///
    /// [`keybind identifier`]: https://minecraft.fandom.com/wiki/Controls#Configurable_controls
    pub fn keybind(keybind: impl Into<Cow<'static, str>>) -> Self {
        Self {
            content: TextContent::Keybind {
                keybind: keybind.into(),
            },
            ..Self::default()
        }
    }

    /// Creates a text component for a block NBT tag.
    pub fn block_nbt(
        nbt: impl Into<Cow<'static, str>>,
        block: impl Into<Cow<'static, str>>,
        interpret: Option<bool>,
        separator: Option<impl Into<Text>>,
    ) -> Self {
        Self {
            content: TextContent::Nbt {
                nbt: nbt.into(),
                interpret,
                separator: separator.map(|v| Box::new(v.into())),
                block: Some(block.into()),
                entity: None,
                storage: None,
            },
            ..Self::default()
        }
    }

    /// Creates a text component for an entity NBT tag.
    pub fn entity_nbt(
        nbt: impl Into<Cow<'static, str>>,
        entity: impl Into<Cow<'static, str>>,
        interpret: Option<bool>,
        separator: Option<impl Into<Text>>,
    ) -> Self {
        Self {
            content: TextContent::Nbt {
                nbt: nbt.into(),
                interpret,
                separator: separator.map(|v| Box::new(v.into())),
                block: None,
                entity: Some(entity.into()),
                storage: None,
            },
            ..Self::default()
        }
    }

    /// Creates a text component for a command storage NBT tag.
    pub fn storage_nbt(
        nbt: impl Into<Cow<'static, str>>,
        storage: impl Into<Ident<String>>,
        interpret: Option<bool>,
        separator: Option<impl Into<Text>>,
    ) -> Self {
        Self {
            content: TextContent::Nbt {
                nbt: nbt.into(),
                interpret,
                separator: separator.map(|v| Box::new(v.into())),
                block: None,
                entity: None,
                storage: Some(storage.into()),
            },
            ..Self::default()
        }
    }

    /// Gets this text object as plain text without any formatting.
    pub fn to_plain(&self) -> String {
        let mut res = String::new();
        self.write_plain(&mut res)
            .expect("failed to write plain text");
        res
    }

    /// Writes this text object as plain text to the provided writer.
    pub fn write_plain(&self, mut w: impl fmt::Write) -> fmt::Result {
        fn write_plain_impl(this: &Text, w: &mut impl fmt::Write) -> fmt::Result {
            match &this.content {
                TextContent::Text { text } => w.write_str(text.as_ref())?,
                TextContent::Translate { translate, with } => {
                    w.write_str(translate.as_ref())?;

                    if !with.is_empty() {
                        w.write_char('{')?;
                        for slot in with {
                            write_plain_impl(slot, w)?;
                            w.write_char(',')?;
                        }
                        w.write_char('}')?;
                    }
                }
                TextContent::ScoreboardValue { score } => {
                    let ScoreboardValueContent {
                        name,
                        objective,
                        value,
                    } = score;

                    write!(w, "scoreboard_value(name={name}, objective={objective}")?;

                    if let Some(value) = value {
                        if !value.is_empty() {
                            w.write_str(", value=")?;
                            w.write_str(value)?;
                        }
                    }

                    w.write_char(')')?;
                }
                TextContent::EntityNames {
                    selector,
                    separator,
                } => {
                    write!(w, "entity_names(selector={selector}")?;

                    if let Some(separator) = separator {
                        if !separator.is_empty() {
                            w.write_str(", separator=")?;
                            write_plain_impl(separator, w)?;
                        }
                    }

                    w.write_char(')')?;
                }
                TextContent::Keybind { keybind } => write!(w, "keybind({keybind})")?,
                TextContent::Nbt {
                    nbt,
                    interpret,
                    separator,
                    block,
                    entity,
                    storage,
                } => {
                    write!(w, "nbt(nbt={nbt}")?;

                    if let Some(interpret) = interpret {
                        write!(w, ", interpret={interpret}")?;
                    }

                    if let Some(separator) = separator {
                        if !separator.is_empty() {
                            w.write_str(", separator=")?;
                            write_plain_impl(separator, w)?;
                        }
                    }

                    if let Some(block) = block {
                        write!(w, ", block={block}")?;
                    }

                    if let Some(entity) = entity {
                        write!(w, ", entity={entity}")?;
                    }

                    if let Some(storage) = storage {
                        write!(w, ", storage={storage}")?;
                    }

                    w.write_char(')')?;
                }
            }

            for child in &this.extra {
                write_plain_impl(child, w)?;
            }

            Ok(())
        }

        write_plain_impl(self, &mut w)
    }

    /// Returns `true` if the text contains no characters. Returns `false`
    /// otherwise.
    pub fn is_empty(&self) -> bool {
        for extra in &self.extra {
            if !extra.is_empty() {
                return false;
            }
        }

        match &self.content {
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
            TextContent::Nbt {
                nbt,
                block,
                entity,
                storage,
                ..
            } => nbt.is_empty() || (block.is_none() && entity.is_none() && storage.is_none()),
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
        t.color = Some(color);
        t
    }

    fn clear_color(self) -> Text {
        let mut t = self.into();
        t.color = None;
        t
    }

    fn font(self, font: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.font = Some(font.into());
        t
    }

    fn clear_font(self) -> Text {
        let mut t = self.into();
        t.font = None;
        t
    }

    fn bold(self) -> Text {
        let mut t = self.into();
        t.bold = Some(true);
        t
    }

    fn not_bold(self) -> Text {
        let mut t = self.into();
        t.bold = Some(false);
        t
    }

    fn clear_bold(self) -> Text {
        let mut t = self.into();
        t.bold = None;
        t
    }

    fn italic(self) -> Text {
        let mut t = self.into();
        t.italic = Some(true);
        t
    }

    fn not_italic(self) -> Text {
        let mut t = self.into();
        t.italic = Some(false);
        t
    }

    fn clear_italic(self) -> Text {
        let mut t = self.into();
        t.italic = None;
        t
    }

    fn underlined(self) -> Text {
        let mut t = self.into();
        t.underlined = Some(true);
        t
    }

    fn not_underlined(self) -> Text {
        let mut t = self.into();
        t.underlined = Some(false);
        t
    }

    fn clear_underlined(self) -> Text {
        let mut t = self.into();
        t.underlined = None;
        t
    }

    fn strikethrough(self) -> Text {
        let mut t = self.into();
        t.strikethrough = Some(true);
        t
    }

    fn not_strikethrough(self) -> Text {
        let mut t = self.into();
        t.strikethrough = Some(false);
        t
    }

    fn clear_strikethrough(self) -> Text {
        let mut t = self.into();
        t.strikethrough = None;
        t
    }

    fn obfuscated(self) -> Text {
        let mut t = self.into();
        t.obfuscated = Some(true);
        t
    }

    fn not_obfuscated(self) -> Text {
        let mut t = self.into();
        t.obfuscated = Some(false);
        t
    }

    fn clear_obfuscated(self) -> Text {
        let mut t = self.into();
        t.obfuscated = None;
        t
    }

    fn insertion(self, insertion: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.insertion = Some(insertion.into());
        t
    }

    fn clear_insertion(self) -> Text {
        let mut t = self.into();
        t.insertion = None;
        t
    }

    fn on_click_open_url(self, url: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.click_event = Some(ClickEvent::OpenUrl(url.into()));
        t
    }

    fn on_click_run_command(self, command: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.click_event = Some(ClickEvent::RunCommand(command.into()));
        t
    }

    fn on_click_suggest_command(self, command: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.click_event = Some(ClickEvent::SuggestCommand(command.into()));
        t
    }

    fn on_click_change_page(self, page: impl Into<i32>) -> Text {
        let mut t = self.into();
        t.click_event = Some(ClickEvent::ChangePage(page.into()));
        t
    }

    fn on_click_copy_to_clipboard(self, text: impl Into<Cow<'static, str>>) -> Text {
        let mut t = self.into();
        t.click_event = Some(ClickEvent::CopyToClipboard(text.into()));
        t
    }

    fn clear_click_event(self) -> Text {
        let mut t = self.into();
        t.click_event = None;
        t
    }

    fn on_hover_show_text(self, text: impl Into<Text>) -> Text {
        let mut t = self.into();
        t.hover_event = Some(HoverEvent::ShowText(Box::new(text.into())));
        t
    }

    fn clear_hover_event(self) -> Text {
        let mut t = self.into();
        t.hover_event = None;
        t
    }

    fn add_child(self, text: impl Into<Text>) -> Text {
        let mut t = self.into();
        t.extra.push(text.into());
        t
    }
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
        separator: Option<Box<Text>>,
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
    /// Displays NBT values from entities, block entities, or command storage.
    Nbt {
        /// The [`NBT path`] used for looking up NBT values from an entity,
        /// block entity, or storage.
        ///
        /// [`NBT path`]: https://minecraft.fandom.com/wiki/NBT_path_format
        nbt: Cow<'static, str>,
        /// Optional property that, when set to true, attempts to parse the text
        /// of each NBT value as a raw JSON text component.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpret: Option<bool>,
        /// An optional custom separator used when the NBT selector has multiple
        /// tags. Defaults to the ", " text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        separator: Option<Box<Text>>,
        /// A string specifying the coordinates of the block entity from which
        /// the NBT value is obtained. The coordinates can be absolute,
        /// relative, or local.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        block: Option<Cow<'static, str>>,
        /// A string specifying the [`selector`] for the entity or entities
        /// from which the NBT value is obtained.
        ///
        /// [`selector`]: https://minecraft.fandom.com/wiki/Target_selectors
        #[serde(default, skip_serializing_if = "Option::is_none")]
        entity: Option<Cow<'static, str>>,
        /// A string specifying the resource location of the command storage
        /// from which the NBT value is obtained.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        storage: Option<Ident<String>>,
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
    ShowText(Box<Text>),
    ShowItem {
        id: Ident<String>,
        count: Option<i32>,
        // TODO: tag
    },
    ShowEntity {
        name: Box<Text>,
        #[serde(rename = "type")]
        kind: Ident<String>,
        // TODO: id (hyphenated entity UUID as a string)
    },
}

impl<T: Into<Text>> TextFormat for T {}

impl<T: Into<Text>> std::ops::Add<T> for Text {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        self.add_child(rhs)
    }
}

impl<T: Into<Text>> std::ops::AddAssign<T> for Text {
    fn add_assign(&mut self, rhs: T) {
        self.extra.push(rhs.into());
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

impl<'a> From<&'a Text> for String {
    fn from(t: &'a Text) -> Self {
        t.to_plain()
    }
}

impl<'a, 'b> From<&'a Text> for Cow<'b, str> {
    fn from(t: &'a Text) -> Self {
        String::from(t).into()
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.write_plain(f)
    }
}

impl Encode for Text {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        BoundedString::<0, 262144>(serde_json::to_string(self)?).encode(w)
    }

    fn encoded_len(&self) -> usize {
        // TODO: This is obviously not ideal. This will be fixed later.
        serde_json::to_string(self).map_or(0, |s| s.encoded_len())
    }
}

impl Decode for Text {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let string = BoundedString::<0, 262144>::decode(r)?;
        Ok(serde_json::from_str(&string.0)?)
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
    use crate::ident;

    #[test]
    fn serialize_deserialize() {
        let before = "foo".color(Color::RED).bold()
            + ("bar".obfuscated().color(Color::YELLOW)
                + "baz".underlined().not_bold().italic().color(Color::BLACK));

        assert_eq!(before.to_plain(), "foobarbaz");

        let json = serde_json::to_string_pretty(&before).unwrap();

        let after: Text = serde_json::from_str(&json).unwrap();

        println!("==== Before ====\n");
        println!("{before:#?}");
        println!("==== After ====\n");
        println!("{after:#?}");

        assert_eq!(before, after);
        assert_eq!(before.to_plain(), after.to_plain());
    }

    #[test]
    fn color() {
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
    fn empty() {
        assert!("".into_text().is_empty());

        let txt = "".into_text() + Text::translate("") + ("".italic().color(Color::RED) + "");
        assert!(txt.is_empty());
        assert!(txt.to_plain().is_empty());
    }

    #[test]
    fn translate() {
        let text = Text::translate("key");
        let json = serde_json::to_string(&text).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();
        assert_eq!(text, after);
        assert_eq!(text.to_plain(), after.to_plain());
        assert_eq!(text.to_plain(), "key");
        assert_eq!(json, "{\"translate\":\"key\"}");

        let text = Text::translate_with_slots("key", []);
        let json = serde_json::to_string(&text).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();
        assert_eq!(text, after);
        assert_eq!(text.to_plain(), after.to_plain());
        assert_eq!(text.to_plain(), "key");
        assert_eq!(json, "{\"translate\":\"key\"}");

        let text = Text::translate_with_slots("key", [Text::text("arg1"), Text::text("arg2")]);
        let json = serde_json::to_string(&text).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();
        assert_eq!(text, after);
        assert_eq!(text.to_plain(), after.to_plain());
        assert_eq!(text.to_plain(), "key{arg1,arg2,}");
        assert_eq!(
            json,
            "{\"translate\":\"key\",\"with\":[{\"text\":\"arg1\"},{\"text\":\"arg2\"}]}"
        );
    }

    #[test]
    fn score() {
        let score = Text::score("foo", "bar");
        let json = serde_json::to_string(&score).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();

        assert_eq!(score, after);
        assert_eq!(score.to_plain(), after.to_plain());
        assert_eq!(
            score.to_plain(),
            "scoreboard_value(name=foo, objective=bar)"
        );
        assert_eq!(json, "{\"score\":{\"name\":\"foo\",\"objective\":\"bar\"}}");
    }

    #[test]
    fn score_with_value() {
        let score = Text::score_with_value("foo", "bar", Some("baz"));
        let json = serde_json::to_string(&score).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();

        assert_eq!(score, after);
        assert_eq!(score.to_plain(), after.to_plain());
        assert_eq!(
            score.to_plain(),
            "scoreboard_value(name=foo, objective=bar, value=baz)"
        );
        assert_eq!(
            json,
            "{\"score\":{\"name\":\"foo\",\"objective\":\"bar\",\"value\":\"baz\"}}"
        );
    }

    #[test]
    fn empty_score() {
        // All properties empty
        let score = Text::score("", "");
        assert!(score.is_empty());
        assert_eq!(score.to_plain(), "scoreboard_value(name=, objective=)");

        let score = Text::score_with_value("", "", Some(""));
        assert!(score.is_empty());
        assert_eq!(score.to_plain(), "scoreboard_value(name=, objective=)");

        // Name set
        let score = Text::score("foo", "");
        assert!(score.is_empty());
        assert_eq!(score.to_plain(), "scoreboard_value(name=foo, objective=)");

        // Objective set
        let score = Text::score("", "bar");
        assert!(score.is_empty());
        assert_eq!(score.to_plain(), "scoreboard_value(name=, objective=bar)");

        // Name and objective set
        let score = Text::score("foo", "bar");
        assert!(!score.is_empty());
        assert_eq!(
            score.to_plain(),
            "scoreboard_value(name=foo, objective=bar)"
        );

        // Value set
        let score = Text::score_with_value("", "", Some("baz"));
        assert!(score.is_empty());
        assert_eq!(
            score.to_plain(),
            "scoreboard_value(name=, objective=, value=baz)"
        );
    }

    #[test]
    fn entity_names() {
        let entity_names = Text::entity_names("foo");
        let json = serde_json::to_string(&entity_names).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();

        assert!(!entity_names.is_empty());
        assert_eq!(entity_names, after);
        assert_eq!(entity_names.to_plain(), after.to_plain());
        assert_eq!(entity_names.to_plain(), "entity_names(selector=foo)");
        assert_eq!(json, "{\"selector\":\"foo\"}");
    }

    #[test]
    fn entity_names_with_separator() {
        let separator = Text::text("bar").color(Color::RED).bold();
        let text = Text::entity_names_with_separator("foo", Some(separator));
        let json = serde_json::to_string(&text).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();

        assert!(!text.is_empty());
        assert_eq!(text, after);
        assert_eq!(text.to_plain(), after.to_plain());
        assert_eq!(text.to_plain(), "entity_names(selector=foo, separator=bar)");
        assert_eq!(
            json,
            "{\"selector\":\"foo\",\"separator\":{\"text\":\"bar\",\"color\":\"#ff5555\",\"bold\":\
             true}}"
        );
    }

    #[test]
    fn empty_entity_names() {
        let entity_names = Text::entity_names("");
        assert!(entity_names.is_empty());
        assert_eq!(entity_names.to_plain(), "entity_names(selector=)");

        let entity_names = Text::entity_names_with_separator("", Some(""));
        assert!(entity_names.is_empty());
        assert_eq!(entity_names.to_plain(), "entity_names(selector=)");
    }

    #[test]
    fn keybind() {
        let text = Text::keybind("foo");
        let json = serde_json::to_string(&text).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();

        assert!(!text.is_empty());
        assert_eq!(text, after);
        assert_eq!(text.to_plain(), after.to_plain());
        assert_eq!(text.to_plain(), "keybind(foo)");
        assert_eq!(json, "{\"keybind\":\"foo\"}");
    }

    #[test]
    fn empty_keybind() {
        let entity_names = Text::keybind("");
        assert!(entity_names.is_empty());
        assert_eq!(entity_names.to_plain(), "keybind()");
    }

    #[test]
    fn block_nbt() {
        let text = Text::block_nbt("foo", "bar", Some(true), Some("baz"));
        let json = serde_json::to_string(&text).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();

        assert!(!text.is_empty());
        assert_eq!(text, after);
        assert_eq!(text.to_plain(), after.to_plain());
        assert_eq!(
            text.to_plain(),
            "nbt(nbt=foo, interpret=true, separator=baz, block=bar)"
        );
        let expected_json = "{\"nbt\":\"foo\",\"interpret\":true,\"separator\":{\"text\":\"baz\"},\
                             \"block\":\"bar\"}";
        assert_eq!(json, expected_json);

        let empty = Text::block_nbt("", "", None, None::<Text>);
        assert!(empty.is_empty());
        assert_eq!(empty.to_plain(), "nbt(nbt=, block=)");
    }

    #[test]
    fn entity_nbt() {
        let text = Text::entity_nbt("foo", "bar", Some(true), Some("baz"));
        let json = serde_json::to_string(&text).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();

        assert!(!text.is_empty());
        assert_eq!(text, after);
        assert_eq!(text.to_plain(), after.to_plain());
        assert_eq!(
            text.to_plain(),
            "nbt(nbt=foo, interpret=true, separator=baz, entity=bar)"
        );
        let expected_json = "{\"nbt\":\"foo\",\"interpret\":true,\"separator\":{\"text\":\"baz\"},\
                             \"entity\":\"bar\"}";
        assert_eq!(json, expected_json);

        let empty = Text::entity_nbt("", "", None, None::<Text>);
        assert!(empty.is_empty());
        assert_eq!(empty.to_plain(), "nbt(nbt=, entity=)");
    }

    #[test]
    fn storage_nbt() {
        let text = Text::storage_nbt("foo", ident!("bar"), Some(true), Some("baz"));
        let json = serde_json::to_string(&text).unwrap();
        let after: Text = serde_json::from_str(&json).unwrap();

        assert!(!text.is_empty());
        assert_eq!(text, after);
        assert_eq!(text.to_plain(), after.to_plain());
        assert_eq!(
            text.to_plain(),
            "nbt(nbt=foo, interpret=true, separator=baz, storage=minecraft:bar)"
        );
        let expected_json = "{\"nbt\":\"foo\",\"interpret\":true,\"separator\":{\"text\":\"baz\"},\
                             \"storage\":\"bar\"}";
        assert_eq!(json, expected_json);

        let empty = Text::storage_nbt("", ident!("bar"), None, None::<Text>);
        assert!(empty.is_empty());
        assert_eq!(empty.to_plain(), "nbt(nbt=, storage=minecraft:bar)");
    }
}
