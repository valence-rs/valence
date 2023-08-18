use std::collections::BTreeMap;
use std::io::Write;

use anyhow::Context;
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{Bounded, Decode, Encode};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct GameProfile<S: Ord = String> {
    /// UUID of the player.
    pub id: Uuid,
    /// Player username.
    pub name: S,
    /// Player properties. This often contains "textures" which hold the
    /// player's skin and cape.
    pub properties: PropertyMap<S>,
}

impl<S> Encode for GameProfile<S>
where
    S: AsRef<str> + Ord + Encode,
{
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.id.encode(&mut w)?;
        Bounded::<_, 16>(self.name.as_ref()).encode(&mut w)?;
        self.properties.encode(w)
    }
}

impl<'a, S> Decode<'a> for GameProfile<S>
where
    S: Decode<'a> + Ord + From<&'a str>,
{
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(Self {
            id: Decode::decode(r)?,
            name: Bounded::<&str, 16>::decode(r)?.0.into(),
            properties: Decode::decode(r)?,
        })
    }
}

/// Maps property names to property values.
pub type PropertyMap<S = String> = BTreeMap<S, PropertyValue<S>>;

/// Property values from a player's game profile.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Serialize, Deserialize)]
pub struct PropertyValue<S = String> {
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
    pub fn try_from_properties(props: &PropertyMap) -> anyhow::Result<Self> {
        let textures = props
            .get("textures")
            .context("no textures in propery map")?;

        Self::try_from_textures(&textures.value)
    }

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
