use base64::prelude::*;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Decode, Encode};

/// A property from the game profile.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct Property<S = String> {
    pub name: S,
    pub value: S,
    pub signature: Option<S>,
}

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
    /// Constructs player textures from the "textures" property of the game
    /// profile.
    ///
    /// "textures" is a base64 string of JSON data.
    pub fn try_from_textures(textures: &str) -> anyhow::Result<Self> {
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

        let decoded = BASE64_STANDARD.decode(textures.as_bytes())?;

        let Textures { textures } = serde_json::from_slice(&decoded)?;

        Ok(Self {
            skin: textures.skin.url,
            cape: textures.cape.map(|t| t.url),
        })
    }
}
