//! The player list (tab list).

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use bitfield_struct::bitfield;
use uuid::Uuid;

use crate::client::GameMode;
use crate::config::Config;
use crate::player_textures::SignedPlayerTextures;
use crate::protocol::packets::s2c::play::{
    PlayerInfo, PlayerListAddPlayer, S2cPlayPacket, SetTabListHeaderAndFooter,
};
use crate::protocol::packets::Property;
use crate::protocol::VarInt;
use crate::slab_rc::{Key, SlabRc};
use crate::text::Text;

/// A container for all [`PlayerList`]s on a server.
pub struct PlayerLists<C: Config> {
    slab: SlabRc<PlayerList<C>>,
}

/// An identifier for a [`PlayerList`] on the server.
///
/// Player list IDs are refcounted. Once all IDs referring to the same player
/// list are dropped, the player list is automatically deleted.
///
/// The [`Ord`] instance on this type is correct but otherwise unspecified. This
/// is useful for storing IDs in containers such as
/// [`BTreeMap`](std::collections::BTreeMap).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PlayerListId(Key);

impl<C: Config> PlayerLists<C> {
    pub(crate) fn new() -> Self {
        Self {
            slab: SlabRc::new(),
        }
    }

    /// Creates a new player list and returns an exclusive reference to it along
    /// with its ID.
    ///
    /// The player list is automatically removed at the end of the tick once all
    /// IDs to it have been dropped.
    pub fn insert(&mut self, state: C::PlayerListState) -> (PlayerListId, &mut PlayerList<C>) {
        let (key, pl) = self.slab.insert(PlayerList {
            state,
            entries: HashMap::new(),
            removed: HashSet::new(),
            header: Text::default(),
            footer: Text::default(),
            modified_header_or_footer: false,
        });

        (PlayerListId(key), pl)
    }

    /// Returns the number of player lists.
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    /// Returns `true` if there are no player lists.
    pub fn is_empty(&self) -> bool {
        self.slab.len() == 0
    }

    /// Gets a shared reference to the player list with the given player list
    /// ID.
    ///
    /// This operation is infallible because [`PlayerListId`] is refcounted.
    pub fn get(&self, id: &PlayerListId) -> &PlayerList<C> {
        self.slab.get(&id.0)
    }

    /// Gets an exclusive reference to the player list with the given player
    /// list ID.
    ///
    /// This operation is infallible because [`PlayerListId`] is refcounted.
    pub fn get_mut(&mut self, id: &PlayerListId) -> &mut PlayerList<C> {
        self.slab.get_mut(&id.0)
    }

    pub(crate) fn update(&mut self) {
        self.slab.collect_garbage();
        for (_, pl) in self.slab.iter_mut() {
            for entry in pl.entries.values_mut() {
                entry.bits = EntryBits::new();
            }
            pl.removed.clear();
            pl.modified_header_or_footer = false;
        }
    }
}

/// The list of players on a server visible by pressing the tab key by default.
///
/// Each entry in the player list is intended to represent a connected client to
/// the server.
///
/// In addition to a list of players, the player list has a header and a footer
/// which can contain arbitrary text.
pub struct PlayerList<C: Config> {
    /// Custom state
    pub state: C::PlayerListState,
    entries: HashMap<Uuid, PlayerListEntry>,
    removed: HashSet<Uuid>,
    header: Text,
    footer: Text,
    modified_header_or_footer: bool,
}

impl<C: Config> PlayerList<C> {
    /// Inserts a player into the player list.
    ///
    /// If the given UUID conflicts with an existing entry, the entry is
    /// overwritten and `false` is returned. Otherwise, `true` is returned.
    pub fn insert(
        &mut self,
        uuid: Uuid,
        username: impl Into<String>,
        textures: Option<SignedPlayerTextures>,
        game_mode: GameMode,
        ping: i32,
        display_name: impl Into<Option<Text>>,
    ) -> bool {
        match self.entries.entry(uuid) {
            Entry::Occupied(mut oe) => {
                let e = oe.get_mut();
                let username = username.into();

                if e.username() != username || e.textures != textures {
                    self.removed.insert(*oe.key());

                    oe.insert(PlayerListEntry {
                        username,
                        textures,
                        game_mode,
                        ping,
                        display_name: display_name.into(),
                        bits: EntryBits::new().with_created_this_tick(true),
                    });
                } else {
                    e.set_game_mode(game_mode);
                    e.set_ping(ping);
                    e.set_display_name(display_name);
                }
                false
            }
            Entry::Vacant(ve) => {
                ve.insert(PlayerListEntry {
                    username: username.into(),
                    textures,
                    game_mode,
                    ping,
                    display_name: display_name.into(),
                    bits: EntryBits::new().with_created_this_tick(true),
                });
                true
            }
        }
    }

    /// Removes an entry from the player list with the given UUID. Returns
    /// whether the entry was present in the list.
    pub fn remove(&mut self, uuid: Uuid) -> bool {
        if self.entries.remove(&uuid).is_some() {
            self.removed.insert(uuid);
            true
        } else {
            false
        }
    }

    /// Removes all entries from the player list for which `f` returns `false`.
    ///
    /// All entries are visited in an unspecified order.
    pub fn retain(&mut self, mut f: impl FnMut(Uuid, &mut PlayerListEntry) -> bool) {
        self.entries.retain(|&uuid, entry| {
            if !f(uuid, entry) {
                self.removed.insert(uuid);
                false
            } else {
                true
            }
        })
    }

    /// Removes all entries from the player list.
    pub fn clear(&mut self) {
        self.removed.extend(self.entries.drain().map(|p| p.0))
    }

