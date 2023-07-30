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