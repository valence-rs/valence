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
    // TODO: entity names
    // TODO: keybind
    // TODO: nbt
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
}
