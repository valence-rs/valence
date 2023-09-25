//! ChatType configuration and identification.
//!
//! **NOTE:**
//!
//! - Modifying the chat type registry after the server has started can
//! break invariants within instances and clients! Make sure there are no
//! instances or clients spawned before mutating.

use std::ops::Index;

use anyhow::{bail, Context};
use bevy_app::{CoreSet, Plugin, StartupSet};
use bevy_ecs::prelude::*;
use tracing::error;
use valence_core::ident;
use valence_core::ident::Ident;
use valence_core::text::Color;
use valence_nbt::{compound, Compound, List, Value};
use valence_registry::{RegistryCodec, RegistryCodecSet, RegistryValue};

pub(crate) struct ChatTypePlugin;

impl Plugin for ChatTypePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(ChatTypeRegistry {
            id_to_chat_type: vec![],
        })
        .add_systems(
            (update_chat_type_registry, remove_chat_types_from_registry)
                .chain()
                .in_base_set(CoreSet::PostUpdate)
                .before(RegistryCodecSet),
        )
        .add_startup_system(load_default_chat_types.in_base_set(StartupSet::PreStartup));
    }
}

#[derive(Resource)]
pub struct ChatTypeRegistry {
    id_to_chat_type: Vec<Entity>,
}

impl ChatTypeRegistry {
    pub const KEY: Ident<&str> = ident!("minecraft:chat_type");

    pub fn get_by_id(&self, id: ChatTypeId) -> Option<Entity> {
        self.id_to_chat_type.get(id.0 as usize).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ChatTypeId, Entity)> + '_ {
        self.id_to_chat_type
            .iter()
            .enumerate()
            .map(|(id, chat_type)| (ChatTypeId(id as _), *chat_type))
    }
}

impl Index<ChatTypeId> for ChatTypeRegistry {
    type Output = Entity;

    fn index(&self, index: ChatTypeId) -> &Self::Output {
        self.id_to_chat_type
            .get(index.0 as usize)
            .unwrap_or_else(|| panic!("invalid {index:?}"))
    }
}

/// An index into the chat type registry
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct ChatTypeId(pub u16);

/// Contains information about how chat is styled, such as the chat color. The
/// notchian server has different chat types for team chat and direct messages.
///
/// Note that [`ChatTypeDecoration::style`] for [`ChatType::narration`]
/// is unused by the notchian client and is ignored.
#[derive(Component, Clone, Debug)]
pub struct ChatType {
    pub name: Ident<String>,
    pub chat: ChatTypeDecoration,
    pub narration: ChatTypeDecoration,
}

impl Default for ChatType {
    fn default() -> Self {
        Self {
            name: ident!("chat").into(),
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

fn load_default_chat_types(
    mut reg: ResMut<ChatTypeRegistry>,
    codec: Res<RegistryCodec>,
    mut commands: Commands,
) {
    let mut helper = move || {
        for value in codec.registry(ChatTypeRegistry::KEY) {
            let Some(Value::Compound(chat)) = value.element.get("chat") else {
                bail!("missing chat type text decorations")
            };

            let chat_key = chat
                .get("translation_key")
                .and_then(|v| v.as_string())
                .context("invalid translation_key")?
                .clone();

            let chat_parameters =
                if let Some(Value::List(List::String(params))) = chat.get("parameters") {
                    ChatTypeParameters {
                        sender: params.contains(&String::from("sender")),
                        target: params.contains(&String::from("target")),
                        content: params.contains(&String::from("content")),
                    }
                } else {
                    bail!("missing chat type text parameters")
                };

            let Some(Value::Compound(narration)) = value.element.get("narration") else {
                bail!("missing chat type text narration decorations")
            };

            let narration_key = narration
                .get("translation_key")
                .and_then(|v| v.as_string())
                .context("invalid translation_key")?
                .clone();

            let narration_parameters =
                if let Some(Value::List(List::String(params))) = chat.get("parameters") {
                    ChatTypeParameters {
                        sender: params.contains(&String::from("sender")),
                        target: params.contains(&String::from("target")),
                        content: params.contains(&String::from("content")),
                    }
                } else {
                    bail!("missing chat type narration parameters")
                };

            let entity = commands
                .spawn(ChatType {
                    name: value.name.clone(),
                    chat: ChatTypeDecoration {
                        translation_key: chat_key,
                        // TODO: Add support for the chat type styling
                        style: None,
                        parameters: chat_parameters,
                    },
                    narration: ChatTypeDecoration {
                        translation_key: narration_key,
                        style: None,
                        parameters: narration_parameters,
                    },
                })
                .id();

            reg.id_to_chat_type.push(entity);
        }

        Ok(())
    };

    if let Err(e) = helper() {
        error!("failed to load default chat types from registry codec: {e:#}");
    }
}

/// Add new chat types to or update existing chat types in the registry.
fn update_chat_type_registry(
    mut reg: ResMut<ChatTypeRegistry>,
    mut codec: ResMut<RegistryCodec>,
    chat_types: Query<(Entity, &ChatType), Changed<ChatType>>,
) {
    for (entity, chat_type) in &chat_types {
        let chat_type_registry = codec.registry_mut(ChatTypeRegistry::KEY);

        let mut chat_text_compound = compound! {
            "translation_key" => &chat_type.chat.translation_key,
            "parameters" => {
                let mut parameters = Vec::new();
                if chat_type.chat.parameters.sender {
                    parameters.push("sender".to_string());
                }
                if chat_type.chat.parameters.target {
                    parameters.push("target".to_string());
                }
                if chat_type.chat.parameters.content {
                    parameters.push("content".to_string());
                }
                List::String(parameters)
            },
        };

        if let Some(style) = &chat_type.chat.style {
            let mut s = Compound::new();
            if let Some(color) = style.color {
                s.insert(
                    "color",
                    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b),
                );
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
            chat_text_compound.insert("style", s);
        }

        let chat_narration_compound = compound! {
            "translation_key" => &chat_type.narration.translation_key,
            "parameters" => {
                let mut parameters = Vec::new();
                if chat_type.narration.parameters.sender {
                    parameters.push("sender".to_string());
                }
                if chat_type.narration.parameters.target {
                    parameters.push("target".to_string());
                }
                if chat_type.narration.parameters.content {
                    parameters.push("content".to_string());
                }
                List::String(parameters)
            },
        };

        let chat_type_compound = compound! {
            "chat" => chat_text_compound,
            "narration" => chat_narration_compound,
        };

        if let Some(value) = chat_type_registry
            .iter_mut()
            .find(|v| v.name == chat_type.name)
        {
            value.name = chat_type.name.clone();
            value.element.merge(chat_type_compound);
        } else {
            chat_type_registry.push(RegistryValue {
                name: chat_type.name.clone(),
                element: chat_type_compound,
            });
            reg.id_to_chat_type.push(entity);
        }
    }
}

/// Remove deleted chat types from the registry.
fn remove_chat_types_from_registry(
    mut chat_types: RemovedComponents<ChatType>,
    mut reg: ResMut<ChatTypeRegistry>,
    mut codec: ResMut<RegistryCodec>,
) {
    for chat_type in chat_types.iter() {
        if let Some(idx) = reg
            .id_to_chat_type
            .iter()
            .position(|entity| *entity == chat_type)
        {
            reg.id_to_chat_type.remove(idx);
            codec.registry_mut(ChatTypeRegistry::KEY).remove(idx);
        }
    }
}
