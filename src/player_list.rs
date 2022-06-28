use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::client::GameMode;
use crate::packets::login::s2c::Property;
use crate::packets::play::s2c::{
    PlayerInfo, PlayerInfoAddPlayer, PlayerListHeaderFooter, S2cPlayPacket,
};
use crate::var_int::VarInt;
use crate::Text;

pub struct PlayerList {
    entries: HashMap<Uuid, PlayerListEntry>,
    removed: HashSet<Uuid>,
    header: Text,
    footer: Text,
    modified_header_or_footer: bool,
}

impl PlayerList {
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
            removed: HashSet::new(),
            header: Text::default(),
            footer: Text::default(),
            modified_header_or_footer: false,
        }
    }

    pub fn header(&self) -> &Text {
        &self.header
    }

    pub fn footer(&self) -> &Text {
        &self.footer
    }

    pub fn entries(&self) -> impl Iterator<Item = (Uuid, &PlayerListEntry)> + '_ {
        self.entries.iter().map(|(k, v)| (*k, v))
    }

    pub(crate) fn initial_packets(&self, mut packet: impl FnMut(S2cPlayPacket)) {
        let add_player: Vec<_> = self
            .entries
            .iter()
            .map(|(&uuid, e)| PlayerInfoAddPlayer {
                uuid,
                username: e.username.clone().into(),
                properties: {
                    let mut properties = Vec::new();
                    if e.skin().is_some() || e.cape().is_some() {
                        let textures = PlayerTextures {
                            skin: e.skin().cloned().map(TextureUrl::new),
                            cape: e.cape().cloned().map(TextureUrl::new),
                        };

                        properties.push(Property {
                            name: "textures".into(),
                            value: base64::encode(serde_json::to_string(&textures).unwrap()),
                            signature: None,
                        });
                    }
                    properties
                },
                game_mode: e.game_mode,
                ping: VarInt(e.ping),
                display_name: e.display_name.clone(),
                sig_data: None,
            })
            .collect();

        if !add_player.is_empty() {
            packet(PlayerInfo::AddPlayer(add_player).into());
        }

        if self.header != Text::default() || self.footer != Text::default() {
            packet(
                PlayerListHeaderFooter {
                    header: self.header.clone(),
                    footer: self.footer.clone(),
                }
                .into(),
            );
        }
    }

    pub(crate) fn packets(&self, mut packet: impl FnMut(S2cPlayPacket)) {
        if !self.removed.is_empty() {
            packet(PlayerInfo::RemovePlayer(self.removed.iter().cloned().collect()).into());
        }

        let mut add_player = Vec::new();
        let mut game_mode = Vec::new();
        let mut ping = Vec::new();
        let mut display_name = Vec::new();

        for (&uuid, e) in self.entries.iter() {
            if e.flags.created_this_tick() {
                let mut properties = Vec::new();
                if e.skin().is_some() || e.cape().is_some() {
                    let textures = PlayerTextures {
                        skin: e.skin().cloned().map(TextureUrl::new),
                        cape: e.cape().cloned().map(TextureUrl::new),
                    };

                    properties.push(Property {
                        name: "textures".into(),
                        value: base64::encode(serde_json::to_string(&textures).unwrap()),
                        signature: None,
                    });
                }

                add_player.push(PlayerInfoAddPlayer {
                    uuid,
                    username: e.username.clone().into(),
                    properties,
                    game_mode: e.game_mode,
                    ping: VarInt(e.ping),
                    display_name: e.display_name.clone(),
                    sig_data: None,
                });

                continue;
            }

            if e.flags.modified_game_mode() {
                game_mode.push((uuid, e.game_mode));
            }

            if e.flags.modified_ping() {
                ping.push((uuid, VarInt(e.ping)));
            }

            if e.flags.modified_display_name() {
                display_name.push((uuid, e.display_name.clone()));
            }
        }

        if !add_player.is_empty() {
            packet(PlayerInfo::AddPlayer(add_player).into());
        }

        if !game_mode.is_empty() {
            packet(PlayerInfo::UpdateGameMode(game_mode).into());
        }

        if !ping.is_empty() {
            packet(PlayerInfo::UpdateLatency(ping).into());
        }

        if !display_name.is_empty() {
            packet(PlayerInfo::UpdateDisplayName(display_name).into());
        }

        if self.modified_header_or_footer {
            packet(
                PlayerListHeaderFooter {
                    header: self.header.clone(),
                    footer: self.footer.clone(),
                }
                .into(),
            );
        }
    }
}

pub struct PlayerListMut<'a>(pub(crate) &'a mut PlayerList);

impl<'a> Deref for PlayerListMut<'a> {
    type Target = PlayerList;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> PlayerListMut<'a> {
    pub fn reborrow(&mut self) -> PlayerListMut {
        PlayerListMut(self.0)
    }

