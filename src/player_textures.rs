use anyhow::Context;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, PartialEq, Debug)]
pub struct SignedPlayerTextures {
    payload: Box<[u8]>,
    signature: Box<[u8]>,
}

impl SignedPlayerTextures {
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

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

#[derive(Clone, PartialEq, Default, Debug)]
pub struct PlayerTextures {
    pub skin: Option<Url>,
    pub cape: Option<Url>,
}

impl From<SignedPlayerTextures> for PlayerTextures {
    fn from(spt: SignedPlayerTextures) -> Self {
        spt.to_textures()
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct PlayerTexturesPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    skin: Option<TextureUrl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cape: Option<TextureUrl>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
struct TextureUrl {
    url: Url,
}
