//! A temporary module for packets that do not yet have a home outside of
//! valence_core.
//!
//! All packets should be moved out of this module eventually.

use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use bitfield_struct::bitfield;
use byteorder::WriteBytesExt;
use glam::IVec3;
use uuid::Uuid;

use crate::ident;
use crate::ident::Ident;
use crate::protocol::var_int::VarInt;
use crate::protocol::{packet_id, Decode, Encode, Packet};
use crate::text::Text;

// TODO: move module contents to valence_chat.
pub mod chat {

    pub use super::*;

    #[derive(Copy, Clone, PartialEq, Debug)]
    pub struct MessageSignature<'a> {
        pub message_id: i32,
        pub signature: Option<&'a [u8; 256]>,
    }

    impl<'a> Encode for MessageSignature<'a> {
        fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
            VarInt(self.message_id + 1).encode(&mut w)?;

            match self.signature {
                None => {}
                Some(signature) => signature.encode(&mut w)?,
            }

            Ok(())
        }
    }

    impl<'a> Decode<'a> for MessageSignature<'a> {
        fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
            let message_id = VarInt::decode(r)?.0 - 1; // TODO: this can underflow.

            let signature = if message_id == -1 {
                Some(<&[u8; 256]>::decode(r)?)
            } else {
                None
            };

            Ok(Self {
                message_id,
                signature,
            })
        }
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::CHAT_MESSAGE_C2S)]
    pub struct ChatMessageC2s<'a> {
        pub message: &'a str,
        pub timestamp: u64,
        pub salt: u64,
        pub signature: Option<&'a [u8; 256]>,
        pub message_count: VarInt,
        // This is a bitset of 20; each bit represents one
        // of the last 20 messages received and whether or not
        // the message was acknowledged by the client
        pub acknowledgement: [u8; 3],
    }

    #[derive(Clone, Debug, Encode, Decode)]
    #[packet(id = packet_id::COMMAND_EXECUTION_C2S)]
    pub struct CommandExecutionC2s<'a> {
        pub command: &'a str,
        pub timestamp: u64,
        pub salt: u64,
        pub argument_signatures: Vec<CommandArgumentSignature<'a>>,
        pub message_count: VarInt,
        //// This is a bitset of 20; each bit represents one
        //// of the last 20 messages received and whether or not
        //// the message was acknowledged by the client
        pub acknowledgement: [u8; 3],
    }

    #[derive(Copy, Clone, Debug, Encode, Decode)]
    pub struct CommandArgumentSignature<'a> {
        pub argument_name: &'a str,
        pub signature: &'a [u8; 256],
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::MESSAGE_ACKNOWLEDGMENT_C2S)]

    pub struct MessageAcknowledgmentC2s {
        pub message_count: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::PLAYER_SESSION_C2S)]
    pub struct PlayerSessionC2s<'a> {
        pub session_id: Uuid,
        // Public key
        pub expires_at: i64,
        pub public_key_data: &'a [u8],
        pub key_signature: &'a [u8],
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::REQUEST_COMMAND_COMPLETIONS_C2S)]
    pub struct RequestCommandCompletionsC2s<'a> {
        pub transaction_id: VarInt,
        pub text: &'a str,
    }

    #[derive(Clone, PartialEq, Debug, Packet)]
    #[packet(id = packet_id::CHAT_MESSAGE_S2C)]
    pub struct ChatMessageS2c<'a> {
        pub sender: Uuid,
        pub index: VarInt,
        pub message_signature: Option<&'a [u8; 256]>,
        pub message: &'a str,
        pub time_stamp: u64,
        pub salt: u64,
        pub previous_messages: Vec<MessageSignature<'a>>,
        pub unsigned_content: Option<Cow<'a, Text>>,
        pub filter_type: MessageFilterType,
        pub filter_type_bits: Option<u8>,
        pub chat_type: VarInt,
        pub network_name: Cow<'a, Text>,
        pub network_target_name: Option<Cow<'a, Text>>,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum MessageFilterType {
        PassThrough,
        FullyFiltered,
        PartiallyFiltered,
    }

    impl<'a> Encode for ChatMessageS2c<'a> {
        fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
            self.sender.encode(&mut w)?;
            self.index.encode(&mut w)?;
            self.message_signature.encode(&mut w)?;
            self.message.encode(&mut w)?;
            self.time_stamp.encode(&mut w)?;
            self.salt.encode(&mut w)?;
            self.previous_messages.encode(&mut w)?;
            self.unsigned_content.encode(&mut w)?;
            self.filter_type.encode(&mut w)?;

            if self.filter_type == MessageFilterType::PartiallyFiltered {
                match self.filter_type_bits {
                    // Filler data
                    None => 0u8.encode(&mut w)?,
                    Some(bits) => bits.encode(&mut w)?,
                }
            }

            self.chat_type.encode(&mut w)?;
            self.network_name.encode(&mut w)?;
            self.network_target_name.encode(&mut w)?;

            Ok(())
        }
    }

    impl<'a> Decode<'a> for ChatMessageS2c<'a> {
        fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
            let sender = Uuid::decode(r)?;
            let index = VarInt::decode(r)?;
            let message_signature = Option::<&'a [u8; 256]>::decode(r)?;
            let message = <&str>::decode(r)?;
            let time_stamp = u64::decode(r)?;
            let salt = u64::decode(r)?;
            let previous_messages = Vec::<MessageSignature>::decode(r)?;
            let unsigned_content = Option::<Cow<'a, Text>>::decode(r)?;
            let filter_type = MessageFilterType::decode(r)?;

            let filter_type_bits = match filter_type {
                MessageFilterType::PartiallyFiltered => Some(u8::decode(r)?),
                _ => None,
            };

            let chat_type = VarInt::decode(r)?;
            let network_name = <Cow<'a, Text>>::decode(r)?;
            let network_target_name = Option::<Cow<'a, Text>>::decode(r)?;

            Ok(Self {
                sender,
                index,
                message_signature,
                message,
                time_stamp,
                salt,
                previous_messages,
                unsigned_content,
                filter_type,
                filter_type_bits,
                chat_type,
                network_name,
                network_target_name,
            })
        }
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::CHAT_SUGGESTIONS_S2C)]
    pub struct ChatSuggestionsS2c<'a> {
        pub action: ChatSuggestionsAction,
        pub entries: Cow<'a, [&'a str]>,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum ChatSuggestionsAction {
        Add,
        Remove,
        Set,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::REMOVE_MESSAGE_S2C)]
    pub struct RemoveMessageS2c<'a> {
        pub signature: MessageSignature<'a>,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::COMMAND_SUGGESTIONS_S2C)]
    pub struct CommandSuggestionsS2c<'a> {
        pub id: VarInt,
        pub start: VarInt,
        pub length: VarInt,
        pub matches: Vec<CommandSuggestionsMatch<'a>>,
    }

    #[derive(Clone, PartialEq, Debug, Encode, Decode)]
    pub struct CommandSuggestionsMatch<'a> {
        pub suggested_match: &'a str,
        pub tooltip: Option<Cow<'a, Text>>,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::PROFILELESS_CHAT_MESSAGE_S2C)]
    pub struct ProfilelessChatMessageS2c<'a> {
        pub message: Cow<'a, Text>,
        pub chat_type: VarInt,
        pub chat_type_name: Cow<'a, Text>,
        pub target_name: Option<Cow<'a, Text>>,
    }
}