    /// Gets the header part of the player list.
    pub fn header(&self) -> &Text {
        &self.header
    }

    /// Sets the header part of the player list.
    pub fn set_header(&mut self, header: impl Into<Text>) {
        let header = header.into();
        if self.header != header {
            self.header = header;
            self.modified_header_or_footer = true;
        }
    }

    /// Gets the footer part of the player list.
    pub fn footer(&self) -> &Text {
        &self.footer
    }

    /// Sets the footer part of the player list.
    pub fn set_footer(&mut self, footer: impl Into<Text>) {
        let footer = footer.into();
        if self.footer != footer {
            self.footer = footer;
            self.modified_header_or_footer = true;
        }
    }

    /// Returns an iterator over all entries in an unspecified order.
    pub fn entries(&self) -> impl Iterator<Item = (Uuid, &PlayerListEntry)> + '_ {
        self.entries.iter().map(|(k, v)| (*k, v))
    }

    /// Returns a mutable iterator over all entries in an unspecified order.
    pub fn entries_mut(&mut self) -> impl Iterator<Item = (Uuid, &mut PlayerListEntry)> + '_ {
        self.entries.iter_mut().map(|(k, v)| (*k, v))
    }

    pub(crate) fn initial_packets(&self, mut push_packet: impl FnMut(S2cPlayPacket)) {
        let add_player: Vec<_> = self
            .entries
            .iter()
            .map(|(&uuid, e)| PlayerListAddPlayer {
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
            push_packet(PlayerInfo::AddPlayer(add_player).into());
        }

        if self.header != Text::default() || self.footer != Text::default() {
            push_packet(
                SetTabListHeaderAndFooter {
                    header: self.header.clone(),
                    footer: self.footer.clone(),
                }
                .into(),
            );
        }
    }

    pub(crate) fn update_packets(&self, mut push_packet: impl FnMut(S2cPlayPacket)) {
        if !self.removed.is_empty() {
            push_packet(PlayerInfo::RemovePlayer(self.removed.iter().cloned().collect()).into());
        }

        let mut add_player = Vec::new();
        let mut game_mode = Vec::new();
        let mut ping = Vec::new();
        let mut display_name = Vec::new();

        for (&uuid, e) in self.entries.iter() {
            if e.bits.created_this_tick() {
                let mut properties = Vec::new();
                if let Some(textures) = &e.textures {
                    properties.push(Property {
                        name: "textures".into(),
                        value: base64::encode(textures.payload()),
                        signature: Some(base64::encode(textures.signature())),
                    });
                }

                add_player.push(PlayerListAddPlayer {
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

            if e.bits.modified_game_mode() {
                game_mode.push((uuid, e.game_mode));
            }

            if e.bits.modified_ping() {
                ping.push((uuid, VarInt(e.ping)));
            }

            if e.bits.modified_display_name() {
                display_name.push((uuid, e.display_name.clone()));
            }
        }

        if !add_player.is_empty() {
            push_packet(PlayerInfo::AddPlayer(add_player).into());
        }

        if !game_mode.is_empty() {
            push_packet(PlayerInfo::UpdateGameMode(game_mode).into());
        }

        if !ping.is_empty() {
            push_packet(PlayerInfo::UpdateLatency(ping).into());
        }

        if !display_name.is_empty() {
            push_packet(PlayerInfo::UpdateDisplayName(display_name).into());
        }

        if self.modified_header_or_footer {
            push_packet(
                SetTabListHeaderAndFooter {
                    header: self.header.clone(),
                    footer: self.footer.clone(),
                }
                .into(),
            );
        }
    }

    pub(crate) fn clear_packets(&self, mut push_packet: impl FnMut(S2cPlayPacket)) {
        push_packet(PlayerInfo::RemovePlayer(self.entries.keys().cloned().collect()).into());
    }
}

/// Represents a player entry in the [`PlayerList`].
pub struct PlayerListEntry {
    username: String,
    textures: Option<SignedPlayerTextures>,
    game_mode: GameMode,
    ping: i32,
    display_name: Option<Text>,
    bits: EntryBits,
}

#[bitfield(u8)]
struct EntryBits {
    created_this_tick: bool,
    modified_game_mode: bool,
    modified_ping: bool,
    modified_display_name: bool,
    #[bits(4)]
    _pad: u8,
}

impl PlayerListEntry {
    /// Gets the username of this entry.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Gets the player textures for this entry.
    pub fn textures(&self) -> Option<&SignedPlayerTextures> {
        self.textures.as_ref()
    }

    /// Gets the game mode of this entry.
    pub fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    /// Sets the game mode of this entry.
    pub fn set_game_mode(&mut self, game_mode: GameMode) {
        if self.game_mode != game_mode {
            self.game_mode = game_mode;
            self.bits.set_modified_game_mode(true);
        }
    }

    /// Gets the ping (latency) of this entry measured in milliseconds.
    pub fn ping(&self) -> i32 {
        self.ping
    }

    /// Sets the ping (latency) of this entry measured in milliseconds.
    pub fn set_ping(&mut self, ping: i32) {
        if self.ping != ping {
            self.ping = ping;
            self.bits.set_modified_ping(true);
        }
    }

    /// Gets the display name of this entry.
    pub fn display_name(&self) -> Option<&Text> {
        self.display_name.as_ref()
    }

    /// Sets the display name of this entry.
    pub fn set_display_name(&mut self, display_name: impl Into<Option<Text>>) {
        let display_name = display_name.into();
        if self.display_name != display_name {
            self.display_name = display_name;
            self.bits.set_modified_display_name(true);
        }
    }
}
