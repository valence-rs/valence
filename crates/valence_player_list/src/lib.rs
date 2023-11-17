#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]
#![allow(clippy::type_complexity)]

use std::borrow::Cow;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use derive_more::{Deref, DerefMut};
use rsa::RsaPublicKey;
use valence_server::client::{Client, Properties, Username};
use valence_server::keepalive::Ping;
use valence_server::layer::UpdateLayersPreClientSet;
use valence_server::protocol::encode::PacketWriter;
use valence_server::protocol::packets::play::player_session_c2s::PlayerSessionData;
use valence_server::protocol::packets::play::{
    player_list_s2c as packet, PlayerListHeaderS2c, PlayerListS2c, PlayerRemoveS2c,
};
use valence_server::protocol::WritePacket;
use valence_server::text::IntoText;
use valence_server::uuid::Uuid;
use valence_server::{Despawned, GameMode, Server, Text, UniqueId};

pub struct PlayerListPlugin;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct PlayerListSet;

impl Plugin for PlayerListPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerList::new())
            .configure_set(
                PostUpdate,
                // Needs to happen before player entities are initialized. Otherwise, they will
                // appear invisible.
                PlayerListSet.before(UpdateLayersPreClientSet),
            )
            .add_systems(
                PostUpdate,
                (
                    update_header_footer,
                    add_new_clients_to_player_list,
                    apply_deferred, // So new clients get the packets for their own entry.
                    update_entries,
                    init_player_list_for_clients,
                    remove_despawned_entries,
                    write_player_list_changes,
                )
                    .in_set(PlayerListSet)
                    .chain(),
            );
    }
}

#[derive(Resource)]
pub struct PlayerList {
    cached_update_packets: Vec<u8>,
    header: Text,
    footer: Text,
    changed_header_or_footer: bool,
    /// If clients should be automatically added and removed from the player
    /// list with the proper components inserted. Enabled by default.
    pub manage_clients: bool,
}

impl PlayerList {
    fn new() -> Self {
        Self {
            cached_update_packets: vec![],
            header: Text::default(),
            footer: Text::default(),
            changed_header_or_footer: false,
            manage_clients: true,
        }
    }

    pub fn header(&self) -> &Text {
        &self.header
    }

    pub fn footer(&self) -> &Text {
        &self.footer
    }

    pub fn set_header<'a>(&mut self, txt: impl IntoText<'a>) {
        let txt = txt.into_cow_text().into_owned();

        if txt != self.header {
            self.changed_header_or_footer = true;
        }

        self.header = txt;
    }

    pub fn set_footer<'a>(&mut self, txt: impl IntoText<'a>) {
        let txt = txt.into_cow_text().into_owned();

        if txt != self.footer {
            self.changed_header_or_footer = true;
        }

        self.footer = txt;
    }
}

/// Bundle for spawning new player list entries. All components are required
/// unless otherwise stated.
///
/// # Despawning player list entries
///
/// The [`Despawned`] component must be used to despawn player list entries.
#[derive(Bundle, Default, Debug)]
pub struct PlayerListEntryBundle {
    pub player_list_entry: PlayerListEntry,
    /// Careful not to modify this!
    pub uuid: UniqueId,
    pub username: Username,
    pub properties: Properties,
    pub game_mode: GameMode,
    pub ping: Ping,
    pub display_name: DisplayName,
    pub listed: Listed,
}

/// Marker component for player list entries.
#[derive(Component, Default, Debug)]
pub struct PlayerListEntry;

/// Displayed name for a player list entry. Appears as [`Username`] if `None`.
#[derive(Component, Default, Debug, Deref, DerefMut)]
pub struct DisplayName(pub Option<Text>);

/// If a player list entry is visible. Defaults to `true`.
#[derive(Component, Copy, Clone, Debug, Deref, DerefMut)]
pub struct Listed(pub bool);

impl Default for Listed {
    fn default() -> Self {
        Self(true)
    }
}

/// Contains information for the player's chat message verification.
/// Not required.
#[derive(Component, Clone, Debug)]
pub struct ChatSession {
    pub public_key: RsaPublicKey,
    pub session_data: PlayerSessionData,
}

fn update_header_footer(player_list: ResMut<PlayerList>, server: Res<Server>) {
    if player_list.changed_header_or_footer {
        let player_list = player_list.into_inner();

        let mut w = PacketWriter::new(
            &mut player_list.cached_update_packets,
            server.compression_threshold(),
        );

        w.write_packet(&PlayerListHeaderS2c {
            header: (&player_list.header).into(),
            footer: (&player_list.footer).into(),
        });

        player_list.changed_header_or_footer = false;
    }
}

fn add_new_clients_to_player_list(
    clients: Query<Entity, Added<Client>>,
    player_list: Res<PlayerList>,
    mut commands: Commands,
) {
    if player_list.manage_clients {
        for entity in &clients {
            commands.entity(entity).insert((
                PlayerListEntry,
                DisplayName::default(),
                Listed::default(),
            ));
        }
    }
}