// TODO: move to valence_scoreboard?
pub mod scoreboard {

    use super::*;

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::TEAM_S2C)]
    pub struct TeamS2c<'a> {
        pub team_name: &'a str,
        pub mode: Mode<'a>,
    }

    #[derive(Clone, PartialEq, Debug)]
    pub enum Mode<'a> {
        CreateTeam {
            team_display_name: Cow<'a, Text>,
            friendly_flags: TeamFlags,
            name_tag_visibility: NameTagVisibility,
            collision_rule: CollisionRule,
            team_color: TeamColor,
            team_prefix: Cow<'a, Text>,
            team_suffix: Cow<'a, Text>,
            entities: Vec<&'a str>,
        },
        RemoveTeam,
        UpdateTeamInfo {
            team_display_name: Cow<'a, Text>,
            friendly_flags: TeamFlags,
            name_tag_visibility: NameTagVisibility,
            collision_rule: CollisionRule,
            team_color: TeamColor,
            team_prefix: Cow<'a, Text>,
            team_suffix: Cow<'a, Text>,
        },
        AddEntities {
            entities: Vec<&'a str>,
        },
        RemoveEntities {
            entities: Vec<&'a str>,
        },
    }

    #[bitfield(u8)]
    #[derive(PartialEq, Eq, Encode, Decode)]
    pub struct TeamFlags {
        pub friendly_fire: bool,
        pub see_invisible_teammates: bool,
        #[bits(6)]
        _pad: u8,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum NameTagVisibility {
        Always,
        Never,
        HideForOtherTeams,
        HideForOwnTeam,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum CollisionRule {
        Always,
        Never,
        PushOtherTeams,
        PushOwnTeam,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum TeamColor {
        Black,
        DarkBlue,
        DarkGreen,
        DarkCyan,
        DarkRed,
        Purple,
        Gold,
        Gray,
        DarkGray,
        Blue,
        BrightGreen,
        Cyan,
        Red,
        Pink,
        Yellow,
        White,
        Obfuscated,
        Bold,
        Strikethrough,
        Underlined,
        Italic,
        Reset,
    }

    impl Encode for Mode<'_> {
        fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
            match self {
                Mode::CreateTeam {
                    team_display_name,
                    friendly_flags,
                    name_tag_visibility,
                    collision_rule,
                    team_color,
                    team_prefix,
                    team_suffix,
                    entities,
                } => {
                    0i8.encode(&mut w)?;
                    team_display_name.encode(&mut w)?;
                    friendly_flags.encode(&mut w)?;
                    match name_tag_visibility {
                        NameTagVisibility::Always => "always",
                        NameTagVisibility::Never => "never",
                        NameTagVisibility::HideForOtherTeams => "hideForOtherTeams",
                        NameTagVisibility::HideForOwnTeam => "hideForOwnTeam",
                    }
                    .encode(&mut w)?;
                    match collision_rule {
                        CollisionRule::Always => "always",
                        CollisionRule::Never => "never",
                        CollisionRule::PushOtherTeams => "pushOtherTeams",
                        CollisionRule::PushOwnTeam => "pushOwnTeam",
                    }
                    .encode(&mut w)?;
                    team_color.encode(&mut w)?;
                    team_prefix.encode(&mut w)?;
                    team_suffix.encode(&mut w)?;
                    entities.encode(&mut w)?;
                }
                Mode::RemoveTeam => 1i8.encode(&mut w)?,
                Mode::UpdateTeamInfo {
                    team_display_name,
                    friendly_flags,
                    name_tag_visibility,
                    collision_rule,
                    team_color,
                    team_prefix,
                    team_suffix,
                } => {
                    2i8.encode(&mut w)?;
                    team_display_name.encode(&mut w)?;
                    friendly_flags.encode(&mut w)?;
                    match name_tag_visibility {
                        NameTagVisibility::Always => "always",
                        NameTagVisibility::Never => "never",
                        NameTagVisibility::HideForOtherTeams => "hideForOtherTeams",
                        NameTagVisibility::HideForOwnTeam => "hideForOwnTeam",
                    }
                    .encode(&mut w)?;
                    match collision_rule {
                        CollisionRule::Always => "always",
                        CollisionRule::Never => "never",
                        CollisionRule::PushOtherTeams => "pushOtherTeams",
                        CollisionRule::PushOwnTeam => "pushOwnTeam",
                    }
                    .encode(&mut w)?;
                    team_color.encode(&mut w)?;
                    team_prefix.encode(&mut w)?;
                    team_suffix.encode(&mut w)?;
                }
                Mode::AddEntities { entities } => {
                    3i8.encode(&mut w)?;
                    entities.encode(&mut w)?;
                }
                Mode::RemoveEntities { entities } => {
                    4i8.encode(&mut w)?;
                    entities.encode(&mut w)?;
                }
            }
            Ok(())
        }
    }

    impl<'a> Decode<'a> for Mode<'a> {
        fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
            Ok(match i8::decode(r)? {
                0 => Self::CreateTeam {
                    team_display_name: Decode::decode(r)?,
                    friendly_flags: Decode::decode(r)?,
                    name_tag_visibility: match <&str>::decode(r)? {
                        "always" => NameTagVisibility::Always,
                        "never" => NameTagVisibility::Never,
                        "hideForOtherTeams" => NameTagVisibility::HideForOtherTeams,
                        "hideForOwnTeam" => NameTagVisibility::HideForOwnTeam,
                        other => bail!("unknown name tag visibility type \"{other}\""),
                    },
                    collision_rule: match <&str>::decode(r)? {
                        "always" => CollisionRule::Always,
                        "never" => CollisionRule::Never,
                        "pushOtherTeams" => CollisionRule::PushOtherTeams,
                        "pushOwnTeam" => CollisionRule::PushOwnTeam,
                        other => bail!("unknown collision rule type \"{other}\""),
                    },
                    team_color: Decode::decode(r)?,
                    team_prefix: Decode::decode(r)?,
                    team_suffix: Decode::decode(r)?,
                    entities: Decode::decode(r)?,
                },
                1 => Self::RemoveTeam,
                2 => Self::UpdateTeamInfo {
                    team_display_name: Decode::decode(r)?,
                    friendly_flags: Decode::decode(r)?,
                    name_tag_visibility: match <&str>::decode(r)? {
                        "always" => NameTagVisibility::Always,
                        "never" => NameTagVisibility::Never,
                        "hideForOtherTeams" => NameTagVisibility::HideForOtherTeams,
                        "hideForOwnTeam" => NameTagVisibility::HideForOwnTeam,
                        other => bail!("unknown name tag visibility type \"{other}\""),
                    },
                    collision_rule: match <&str>::decode(r)? {
                        "always" => CollisionRule::Always,
                        "never" => CollisionRule::Never,
                        "pushOtherTeams" => CollisionRule::PushOtherTeams,
                        "pushOwnTeam" => CollisionRule::PushOwnTeam,
                        other => bail!("unknown collision rule type \"{other}\""),
                    },
                    team_color: Decode::decode(r)?,
                    team_prefix: Decode::decode(r)?,
                    team_suffix: Decode::decode(r)?,
                },
                3 => Self::AddEntities {
                    entities: Decode::decode(r)?,
                },
                4 => Self::RemoveEntities {
                    entities: Decode::decode(r)?,
                },
                n => bail!("unknown update teams action of {n}"),
            })
        }
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::SCOREBOARD_DISPLAY_S2C)]
    pub struct ScoreboardDisplayS2c<'a> {
        pub position: ScoreboardPosition,
        pub score_name: &'a str,
    }

    #[derive(Copy, Clone, PartialEq, Debug)]
    pub enum ScoreboardPosition {
        List,
        Sidebar,
        BelowName,
        SidebarTeam(TeamColor),
    }

    impl Encode for ScoreboardPosition {
        fn encode(&self, w: impl std::io::Write) -> anyhow::Result<()> {
            match self {
                ScoreboardPosition::List => 0u8.encode(w),
                ScoreboardPosition::Sidebar => 1u8.encode(w),
                ScoreboardPosition::BelowName => 2u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::Black) => 3u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::DarkBlue) => 4u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::DarkGreen) => 5u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::DarkCyan) => 6u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::DarkRed) => 7u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::Purple) => 8u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::Gold) => 9u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::Gray) => 10u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::DarkGray) => 11u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::Blue) => 12u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::BrightGreen) => 13u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::Cyan) => 14u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::Red) => 15u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::Pink) => 16u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::Yellow) => 17u8.encode(w),
                ScoreboardPosition::SidebarTeam(TeamColor::White) => 18u8.encode(w),
                ScoreboardPosition::SidebarTeam(_) => {
                    Err(anyhow::anyhow!("Invalid scoreboard display position"))
                }
            }
        }
    }

    impl<'a> Decode<'a> for ScoreboardPosition {
        fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
            let value = u8::decode(r)?;
            match value {
                0 => Ok(ScoreboardPosition::List),
                1 => Ok(ScoreboardPosition::Sidebar),
                2 => Ok(ScoreboardPosition::BelowName),
                3 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::Black)),
                4 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::DarkBlue)),
                5 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::DarkGreen)),
                6 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::DarkCyan)),
                7 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::DarkRed)),
                8 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::Purple)),
                9 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::Gold)),
                10 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::Gray)),
                11 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::DarkGray)),
                12 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::Blue)),
                13 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::BrightGreen)),
                14 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::Cyan)),
                15 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::Red)),
                16 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::Pink)),
                17 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::Yellow)),
                18 => Ok(ScoreboardPosition::SidebarTeam(TeamColor::White)),
                _ => Err(anyhow::anyhow!("Invalid scoreboard display position")),
            }
        }
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::SCOREBOARD_OBJECTIVE_UPDATE_S2C)]
    pub struct ScoreboardObjectiveUpdateS2c<'a> {
        pub objective_name: &'a str,
        pub mode: ObjectiveMode,
    }

    #[derive(Clone, PartialEq, Debug, Encode, Decode)]
    pub enum ObjectiveMode {
        Create {
            objective_display_name: Text,
            render_type: ObjectiveRenderType,
        },
        Remove,
        Update {
            objective_display_name: Text,
            render_type: ObjectiveRenderType,
        },
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum ObjectiveRenderType {
        Integer,
        Hearts,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::SCOREBOARD_PLAYER_UPDATE_S2C)]
    pub struct ScoreboardPlayerUpdateS2c<'a> {
        pub entity_name: &'a str,
        pub action: ScoreboardPlayerUpdateAction<'a>,
    }

    #[derive(Clone, PartialEq, Debug, Encode, Decode)]
    pub enum ScoreboardPlayerUpdateAction<'a> {
        Update {
            objective_name: &'a str,
            objective_score: VarInt,
        },
        Remove {
            objective_name: &'a str,
        },
    }
}

