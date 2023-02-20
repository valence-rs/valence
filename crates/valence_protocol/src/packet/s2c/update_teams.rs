use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use bitfield_struct::bitfield;

use crate::{Decode, Encode, Text};

#[derive(Clone, PartialEq, Debug)]
pub enum UpdateTeamsMode<'a> {
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

impl Encode for UpdateTeamsMode<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            UpdateTeamsMode::CreateTeam {
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
            UpdateTeamsMode::RemoveTeam => 1i8.encode(&mut w)?,
            UpdateTeamsMode::UpdateTeamInfo {
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
            UpdateTeamsMode::AddEntities { entities } => {
                3i8.encode(&mut w)?;
                entities.encode(&mut w)?;
            }
            UpdateTeamsMode::RemoveEntities { entities } => {
                4i8.encode(&mut w)?;
                entities.encode(&mut w)?;
            }
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for UpdateTeamsMode<'a> {
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
