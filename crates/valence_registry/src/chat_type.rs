//! ChatType configuration and identification.
//!
//! **NOTE:**
//! - Modifying the chat type registry after the server has started can
//! break invariants within instances and clients! Make sure there are no
//! instances or clients spawned before mutating.

use std::fmt;
use std::ops::{Deref, DerefMut};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use tracing::error;
use valence_ident::{ident, Ident};
use valence_nbt::serde::CompoundSerializer;
use valence_text::Color;

use crate::codec::{RegistryCodec, RegistryValue};
use crate::{Registry, RegistryIdx, RegistrySet};

pub struct ChatTypePlugin;

impl Plugin for ChatTypePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ChatTypeRegistry>()
            .add_systems(PreStartup, load_default_chat_types)
            .add_systems(PostUpdate, update_chat_type_registry.before(RegistrySet));
    }
}

fn load_default_chat_types(mut reg: ResMut<ChatTypeRegistry>, codec: Res<RegistryCodec>) {
    let mut helper = move || -> anyhow::Result<()> {
        for value in codec.registry(ChatTypeRegistry::KEY) {
            let chat_type = ChatType::deserialize(value.element.clone())?;

            reg.insert(value.name.clone(), chat_type);
        }

        reg.swap_to_front(ident!("chat"));

        Ok(())
    };

    if let Err(e) = helper() {
        error!("failed to load default chat types from registry codec: {e:#}");
    }
}

/// Add new chat types to or update existing chat types in the registry.
fn update_chat_type_registry(reg: Res<ChatTypeRegistry>, mut codec: ResMut<RegistryCodec>) {
    if reg.is_changed() {
        let chat_types = codec.registry_mut(ChatTypeRegistry::KEY);

        chat_types.clear();

        chat_types.extend(reg.iter().map(|(_, name, chat_type)| {
            RegistryValue {
                name: name.into(),
                element: chat_type
                    .serialize(CompoundSerializer)
                    .expect("failed to serialize chat type"),
            }
        }));
    }
}

#[derive(Resource, Default, Debug)]
pub struct ChatTypeRegistry {
    reg: Registry<ChatTypeId, ChatType>,
}

impl ChatTypeRegistry {
    pub const KEY: Ident<&str> = ident!("chat_type");
}

impl Deref for ChatTypeRegistry {
    type Target = Registry<ChatTypeId, ChatType>;

    fn deref(&self) -> &Self::Target {
        &self.reg
    }
}

impl DerefMut for ChatTypeRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reg
    }
}

/// An index into the chat type registry
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct ChatTypeId(pub u16);

impl ChatTypeId {
    pub const DEFAULT: Self = ChatTypeId(0);
}

impl RegistryIdx for ChatTypeId {
    const MAX: usize = u32::MAX as _;

    #[inline]
    fn to_index(self) -> usize {
        self.0 as _
    }

    #[inline]
    fn from_index(idx: usize) -> Self {
        Self(idx as _)
    }
}

/// Contains information about how chat is styled, such as the chat color. The
/// notchian server has different chat types for team chat and direct messages.
///
/// Note that [`ChatTypeDecoration::style`] for [`ChatType::narration`]
/// is unused by the notchian client and is ignored.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatType {
    pub chat: ChatTypeDecoration,
    pub narration: ChatTypeDecoration,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct ChatTypeDecoration {
    pub translation_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<ChatTypeStyle>,
    pub parameters: ChatTypeParameters,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct ChatTypeStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlined: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfuscated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insertion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<Ident<String>>,
    // TODO
    // * click_event: Option<ClickEvent>,
    // * hover_event: Option<HoverEvent>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct ChatTypeParameters {
    sender: bool,
    target: bool,
    content: bool,
}

impl Default for ChatType {
    fn default() -> Self {
        Self {
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

impl Serialize for ChatTypeParameters {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut args = vec![];
        if self.sender {
            args.push("sender");
        }
        if self.target {
            args.push("target");
        }
        if self.content {
            args.push("content");
        }
        args.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChatTypeParameters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ParameterVisitor;

        impl<'de> de::Visitor<'de> for ParameterVisitor {
            type Value = ChatTypeParameters;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ChatTypeParameters")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                let mut value = Self::Value::default();
                while let Some(element) = seq.next_element::<String>()? {
                    match element.as_str() {
                        "sender" => value.sender = true,
                        "target" => value.target = true,
                        "content" => value.content = true,
                        _ => return Err(de::Error::unknown_field(&element, FIELDS)),
                    }
                }
                Ok(value)
            }
        }

        const FIELDS: &[&str] = &["sender", "target", "content"];
        deserializer.deserialize_struct("ChatTypeParameters", FIELDS, ParameterVisitor)
    }
}