// TODO: move to valence_boss_bar?
pub mod boss_bar {
    use super::*;

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::BOSS_BAR_S2C)]
    pub struct BossBarS2c {
        pub id: Uuid,
        pub action: BossBarAction,
    }

    #[derive(Clone, PartialEq, Debug, Encode, Decode)]
    pub enum BossBarAction {
        Add {
            title: Text,
            health: f32,
            color: BossBarColor,
            division: BossBarDivision,
            flags: BossBarFlags,
        },
        Remove,
        UpdateHealth(f32),
        UpdateTitle(Text),
        UpdateStyle(BossBarColor, BossBarDivision),
        UpdateFlags(BossBarFlags),
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum BossBarColor {
        Pink,
        Blue,
        Red,
        Green,
        Yellow,
        Purple,
        White,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum BossBarDivision {
        NoDivision,
        SixNotches,
        TenNotches,
        TwelveNotches,
        TwentyNotches,
    }

    #[bitfield(u8)]
    #[derive(PartialEq, Eq, Encode, Decode)]
    pub struct BossBarFlags {
        pub darken_sky: bool,
        pub dragon_bar: bool,
        pub create_fog: bool,
        #[bits(5)]
        _pad: u8,
    }
}

// TODO: move to valence_sound?
pub mod sound {
    use super::*;

    include!(concat!(env!("OUT_DIR"), "/sound.rs"));

    impl Sound {
        pub fn to_id(self) -> SoundId<'static> {
            SoundId::Direct {
                id: self.to_ident().into(),
                range: None,
            }
        }
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum SoundCategory {
        Master,
        Music,
        Record,
        Weather,
        Block,
        Hostile,
        Neutral,
        Player,
        Ambient,
        Voice,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn sound_to_soundid() {
            assert_eq!(
                Sound::BlockBellUse.to_id(),
                SoundId::Direct {
                    id: ident!("block.bell.use").into(),
                    range: None
                },
            );
        }
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::PLAY_SOUND_FROM_ENTITY_S2C)]
    pub struct PlaySoundFromEntityS2c {
        pub id: VarInt,
        pub category: SoundCategory,
        pub entity_id: VarInt,
        pub volume: f32,
        pub pitch: f32,
        pub seed: i64,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::PLAY_SOUND_S2C)]
    pub struct PlaySoundS2c<'a> {
        pub id: SoundId<'a>,
        pub category: SoundCategory,
        pub position: IVec3,
        pub volume: f32,
        pub pitch: f32,
        pub seed: i64,
    }

    #[derive(Clone, PartialEq, Debug)]
    pub enum SoundId<'a> {
        Direct {
            id: Ident<Cow<'a, str>>,
            range: Option<f32>,
        },
        Reference {
            id: VarInt,
        },
    }

    impl Encode for SoundId<'_> {
        fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
            match self {
                SoundId::Direct { id, range } => {
                    VarInt(0).encode(&mut w)?;
                    id.encode(&mut w)?;
                    range.encode(&mut w)?;
                }
                SoundId::Reference { id } => VarInt(id.0 + 1).encode(&mut w)?,
            }

            Ok(())
        }
    }

    impl<'a> Decode<'a> for SoundId<'a> {
        fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
            let i = VarInt::decode(r)?.0;

            if i == 0 {
                Ok(SoundId::Direct {
                    id: Ident::decode(r)?,
                    range: <Option<f32>>::decode(r)?,
                })
            } else {
                Ok(SoundId::Reference { id: VarInt(i - 1) })
            }
        }
    }

    #[derive(Clone, PartialEq, Debug, Packet)]
    #[packet(id = packet_id::STOP_SOUND_S2C)]
    pub struct StopSoundS2c<'a> {
        pub source: Option<SoundCategory>,
        pub sound: Option<Ident<Cow<'a, str>>>,
    }

    impl Encode for StopSoundS2c<'_> {
        fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
            match (self.source, self.sound.as_ref()) {
                (Some(source), Some(sound)) => {
                    3i8.encode(&mut w)?;
                    source.encode(&mut w)?;
                    sound.encode(&mut w)?;
                }
                (None, Some(sound)) => {
                    2i8.encode(&mut w)?;
                    sound.encode(&mut w)?;
                }
                (Some(source), None) => {
                    1i8.encode(&mut w)?;
                    source.encode(&mut w)?;
                }
                _ => 0i8.encode(&mut w)?,
            }

            Ok(())
        }
    }

    impl<'a> Decode<'a> for StopSoundS2c<'a> {
        fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
            let (source, sound) = match i8::decode(r)? {
                3 => (
                    Some(SoundCategory::decode(r)?),
                    Some(<Ident<Cow<'a, str>>>::decode(r)?),
                ),
                2 => (None, Some(<Ident<Cow<'a, str>>>::decode(r)?)),
                1 => (Some(SoundCategory::decode(r)?), None),
                _ => (None, None),
            };

            Ok(Self { source, sound })
        }
    }
}