    pub fn add_player(
        &mut self,
        uuid: Uuid,
        username: impl Into<String>,
        skin: Option<Url>,
        cape: Option<Url>,
        game_mode: GameMode,
        ping: i32,
        display_name: impl Into<Option<Text>>,
    ) {
        match self.0.entries.entry(uuid) {
            Entry::Occupied(mut oe) => {
                let mut entry = PlayerListEntryMut(oe.get_mut());
                let username = username.into();

                if entry.username() != username
                    || entry.skin() != skin.as_ref()
                    || entry.cape() != cape.as_ref()
                {
                    self.0.removed.insert(*oe.key());

                    oe.insert(PlayerListEntry {
                        username,
                        textures: PlayerTextures {
                            skin: skin.map(TextureUrl::new),
                            cape: cape.map(TextureUrl::new),
                        },
                        game_mode,
                        ping,
                        display_name: display_name.into(),
                        flags: EntryFlags::new().with_created_this_tick(true),
                    });
                } else {
                    entry.set_game_mode(game_mode);
                    entry.set_ping(ping);
                    entry.set_display_name(display_name);
                }
            }
            Entry::Vacant(ve) => {
                ve.insert(PlayerListEntry {
                    username: username.into(),
                    textures: PlayerTextures {
                        skin: skin.map(TextureUrl::new),
                        cape: cape.map(TextureUrl::new),
                    },
                    game_mode,
                    ping,
                    display_name: display_name.into(),
                    flags: EntryFlags::new().with_created_this_tick(true),
                });
            }
        }
    }

    pub fn remove_player(&mut self, uuid: Uuid) -> bool {
        if self.0.entries.remove(&uuid).is_some() {
            self.0.removed.insert(uuid);
            true
        } else {
            false
        }
    }

    pub fn set_header(&mut self, header: impl Into<Text>) {
        let header = header.into();
        if self.0.header != header {
            self.0.header = header;
            self.0.modified_header_or_footer = true;
        }
    }

    pub fn set_footer(&mut self, footer: impl Into<Text>) {
        let footer = footer.into();
        if self.0.footer != footer {
            self.0.footer = footer;
            self.0.modified_header_or_footer = true;
        }
    }

    pub fn entries_mut(&mut self) -> impl Iterator<Item = (Uuid, PlayerListEntryMut)> + '_ {
        self.0
            .entries
            .iter_mut()
            .map(|(k, v)| (*k, PlayerListEntryMut(v)))
    }

    pub(crate) fn update(&mut self) {
        for e in self.0.entries.values_mut() {
            e.flags = EntryFlags(0);
        }
        self.0.removed.clear();
        self.0.modified_header_or_footer = false;
    }
}

pub struct PlayerListEntry {
    username: String,
    textures: PlayerTextures,
    game_mode: GameMode,
    ping: i32,
    display_name: Option<Text>,
    flags: EntryFlags,
}

impl PlayerListEntry {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn skin(&self) -> Option<&Url> {
        self.textures.skin.as_ref().map(|t| &t.url)
    }

    pub fn cape(&self) -> Option<&Url> {
        self.textures.cape.as_ref().map(|t| &t.url)
    }

    pub fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    pub fn ping(&self) -> i32 {
        self.ping
    }

    pub fn display_name(&self) -> Option<&Text> {
        self.display_name.as_ref()
    }
}

pub struct PlayerListEntryMut<'a>(&'a mut PlayerListEntry);

impl<'a> Deref for PlayerListEntryMut<'a> {
    type Target = PlayerListEntry;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> PlayerListEntryMut<'a> {
    pub fn reborrow(&mut self) -> PlayerListEntryMut {
        PlayerListEntryMut(self.0)
    }

    pub fn set_game_mode(&mut self, game_mode: GameMode) {
        if self.0.game_mode != game_mode {
            self.0.game_mode = game_mode;
            self.0.flags.set_modified_game_mode(true);
        }
    }

    pub fn set_ping(&mut self, ping: i32) {
        if self.0.ping != ping {
            self.0.ping = ping;
            self.0.flags.set_modified_ping(true);
        }
    }

    pub fn set_display_name(&mut self, display_name: impl Into<Option<Text>>) {
        let display_name = display_name.into();
        if self.0.display_name != display_name {
            self.0.display_name = display_name;
            self.0.flags.set_modified_display_name(true);
        }
    }
}

#[bitfield(u8)]
struct EntryFlags {
    created_this_tick: bool,
    modified_game_mode: bool,
    modified_ping: bool,
    modified_display_name: bool,
    #[bits(4)]
    _pad: u8,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) struct PlayerTextures {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    skin: Option<TextureUrl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cape: Option<TextureUrl>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
struct TextureUrl {
    url: Url,
}

impl TextureUrl {
    fn new(url: Url) -> Self {
        Self { url }
    }
}
