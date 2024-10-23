use bevy_ecs::prelude::Component;

use super::set_player_team_s2c::TeamColor;
use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SetDisplayObjectiveS2c<'a> {
    pub position: ScoreboardPosition,
    pub score_name: &'a str,
}

/// Defines where a scoreboard is displayed.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Component, Default)]
pub enum ScoreboardPosition {
    /// Display the scoreboard in the player list (the one you see when you
    /// press tab), as a yellow number next to players' names.
    List,
    /// Display the scoreboard on the sidebar.
    #[default]
    Sidebar,
    /// Display the scoreboard below players' name tags in the world.
    BelowName,
    /// Display the scoreboard on the sidebar, visible only to one team.
    SidebarTeam(TeamColor),
}

impl Encode for ScoreboardPosition {
    fn encode(&self, w: impl std::io::Write) -> anyhow::Result<()> {
        match self {
            ScoreboardPosition::List => 0_u8.encode(w),
            ScoreboardPosition::Sidebar => 1_u8.encode(w),
            ScoreboardPosition::BelowName => 2_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::Black) => 3_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::DarkBlue) => 4_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::DarkGreen) => 5_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::DarkCyan) => 6_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::DarkRed) => 7_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::Purple) => 8_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::Gold) => 9_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::Gray) => 10_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::DarkGray) => 11_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::Blue) => 12_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::BrightGreen) => 13_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::Cyan) => 14_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::Red) => 15_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::Pink) => 16_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::Yellow) => 17_u8.encode(w),
            ScoreboardPosition::SidebarTeam(TeamColor::White) => 18_u8.encode(w),
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