// TODO: move to valence_command
pub mod command {

    use super::*;

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::COMMAND_TREE_S2C)]
    pub struct CommandTreeS2c<'a> {
        pub commands: Vec<Node<'a>>,
        pub root_index: VarInt,
    }

    #[derive(Clone, Debug)]
    pub struct Node<'a> {
        pub children: Vec<VarInt>,
        pub data: NodeData<'a>,
        pub executable: bool,
        pub redirect_node: Option<VarInt>,
    }

    #[derive(Clone, Debug)]
    pub enum NodeData<'a> {
        Root,
        Literal {
            name: &'a str,
        },
        Argument {
            name: &'a str,
            parser: Parser<'a>,
            suggestion: Option<Suggestion>,
        },
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum Suggestion {
        AskServer,
        AllRecipes,
        AvailableSounds,
        AvailableBiomes,
        SummonableEntities,
    }

    #[derive(Clone, Debug)]
    pub enum Parser<'a> {
        Bool,
        Float { min: Option<f32>, max: Option<f32> },
        Double { min: Option<f64>, max: Option<f64> },
        Integer { min: Option<i32>, max: Option<i32> },
        Long { min: Option<i64>, max: Option<i64> },
        String(StringArg),
        Entity { single: bool, only_players: bool },
        GameProfile,
        BlockPos,
        ColumnPos,
        Vec3,
        Vec2,
        BlockState,
        BlockPredicate,
        ItemStack,
        ItemPredicate,
        Color,
        Component,
        Message,
        NbtCompoundTag,
        NbtTag,
        NbtPath,
        Objective,
        ObjectiveCriteria,
        Operation,
        Particle,
        Angle,
        Rotation,
        ScoreboardSlot,
        ScoreHolder { allow_multiple: bool },
        Swizzle,
        Team,
        ItemSlot,
        ResourceLocation,
        Function,
        EntityAnchor,
        IntRange,
        FloatRange,
        Dimension,
        GameMode,
        Time,
        ResourceOrTag { registry: Ident<Cow<'a, str>> },
        ResourceOrTagKey { registry: Ident<Cow<'a, str>> },
        Resource { registry: Ident<Cow<'a, str>> },
        ResourceKey { registry: Ident<Cow<'a, str>> },
        TemplateMirror,
        TemplateRotation,
        Uuid,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum StringArg {
        SingleWord,
        QuotablePhrase,
        GreedyPhrase,
    }

    impl Encode for Node<'_> {
        fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
            let node_type = match &self.data {
                NodeData::Root => 0,
                NodeData::Literal { .. } => 1,
                NodeData::Argument { .. } => 2,
            };

            let has_suggestion = matches!(
                &self.data,
                NodeData::Argument {
                    suggestion: Some(_),
                    ..
                }
            );

            let flags: u8 = node_type
                | (self.executable as u8 * 0x04)
                | (self.redirect_node.is_some() as u8 * 0x08)
                | (has_suggestion as u8 * 0x10);

            w.write_u8(flags)?;

            self.children.encode(&mut w)?;

            if let Some(redirect_node) = self.redirect_node {
                redirect_node.encode(&mut w)?;
            }

            match &self.data {
                NodeData::Root => {}
                NodeData::Literal { name } => {
                    name.encode(&mut w)?;
                }
                NodeData::Argument {
                    name,
                    parser,
                    suggestion,
                } => {
                    name.encode(&mut w)?;
                    parser.encode(&mut w)?;

                    if let Some(suggestion) = suggestion {
                        match suggestion {
                            Suggestion::AskServer => "ask_server",
                            Suggestion::AllRecipes => "all_recipes",
                            Suggestion::AvailableSounds => "available_sounds",
                            Suggestion::AvailableBiomes => "available_biomes",
                            Suggestion::SummonableEntities => "summonable_entities",
                        }
                        .encode(&mut w)?;
                    }
                }
            }

            Ok(())
        }
    }

    impl<'a> Decode<'a> for Node<'a> {
        fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
            let flags = u8::decode(r)?;

            let children = Vec::decode(r)?;

            let redirect_node = if flags & 0x08 != 0 {
                Some(VarInt::decode(r)?)
            } else {
                None
            };

            let node_data = match flags & 0x3 {
                0 => NodeData::Root,
                1 => NodeData::Literal {
                    name: <&str>::decode(r)?,
                },
                2 => NodeData::Argument {
                    name: <&str>::decode(r)?,
                    parser: Parser::decode(r)?,
                    suggestion: if flags & 0x10 != 0 {
                        Some(match Ident::<Cow<str>>::decode(r)?.as_str() {
                            "minecraft:ask_server" => Suggestion::AskServer,
                            "minecraft:all_recipes" => Suggestion::AllRecipes,
                            "minecraft:available_sounds" => Suggestion::AvailableSounds,
                            "minecraft:available_biomes" => Suggestion::AvailableBiomes,
                            "minecraft:summonable_entities" => Suggestion::SummonableEntities,
                            other => bail!("unknown command suggestion type of \"{other}\""),
                        })
                    } else {
                        None
                    },
                },
                n => bail!("invalid node type of {n}"),
            };

            Ok(Self {
                children,
                data: node_data,
                executable: flags & 0x04 != 0,
                redirect_node,
            })
        }
    }

    impl Encode for Parser<'_> {
        fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
            match self {
                Parser::Bool => 0u8.encode(&mut w)?,
                Parser::Float { min, max } => {
                    1u8.encode(&mut w)?;

                    (min.is_some() as u8 | (max.is_some() as u8 * 0x2)).encode(&mut w)?;

                    if let Some(min) = min {
                        min.encode(&mut w)?;
                    }

                    if let Some(max) = max {
                        max.encode(&mut w)?;
                    }
                }
                Parser::Double { min, max } => {
                    2u8.encode(&mut w)?;

                    (min.is_some() as u8 | (max.is_some() as u8 * 0x2)).encode(&mut w)?;

                    if let Some(min) = min {
                        min.encode(&mut w)?;
                    }

                    if let Some(max) = max {
                        max.encode(&mut w)?;
                    }
                }
                Parser::Integer { min, max } => {
                    3u8.encode(&mut w)?;

                    (min.is_some() as u8 | (max.is_some() as u8 * 0x2)).encode(&mut w)?;

                    if let Some(min) = min {
                        min.encode(&mut w)?;
                    }

                    if let Some(max) = max {
                        max.encode(&mut w)?;
                    }
                }
                Parser::Long { min, max } => {
                    4u8.encode(&mut w)?;

                    (min.is_some() as u8 | (max.is_some() as u8 * 0x2)).encode(&mut w)?;

                    if let Some(min) = min {
                        min.encode(&mut w)?;
                    }

                    if let Some(max) = max {
                        max.encode(&mut w)?;
                    }
                }
                Parser::String(arg) => {
                    5u8.encode(&mut w)?;
                    arg.encode(&mut w)?;
                }
                Parser::Entity {
                    single,
                    only_players,
                } => {
                    6u8.encode(&mut w)?;
                    (*single as u8 | (*only_players as u8 * 0x2)).encode(&mut w)?;
                }
                Parser::GameProfile => 7u8.encode(&mut w)?,
                Parser::BlockPos => 8u8.encode(&mut w)?,
                Parser::ColumnPos => 9u8.encode(&mut w)?,
                Parser::Vec3 => 10u8.encode(&mut w)?,
                Parser::Vec2 => 11u8.encode(&mut w)?,
                Parser::BlockState => 12u8.encode(&mut w)?,
                Parser::BlockPredicate => 13u8.encode(&mut w)?,
                Parser::ItemStack => 14u8.encode(&mut w)?,
                Parser::ItemPredicate => 15u8.encode(&mut w)?,
                Parser::Color => 16u8.encode(&mut w)?,
                Parser::Component => 17u8.encode(&mut w)?,
                Parser::Message => 18u8.encode(&mut w)?,
                Parser::NbtCompoundTag => 19u8.encode(&mut w)?,
                Parser::NbtTag => 20u8.encode(&mut w)?,
                Parser::NbtPath => 21u8.encode(&mut w)?,
                Parser::Objective => 22u8.encode(&mut w)?,
                Parser::ObjectiveCriteria => 23u8.encode(&mut w)?,
                Parser::Operation => 24u8.encode(&mut w)?,
                Parser::Particle => 25u8.encode(&mut w)?,
                Parser::Angle => 26u8.encode(&mut w)?,
                Parser::Rotation => 27u8.encode(&mut w)?,
                Parser::ScoreboardSlot => 28u8.encode(&mut w)?,
                Parser::ScoreHolder { allow_multiple } => {
                    29u8.encode(&mut w)?;
                    allow_multiple.encode(&mut w)?;
                }
                Parser::Swizzle => 30u8.encode(&mut w)?,
                Parser::Team => 31u8.encode(&mut w)?,
                Parser::ItemSlot => 32u8.encode(&mut w)?,
                Parser::ResourceLocation => 33u8.encode(&mut w)?,
                Parser::Function => 34u8.encode(&mut w)?,
                Parser::EntityAnchor => 35u8.encode(&mut w)?,
                Parser::IntRange => 36u8.encode(&mut w)?,
                Parser::FloatRange => 37u8.encode(&mut w)?,
                Parser::Dimension => 38u8.encode(&mut w)?,
                Parser::GameMode => 39u8.encode(&mut w)?,
                Parser::Time => 40u8.encode(&mut w)?,
                Parser::ResourceOrTag { registry } => {
                    41u8.encode(&mut w)?;
                    registry.encode(&mut w)?;
                }
                Parser::ResourceOrTagKey { registry } => {
                    42u8.encode(&mut w)?;
                    registry.encode(&mut w)?;
                }
                Parser::Resource { registry } => {
                    43u8.encode(&mut w)?;
                    registry.encode(&mut w)?;
                }
                Parser::ResourceKey { registry } => {
                    44u8.encode(&mut w)?;
                    registry.encode(&mut w)?;
                }
                Parser::TemplateMirror => 45u8.encode(&mut w)?,
                Parser::TemplateRotation => 46u8.encode(&mut w)?,
                Parser::Uuid => 47u8.encode(&mut w)?,
            }

            Ok(())
        }
    }

    impl<'a> Decode<'a> for Parser<'a> {
        fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
            fn decode_min_max<'a, T: Decode<'a>>(
                r: &mut &'a [u8],
            ) -> anyhow::Result<(Option<T>, Option<T>)> {
                let flags = u8::decode(r)?;

                let min = if flags & 0x1 != 0 {
                    Some(T::decode(r)?)
                } else {
                    None
                };

                let max = if flags & 0x2 != 0 {
                    Some(T::decode(r)?)
                } else {
                    None
                };

                Ok((min, max))
            }

            Ok(match u8::decode(r)? {
                0 => Self::Bool,
                1 => {
                    let (min, max) = decode_min_max(r)?;
                    Self::Float { min, max }
                }
                2 => {
                    let (min, max) = decode_min_max(r)?;
                    Self::Double { min, max }
                }
                3 => {
                    let (min, max) = decode_min_max(r)?;
                    Self::Integer { min, max }
                }
                4 => {
                    let (min, max) = decode_min_max(r)?;
                    Self::Long { min, max }
                }
                5 => Self::String(StringArg::decode(r)?),
                6 => {
                    let flags = u8::decode(r)?;
                    Self::Entity {
                        single: flags & 0x1 != 0,
                        only_players: flags & 0x2 != 0,
                    }
                }
                7 => Self::GameProfile,
                8 => Self::BlockPos,
                9 => Self::ColumnPos,
                10 => Self::Vec3,
                11 => Self::Vec2,
                12 => Self::BlockState,
                13 => Self::BlockPredicate,
                14 => Self::ItemStack,
                15 => Self::ItemPredicate,
                16 => Self::Color,
                17 => Self::Component,
                18 => Self::Message,
                19 => Self::NbtCompoundTag,
                20 => Self::NbtTag,
                21 => Self::NbtPath,
                22 => Self::Objective,
                23 => Self::ObjectiveCriteria,
                24 => Self::Operation,
                25 => Self::Particle,
                26 => Self::Angle,
                27 => Self::Rotation,
                28 => Self::ScoreboardSlot,
                29 => Self::ScoreHolder {
                    allow_multiple: bool::decode(r)?,
                },
                30 => Self::Swizzle,
                31 => Self::Team,
                32 => Self::ItemSlot,
                33 => Self::ResourceLocation,
                34 => Self::Function,
                35 => Self::EntityAnchor,
                36 => Self::IntRange,
                37 => Self::FloatRange,
                38 => Self::Dimension,
                39 => Self::GameMode,
                40 => Self::Time,
                41 => Self::ResourceOrTag {
                    registry: Ident::decode(r)?,
                },
                42 => Self::ResourceOrTagKey {
                    registry: Ident::decode(r)?,
                },
                43 => Self::Resource {
                    registry: Ident::decode(r)?,
                },
                44 => Self::ResourceKey {
                    registry: Ident::decode(r)?,
                },
                45 => Self::TemplateMirror,
                46 => Self::TemplateRotation,
                47 => Self::Uuid,
                n => bail!("unknown command parser ID of {n}"),
            })
        }
    }
}

