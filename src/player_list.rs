use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use bitfield_struct::bitfield;
use uuid::Uuid;

use crate::client::GameMode;
use crate::packets::play::s2c::{
    PlayerInfo, PlayerInfoAddPlayer, PlayerListHeaderFooter, S2cPlayPacket,
};
use crate::packets::Property;
use crate::player_textures::SignedPlayerTextures;
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
                    if let Some(textures) = &e.textures {
                        properties.push(Property {
                            name: "textures".into(),
                            value: base64::encode(textures.payload()),
                            signature: Some(base64::encode(textures.signature())),
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
                if let Some(textures) = &e.textures {
                    properties.push(Property {
                        name: "textures".into(),
                        value: base64::encode(textures.payload()),
                        signature: Some(base64::encode(textures.signature())),
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

    pub fn insert(
        &mut self,
        uuid: Uuid,
        username: impl Into<String>,
        textures: Option<SignedPlayerTextures>,
        game_mode: GameMode,
        ping: i32,
        display_name: impl Into<Option<Text>>,
    ) {
        match self.0.entries.entry(uuid) {
            Entry::Occupied(mut oe) => {
                let mut e = PlayerListEntryMut(oe.get_mut());
                let username = username.into();

                if e.username() != username || e.textures != textures {
                    self.0.removed.insert(*oe.key());

                    oe.insert(PlayerListEntry {
                        username,
                        textures,
                        game_mode,
                        ping,
                        display_name: display_name.into(),
                        flags: EntryFlags::new().with_created_this_tick(true),
                    });
                } else {
                    e.set_game_mode(game_mode);
                    e.set_ping(ping);
                    e.set_display_name(display_name);
                }
            }
            Entry::Vacant(ve) => {
                ve.insert(PlayerListEntry {
                    username: username.into(),
                    textures,
                    game_mode,
                    ping,
                    display_name: display_name.into(),
                    flags: EntryFlags::new().with_created_this_tick(true),
                });
            }
        }
    }

    pub fn remove(&mut self, uuid: Uuid) -> bool {
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
    textures: Option<SignedPlayerTextures>,
    game_mode: GameMode,
    ping: i32,
    display_name: Option<Text>,
    flags: EntryFlags,
}

impl PlayerListEntry {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn textures(&self) -> Option<&SignedPlayerTextures> {
        self.textures.as_ref()
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
