//! Player skins and capes.

use anyhow::Context;
use serde::{Deserialize, Serialize};
use url::Url;

/// Contains URLs to the skin and cape of a player.
///
/// This data has been cryptographically signed to ensure it will not be altered
/// by the server.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SignedPlayerTextures {
    payload: Box<[u8]>,
    signature: Box<[u8]>,
}

impl SignedPlayerTextures {
    pub(crate) fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub(crate) fn signature(&self) -> &[u8] {
        &self.signature
    }

    /// Gets the unsigned texture URLs.
    pub fn to_textures(&self) -> PlayerTextures {
        self.to_textures_fallible()
            .expect("payload should have been validated earlier")
    }

    fn to_textures_fallible(&self) -> anyhow::Result<PlayerTextures> {
        #[derive(Debug, Deserialize)]
        struct Textures {
            textures: PlayerTexturesPayload,
        }

        let textures: Textures = serde_json::from_slice(&self.payload)?;

        Ok(PlayerTextures {
            skin: textures.textures.skin.map(|t| t.url),
            cape: textures.textures.cape.map(|t| t.url),
        })
    }

    pub(crate) fn from_base64(payload: String, signature: String) -> anyhow::Result<Self> {
        let res = Self {
            payload: base64::decode(payload)?.into_boxed_slice(),
            signature: base64::decode(signature)?.into_boxed_slice(),
        };

        match res.to_textures_fallible() {
            Ok(_) => Ok(res),
            Err(e) => Err(e).context("failed to parse textures payload"),
        }
    }
}

/// Contains URLs to the skin and cape of a player.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct PlayerTextures {
    /// A URL to the skin of a player. Is `None` if the player does not have a
    /// skin.
    pub skin: Option<Url>,
    /// A URL to the cape of a player. Is `None` if the player does not have a
    /// cape.
    pub cape: Option<Url>,
}

impl From<SignedPlayerTextures> for PlayerTextures {
    fn from(spt: SignedPlayerTextures) -> Self {
        spt.to_textures()
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct PlayerTexturesPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    skin: Option<TextureUrl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cape: Option<TextureUrl>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct TextureUrl {
    url: Url,
}