/// Move to valence_map?
pub mod map {
    use super::*;

    #[derive(Clone, PartialEq, Debug, Packet)]
    #[packet(id = packet_id::MAP_UPDATE_S2C)]
    pub struct MapUpdateS2c<'a> {
        pub map_id: VarInt,
        pub scale: i8,
        pub locked: bool,
        pub icons: Option<Vec<Icon<'a>>>,
        pub data: Option<Data<'a>>,
    }

    #[derive(Clone, PartialEq, Debug, Encode, Decode)]
    pub struct Icon<'a> {
        pub icon_type: IconType,
        /// In map coordinates; -128 for furthest left, +127 for furthest right
        pub position: [i8; 2],
        /// 0 is a vertical icon and increments by 22.5°
        pub direction: i8,
        pub display_name: Option<Cow<'a, Text>>,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum IconType {
        WhiteArrow,
        GreenArrow,
        RedArrow,
        BlueArrow,
        WhiteCross,
        RedPointer,
        WhiteCircle,
        SmallWhiteCircle,
        Mansion,
        Temple,
        WhiteBanner,
        OrangeBanner,
        MagentaBanner,
        LightBlueBanner,
        YellowBanner,
        LimeBanner,
        PinkBanner,
        GrayBanner,
        LightGrayBanner,
        CyanBanner,
        PurpleBanner,
        BlueBanner,
        BrownBanner,
        GreenBanner,
        RedBanner,
        BlackBanner,
        TreasureMarker,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode)]
    pub struct Data<'a> {
        pub columns: u8,
        pub rows: u8,
        pub position: [i8; 2],
        pub data: &'a [u8],
    }

    impl Encode for MapUpdateS2c<'_> {
        fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
            self.map_id.encode(&mut w)?;
            self.scale.encode(&mut w)?;
            self.locked.encode(&mut w)?;
            self.icons.encode(&mut w)?;

            match self.data {
                None => 0u8.encode(&mut w)?,
                Some(data) => data.encode(&mut w)?,
            }

            Ok(())
        }
    }

    impl<'a> Decode<'a> for MapUpdateS2c<'a> {
        fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
            let map_id = VarInt::decode(r)?;
            let scale = i8::decode(r)?;
            let locked = bool::decode(r)?;
            let icons = <Option<Vec<Icon<'a>>>>::decode(r)?;
            let columns = u8::decode(r)?;

            let data = if columns > 0 {
                let rows = u8::decode(r)?;
                let position = <[i8; 2]>::decode(r)?;
                let data = <&'a [u8]>::decode(r)?;

                Some(Data {
                    columns,
                    rows,
                    position,
                    data,
                })
            } else {
                None
            };

            Ok(Self {
                map_id,
                scale,
                locked,
                icons,
                data,
            })
        }
    }
}

// TODO: Move this to valence_registry?
pub mod synchronize_tags {
    use super::*;

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::SYNCHRONIZE_TAGS_S2C)]
    pub struct SynchronizeTagsS2c<'a> {
        pub tags: Vec<TagGroup<'a>>,
    }

    #[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub struct TagGroup<'a> {
        pub kind: Ident<Cow<'a, str>>,
        pub tags: Vec<Tag<'a>>,
    }

    #[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub struct Tag<'a> {
        pub name: Ident<Cow<'a, str>>,
        pub entries: Vec<VarInt>,
    }
}
