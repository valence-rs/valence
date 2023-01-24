//! Player skins and capes.

use anyhow::Context;
use serde::Deserialize;
use url::Url;
use valence_protocol::types::Property;
use base64::prelude::*;

/// Contains URLs to the skin and cape of a player.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlayerTextures {
    /// URL to the player's skin texture.
    pub skin: Url,
    /// URL to the player's cape texture. May be absent if the player does not
    /// have a cape.
    pub cape: Option<Url>,
}

impl PlayerTextures {
    pub fn from_properties(props: &[Property]) -> anyhow::Result<Self> {
        let textures = props
            .iter()
            .find(|p| p.name == "textures")
            .context("no textures in property list")?;

        #[derive(Debug, Deserialize)]
        struct Textures {
            textures: PlayerTexturesPayload,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "UPPERCASE")]
        struct PlayerTexturesPayload {
            skin: TextureUrl,
            #[serde(default)]
            cape: Option<TextureUrl>,
        }

        #[derive(Debug, Deserialize)]
        struct TextureUrl {
            url: Url,
        }

        let decoded = BASE64_STANDARD.decode(textures.value.as_bytes())?;

        let Textures { textures } = serde_json::from_slice(&decoded)?;

        Ok(Self {
            skin: textures.skin.url,
            cape: textures.cape.map(|t| t.url),
        })
    }
}
