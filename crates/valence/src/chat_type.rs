//! ChatType configuration and identification.

use std::collections::HashSet;

use anyhow::ensure;
use valence_nbt::{compound, Compound, List};
use valence_protocol::ident;
use valence_protocol::ident::Ident;
use valence_protocol::text::Color;

/// Identifies a particular [`ChatType`] on the server.
///
/// The default chat type ID refers to the first chat type added in
/// [`ServerPlugin::chat_types`].
///
/// To obtain chat type IDs for other chat types, see
/// [`ServerPlugin::chat_types`].
///
/// [`ServerPlugin::chat_types`]: crate::config::ServerPlugin::chat_types
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ChatTypeId(pub(crate) u16);

/// Contains the configuration for a chat type.
///
/// Chat types are registered once at startup through
/// [`ServerPlugin::with_chat_types`]
///
/// Note that [`ChatTypeDecoration::style`] for [`ChatType::narration`]
/// is unused by the notchian client and is ignored.
///
/// [`ServerPlugin::with_chat_types`]: crate::config::ServerPlugin::with_chat_types
#[derive(Clone, Debug)]
pub struct ChatType {
    pub name: Ident<String>,
    pub chat: ChatTypeDecoration,
    pub narration: ChatTypeDecoration,
}

impl ChatType {
    pub(crate) fn to_chat_type_registry_item(&self, id: i32) -> Compound {
        compound! {
            "name" => self.name.clone(),
            "id" => id,
            "element" => compound! {
                "chat" => {
                    let mut chat = compound! {
                        "translation_key" => self.chat.translation_key.clone(),
                        "parameters" => {
                            let mut parameters = Vec::new();
                            if self.chat.parameters.sender {
                                parameters.push("sender".to_string());
                            }
                            if self.chat.parameters.target {
                                parameters.push("target".to_string());
                            }
                            if self.chat.parameters.content {
                                parameters.push("content".to_string());
                            }
                            List::String(parameters)
                        },
                    };
                    if let Some(style) = &self.chat.style {
                        let mut s = Compound::new();
                        if let Some(color) = style.color {
                            s.insert("color", format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b));
                        }
                        if let Some(bold) = style.bold {
                            s.insert("bold", bold);
                        }
                        if let Some(italic) = style.italic {
                            s.insert("italic", italic);
                        }
                        if let Some(underlined) = style.underlined {
                            s.insert("underlined", underlined);
                        }
                        if let Some(strikethrough) = style.strikethrough {
                            s.insert("strikethrough", strikethrough);
                        }
                        if let Some(obfuscated) = style.obfuscated {
                            s.insert("obfuscated", obfuscated);
                        }
                        if let Some(insertion) = &style.insertion {
                            s.insert("insertion", insertion.clone());
                        }
                        if let Some(font) = &style.font {
                            s.insert("font", font.clone());
                        }
                        chat.insert("style", s);
                    }
                    chat
                },
                "narration" => compound! {
                    "translation_key" => self.narration.translation_key.clone(),
                    "parameters" => {
                        let mut parameters = Vec::new();
                        if self.narration.parameters.sender {
                            parameters.push("sender".into());
                        }
                        if self.narration.parameters.target {
                            parameters.push("target".into());
                        }
                        if self.narration.parameters.content {
                            parameters.push("content".into());
                        }
                        List::String(parameters)
                    },
                },
            }
        }
    }
}

pub(crate) fn validate_chat_types(chat_types: &[ChatType]) -> anyhow::Result<()> {
    ensure!(
        !chat_types.is_empty(),
        "at least one chat type must be present"
    );

    ensure!(
        chat_types.len() <= u16::MAX as _,
        "more than u16::MAX chat types present"
    );

    let mut names = HashSet::new();

    for chat_type in chat_types {
        ensure!(
            names.insert(chat_type.name.clone()),
            "chat type \"{}\" already exists",
            chat_type.name
        );
    }

    Ok(())
}

impl Default for ChatType {
    fn default() -> Self {
        Self {
            name: ident!("chat"),
            chat: ChatTypeDecoration {
                translation_key: "chat.type.text".into(),
                style: None,
                parameters: ChatTypeParameters {
                    sender: true,
                    content: true,
                    ..Default::default()
                },
            },
            narration: ChatTypeDecoration {
                translation_key: "chat.type.text.narrate".into(),
                style: None,
                parameters: ChatTypeParameters {
                    sender: true,
                    content: true,
                    ..Default::default()
                },
            },
        }
    }
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct ChatTypeDecoration {
    pub translation_key: String,
    pub style: Option<ChatTypeStyle>,
    pub parameters: ChatTypeParameters,
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct ChatTypeStyle {
    pub color: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub strikethrough: Option<bool>,
    pub obfuscated: Option<bool>,
    pub insertion: Option<String>,
    pub font: Option<Ident<String>>,
    // TODO
    // * click_event: Option<ClickEvent>,
    // * hover_event: Option<HoverEvent>,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct ChatTypeParameters {
    sender: bool,
    target: bool,
    content: bool,
}