fn init_player_list_for_clients(
    mut clients: Query<&mut Client, (Added<Client>, Without<Despawned>)>,
    player_list: Res<PlayerList>,
    entries: Query<
        (
            &UniqueId,
            &Username,
            &Properties,
            &GameMode,
            &Ping,
            &DisplayName,
            &Listed,
            Option<&ChatSession>,
        ),
        With<PlayerListEntry>,
    >,
) {
    if player_list.manage_clients {
        for mut client in &mut clients {
            let actions = packet::PlayerListActions::new()
                .with_add_player(true)
                .with_update_game_mode(true)
                .with_update_listed(true)
                .with_update_latency(true)
                .with_update_display_name(true)
                .with_initialize_chat(true);

            let entries: Vec<_> = entries
                .iter()
                .map(
                    |(
                        uuid,
                        username,
                        props,
                        game_mode,
                        ping,
                        display_name,
                        listed,
                        chat_session,
                    )| {
                        packet::PlayerListEntry {
                            player_uuid: uuid.0,
                            username: &username.0,
                            properties: Cow::Borrowed(&props.0),
                            chat_data: chat_session.map(|s| s.session_data.clone().into()),
                            listed: listed.0,
                            ping: ping.0,
                            game_mode: *game_mode,
                            display_name: display_name.0.as_ref().map(Cow::Borrowed),
                        }
                    },
                )
                .collect();

            if !entries.is_empty() {
                client.write_packet(&PlayerListS2c {
                    actions,
                    entries: Cow::Owned(entries),
                });
            }

            if !player_list.header.is_empty() || !player_list.footer.is_empty() {
                client.write_packet(&PlayerListHeaderS2c {
                    header: Cow::Borrowed(&player_list.header),
                    footer: Cow::Borrowed(&player_list.footer),
                });
            }
        }
    }
}

fn remove_despawned_entries(
    entries: Query<&UniqueId, (Added<Despawned>, With<PlayerListEntry>)>,
    player_list: ResMut<PlayerList>,
    server: Res<Server>,
    mut removed: Local<Vec<Uuid>>,
) {
    if player_list.manage_clients {
        debug_assert!(removed.is_empty());

        removed.extend(entries.iter().map(|uuid| uuid.0));

        if !removed.is_empty() {
            let player_list = player_list.into_inner();

            let mut w = PacketWriter::new(
                &mut player_list.cached_update_packets,
                server.compression_threshold(),
            );

            w.write_packet(&PlayerRemoveS2c {
                uuids: Cow::Borrowed(&removed),
            });

            removed.clear();
        }
    }
}

fn update_entries(
    entries: Query<
        (
            Ref<UniqueId>,
            Ref<Username>,
            Ref<Properties>,
            Ref<GameMode>,
            Ref<Ping>,
            Ref<DisplayName>,
            Ref<Listed>,
            Option<Ref<ChatSession>>,
        ),
        (
            With<PlayerListEntry>,
            Or<(
                Changed<UniqueId>,
                Changed<Username>,
                Changed<Properties>,
                Changed<GameMode>,
                Changed<Ping>,
                Changed<DisplayName>,
                Changed<Listed>,
                Changed<ChatSession>,
            )>,
        ),
    >,
    server: Res<Server>,
    player_list: ResMut<PlayerList>,
) {
    let player_list = player_list.into_inner();

    let mut writer = PacketWriter::new(
        &mut player_list.cached_update_packets,
        server.compression_threshold(),
    );

    for (uuid, username, props, game_mode, ping, display_name, listed, chat_session) in &entries {
        let mut actions = packet::PlayerListActions::new();

        // Did a change occur that would force us to overwrite the entry? This also adds
        // new entries.
        if uuid.is_changed() || username.is_changed() || props.is_changed() {
            actions.set_add_player(true);

            if *game_mode != GameMode::default() {
                actions.set_update_game_mode(true);
            }

            if ping.0 != 0 {
                actions.set_update_latency(true);
            }

            if display_name.0.is_some() {
                actions.set_update_display_name(true);
            }

            if listed.0 {
                actions.set_update_listed(true);
            }

            if chat_session.is_some() {
                actions.set_initialize_chat(true);
            }
        } else {
            if game_mode.is_changed() {
                actions.set_update_game_mode(true);
            }

            if ping.is_changed() {
                actions.set_update_latency(true);
            }

            if display_name.is_changed() {
                actions.set_update_display_name(true);
            }

            if listed.is_changed() {
                actions.set_update_listed(true);
            }

            if matches!(&chat_session, Some(session) if session.is_changed()) {
                actions.set_initialize_chat(true);
            }

            debug_assert_ne!(u8::from(actions), 0);
        }

        let entry = packet::PlayerListEntry {
            player_uuid: uuid.0,
            username: &username.0,
            properties: Cow::Borrowed(&props.0),
            chat_data: chat_session.map(|s| s.session_data.clone().into()),
            listed: listed.0,
            ping: ping.0,
            game_mode: *game_mode,
            display_name: display_name.0.as_ref().map(|x| x.into()),
        };

        writer.write_packet(&PlayerListS2c {
            actions,
            entries: Cow::Borrowed(&[entry]),
        });
    }
}

fn write_player_list_changes(
    mut player_list: ResMut<PlayerList>,
    mut clients: Query<&mut Client, Without<Despawned>>,
) {
    if !player_list.cached_update_packets.is_empty() {
        for mut client in &mut clients {
            if !client.is_added() {
                client.write_packet_bytes(&player_list.cached_update_packets);
            }
        }

        player_list.cached_update_packets.clear();
    }
}
