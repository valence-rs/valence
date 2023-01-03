//! Player skins and capes.

use serde::Deserialize;
use url::Url;

/// Contains URLs to the skin and cape of a player.
///
/// This data has been cryptographically signed to ensure it will not be altered
/// by the server.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SignedPlayerTextures {
    payload: Box<str>,
    signature: Box<str>,
    skin_url: Box<str>,
    cape_url: Option<Box<str>>,
}

impl SignedPlayerTextures {
    /// Constructs the signed player textures from payload and signature
    /// components in base64.
    ///
    /// Note that this does not validate that the signature is valid for the
    /// given payload.
    pub(crate) fn from_base64(
        payload: impl Into<Box<str>>,
        signature: impl Into<Box<str>>,
    ) -> anyhow::Result<Self> {
        let payload = payload.into();
        let signature = signature.into();

        let payload_decoded = base64::decode(payload.as_bytes())?;
        base64::decode(signature.as_bytes())?;

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

        let textures: Textures = serde_json::from_slice(&payload_decoded)?;

        Ok(Self {
            payload,
            signature,
            skin_url: String::from(textures.textures.skin.url).into(),
            cape_url: textures.textures.cape.map(|t| String::from(t.url).into()),
        })
    }

    /// The payload in base64.
    pub(crate) fn payload(&self) -> &str {
        &self.payload
    }

    /// The signature in base64.
    pub(crate) fn signature(&self) -> &str {
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
