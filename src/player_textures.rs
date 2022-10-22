//! Player skins and capes.

use serde::Deserialize;
use url::Url;

/// Contains URLs to the skin and cape of a player.
///
/// This data has been cryptographically signed to ensure it will not be altered
/// by the server.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SignedPlayerTextures {
    payload: Box<[u8]>,
    signature: Box<[u8]>,
    skin_url: Box<str>,
    cape_url: Option<Box<str>>,
}

impl SignedPlayerTextures {
    pub(crate) fn from_base64(
        payload: impl AsRef<str>,
        signature: impl AsRef<str>,
    ) -> anyhow::Result<Self> {
        let payload = base64::decode(payload.as_ref())?;
        let signature = base64::decode(signature.as_ref())?;

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

        let textures: Textures = serde_json::from_slice(&payload)?;

        Ok(Self {
            payload: payload.into(),
            signature: signature.into(),
            skin_url: String::from(textures.textures.skin.url).into(),
            cape_url: textures.textures.cape.map(|t| String::from(t.url).into()),
        })
    }

    pub(crate) fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub(crate) fn signature(&self) -> &[u8] {
        &self.signature
    }

    /// Returns the URL to the texture's skin as a `str`.
    ///
    /// The returned string is guaranteed to be a valid URL.
    pub fn skin(&self) -> &str {
        &self.skin_url
    }

    /// Returns the URL to the texture's cape as a `str` if present.
    ///
    /// The returned string is guaranteed to be a valid URL. `None` is returned
    /// instead if there is no cape.
    pub fn cape(&self) -> Option<&str> {
        self.cape_url.as_deref()
    }
}
