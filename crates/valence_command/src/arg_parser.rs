use std::ops::Add;
use thiserror::Error;

use crate::command_graph::{Parser, StringArg};

pub trait CommandArgSet {
    fn from_args(args: Vec<String>) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgLen {
    Infinite, // can only be the last argument
    Exact(u32),
    Within(char), // Example "man this is cool" would be 4 args without this distinction
    WithinExplicit(char, char), // [man this is cooler] would be 4 arg without this distinction
}

pub trait CommandArg: Default {
    type Result: Default;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError>;
    /// how many arguments does this type take up
    fn len() -> ArgLen {
        ArgLen::Exact(1)
    }
    /// what will the client be sent
    fn display() -> Parser;
}

#[derive(Debug, Error)]
pub enum CommandArgParseError {
    // these should be player facing and not disclose internal information
    #[error("invalid argument, expected {0} got {1}")] // e.g. "integer" number
    InvalidArgument(String, String),
    #[error("invalid argument length")]
    InvalidArgLength,
}

impl CommandArg for bool {
    type Result = bool;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        match string.to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(CommandArgParseError::InvalidArgument(
                "bool".to_string(),
                string,
            )),
        }
    }

    fn display() -> Parser {
        Parser::Bool
    }
}

macro_rules! impl_parser_for_number {
    ($type:ty, $name:expr, $parser:ident) => {
        impl CommandArg for $type {
            type Result = $type;

            fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
                match string.parse::<$type>() {
                    Ok(num) => Ok(num),
                    Err(_) => Err(CommandArgParseError::InvalidArgument(
                        $name.to_string(),
                        string,
                    )),
                }
            }

            fn display() -> Parser {
                Parser::$parser {
                    min: None,
                    max: None,
                }
            }
        }
    };
}

impl_parser_for_number!(f32, "float", Float);
impl_parser_for_number!(f64, "double", Double);
impl_parser_for_number!(i32, "integer", Integer);
impl_parser_for_number!(i64, "long", Long);

impl CommandArg for String {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::String(StringArg::SingleWord)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GreedyString;

impl CommandArg for GreedyString {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn len() -> ArgLen {
        ArgLen::Infinite
    }

    fn display() -> Parser {
        Parser::String(StringArg::GreedyPhrase)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuotableString;

impl CommandArg for QuotableString {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn len() -> ArgLen {
        ArgLen::Within('"')
    }

    fn display() -> Parser {
        Parser::String(StringArg::QuotablePhrase)
    }
}

// TODO: impl Enity properly

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntitySelector;

impl CommandArg for EntitySelector {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::Entity {
            only_players: false,
            single: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SingleEntitySelector;

impl CommandArg for SingleEntitySelector {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::Entity {
            only_players: false,
            single: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PlayerSelector {
    Single(String),
    #[default]
    All,
    SelfPlayer,
    Nearest,
}

impl CommandArg for PlayerSelector {
    type Result = PlayerSelector;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(if string == "@a" {
            PlayerSelector::All
        } else if string == "@s" {
            PlayerSelector::SelfPlayer
        } else if string == "@p" {
            PlayerSelector::Nearest
        } else {
            PlayerSelector::Single(string)
        })
    }

    fn display() -> Parser {
        Parser::Entity {
            only_players: true,
            single: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SinglePlayerSelector;

impl CommandArg for SinglePlayerSelector {
    type Result = PlayerSelector;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(if string == "@s" {
            PlayerSelector::SelfPlayer
        } else if string == "@p" {
            PlayerSelector::Nearest
        } else {
            PlayerSelector::Single(string)
        })
    }

    fn display() -> Parser {
        Parser::Entity {
            only_players: true,
            single: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PossiblyRelative<T> {
    Absolute(T),
    Relative(T), // current value + T
}

impl<T> PossiblyRelative<T>
    where T: Add<Output = T> + Copy,
{
    pub fn get(&self, origanal: T) -> T {
        match self {
            Self::Absolute(num) => *num,
            Self::Relative(num) => *num + origanal,
        }
    }
}

impl<T> CommandArg for PossiblyRelative<T>
where
    T: CommandArg,
{
    type Result = PossiblyRelative<T::Result>;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        if string.starts_with('~') {
            let possibly_value = string.trim_start_matches('~').to_string();
            Ok(PossiblyRelative::Relative(if possibly_value.is_empty() {
                T::Result::default()
            } else {
                T::arg_from_string(possibly_value)?
            }))
        } else {
            Ok(PossiblyRelative::Absolute(T::arg_from_string(string)?))
        }
    }

    fn display() -> Parser {
        T::display()
    }
}

impl<T: Default> Default for PossiblyRelative<T> {
    fn default() -> Self {
        PossiblyRelative::Absolute(T::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockPos {
    x: PossiblyRelative<i32>,
    y: PossiblyRelative<i32>,
    z: PossiblyRelative<i32>,
}

impl CommandArg for BlockPos {
    type Result = BlockPos;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        let mut split = string.split(' ');
        let x = PossiblyRelative::<i32>::arg_from_string(split.next().unwrap().to_string())?;
        let y = PossiblyRelative::<i32>::arg_from_string(split.next().unwrap().to_string())?;
        let z = PossiblyRelative::<i32>::arg_from_string(split.next().unwrap().to_string())?;

        Ok(BlockPos { x, y, z })
    }
    fn len() -> ArgLen {
        ArgLen::Exact(3)
    }

    fn display() -> Parser {
        Parser::BlockPos
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColumnPos {
    x: PossiblyRelative<i32>,
    y: PossiblyRelative<i32>,
    z: PossiblyRelative<i32>,
}

impl CommandArg for ColumnPos {
    type Result = ColumnPos;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        let mut split = string.split(' ');
        let x = PossiblyRelative::<i32>::arg_from_string(split.next().unwrap().to_string())?;
        let y = PossiblyRelative::<i32>::arg_from_string(split.next().unwrap().to_string())?;
        let z = PossiblyRelative::<i32>::arg_from_string(split.next().unwrap().to_string())?;

        Ok(ColumnPos { x, y, z })
    }

    fn len() -> ArgLen {
        ArgLen::Exact(3)
    }

    fn display() -> Parser {
        Parser::ColumnPos
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: PossiblyRelative<f32>,
    pub y: PossiblyRelative<f32>,
    pub z: PossiblyRelative<f32>,
}

impl CommandArg for Vec3 {
    type Result = Vec3;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        let mut split = string.split(' ');
        let x = PossiblyRelative::<f32>::arg_from_string(split.next().unwrap().to_string())?;
        let y = PossiblyRelative::<f32>::arg_from_string(split.next().unwrap().to_string())?;
        let z = PossiblyRelative::<f32>::arg_from_string(split.next().unwrap().to_string())?;

        Ok(Vec3 { x, y, z })
    }

    fn len() -> ArgLen {
        ArgLen::Exact(3)
    }

    fn display() -> Parser {
        Parser::Vec3
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    x: PossiblyRelative<f32>,
    y: PossiblyRelative<f32>,
}

impl CommandArg for Vec2 {
    type Result = Vec2;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        let mut split = string.split(' ');
        let x = PossiblyRelative::<f32>::arg_from_string(split.next().unwrap().to_string())?;
        let y = PossiblyRelative::<f32>::arg_from_string(split.next().unwrap().to_string())?;

        Ok(Vec2 { x, y })
    }

    fn len() -> ArgLen {
        ArgLen::Exact(2)
    }

    fn display() -> Parser {
        Parser::Vec2
    }
}

// TODO: BlockState proper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockState;

impl CommandArg for BlockState {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::BlockState
    }
}

// TODO: block predicate proper

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockPredicate;

impl CommandArg for BlockPredicate {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::BlockPredicate
    }
}

// TODO: item stack proper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemStack;

impl CommandArg for ItemStack {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::ItemStack
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatColor {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    #[default]
    White,
    Reset,
}

impl CommandArg for ChatColor {
    type Result = ChatColor;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(match string.to_lowercase().as_str() {
            "black" => ChatColor::Black,
            "dark_blue" => ChatColor::DarkBlue,
            "dark_green" => ChatColor::DarkGreen,
            "dark_aqua" => ChatColor::DarkAqua,
            "dark_red" => ChatColor::DarkRed,
            "dark_purple" => ChatColor::DarkPurple,
            "gold" => ChatColor::Gold,
            "gray" => ChatColor::Gray,
            "dark_gray" => ChatColor::DarkGray,
            "blue" => ChatColor::Blue,
            "green" => ChatColor::Green,
            "aqua" => ChatColor::Aqua,
            "red" => ChatColor::Red,
            "light_purple" => ChatColor::LightPurple,
            "yellow" => ChatColor::Yellow,
            "white" => ChatColor::White,
            "reset" => ChatColor::Reset,
            _ => {
                return Err(CommandArgParseError::InvalidArgument(
                    "chat_color".to_string(),
                    string,
                ))
            }
        })
    }

    fn display() -> Parser {
        Parser::Color
    }
}

// TODO: json chat component proper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JsonChatComponent;

impl CommandArg for JsonChatComponent {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::Component
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Message;

impl CommandArg for Message {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::Message
    }
}

// TODO: nbt proper

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Nbt;

impl CommandArg for Nbt {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::NbtCompoundTag
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NbtTag;

impl CommandArg for NbtTag {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::NbtTag
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NbtPath;

impl CommandArg for NbtPath {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::NbtPath
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Objective;

impl CommandArg for Objective {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::Objective
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ObjectiveCriteria;

impl CommandArg for ObjectiveCriteria {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::ObjectiveCriteria
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Angle;

impl CommandArg for Angle {
    type Result = f32;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string.parse::<f32>().map_err(|_| {
            CommandArgParseError::InvalidArgument("angle".to_string(), string.clone())
        })?)
    }

    fn display() -> Parser {
        Parser::Angle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rotation;

impl CommandArg for Rotation {
    type Result = Vec2;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        let mut split = string.split(' ');
        let x = PossiblyRelative::<f32>::arg_from_string(split.next().unwrap().to_string())?;
        let y = PossiblyRelative::<f32>::arg_from_string(split.next().unwrap().to_string())?;

        Ok(Vec2 { x, y })
    }

    fn len() -> ArgLen {
        ArgLen::Exact(2)
    }

    fn display() -> Parser {
        Parser::Rotation
    }
}

// TODO: ScoreboardSlot proper

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScoreboardSlot;

impl CommandArg for ScoreboardSlot {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::ScoreboardSlot
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ScoreHolder {
    Entity(String), // TODO: EntitySelector proper
    #[default]
    All,
}

impl CommandArg for ScoreHolder {
    type Result = ScoreHolder;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(match string.as_str() {
            "*" => ScoreHolder::All,
            _ => ScoreHolder::Entity(EntitySelector::arg_from_string(string)?),
        })
    }

    fn display() -> Parser {
        Parser::ScoreHolder {
            allow_multiple: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Swizzle {
    x: bool,
    y: bool,
    z: bool,
}

impl CommandArg for Swizzle {
    type Result = Swizzle;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        let mut x = false;
        let mut y = false;
        let mut z = false;

        for c in string.chars() {
            match c {
                'x' => x = true,
                'y' => y = true,
                'z' => z = true,
                _ => {
                    return Err(CommandArgParseError::InvalidArgument(
                        "swizzle".to_string(),
                        string,
                    ))
                }
            }
        }

        Ok(Swizzle { x, y, z })
    }

    fn display() -> Parser {
        Parser::Swizzle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TeamName;

impl CommandArg for TeamName {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::Team
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InventorySlot;

impl CommandArg for InventorySlot {
    type Result = u32;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string.parse::<u32>().map_err(|_| {
            CommandArgParseError::InvalidArgument("inventory_slot".to_string(), string.clone())
        })?)
    }

    fn display() -> Parser {
        Parser::ItemSlot
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceLocation;

impl CommandArg for ResourceLocation {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::ResourceLocation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Function;

impl CommandArg for Function {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::Function
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityAnchor {
    #[default]
    Eyes,
    Feet,
}

impl CommandArg for EntityAnchor {
    type Result = EntityAnchor;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(match string.as_str() {
            "eyes" => EntityAnchor::Eyes,
            "feet" => EntityAnchor::Feet,
            _ => {
                return Err(CommandArgParseError::InvalidArgument(
                    "entity_anchor".to_string(),
                    string,
                ))
            }
        })
    }

    fn display() -> Parser {
        Parser::EntityAnchor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IntRange {
    min: i32,
    max: i32,
}

impl CommandArg for IntRange {
    type Result = IntRange;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        let mut split = string.split("..");
        let min = split.next().unwrap().parse::<i32>().map_err(|_| {
            CommandArgParseError::InvalidArgument("int_range max".to_string(), string.clone())
        })?;
        let max = split.next().unwrap().parse::<i32>().map_err(|_| {
            CommandArgParseError::InvalidArgument("int_range min".to_string(), string.clone())
        })?;

        Ok(IntRange { min, max })
    }

    fn len() -> ArgLen {
        ArgLen::Exact(2)
    }

    fn display() -> Parser {
        Parser::IntRange
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FloatRange {
    min: f32,
    max: f32,
}

impl CommandArg for FloatRange {
    type Result = FloatRange;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        let mut split = string.split("..");
        let min = split.next().unwrap().parse::<f32>().map_err(|_| {
            CommandArgParseError::InvalidArgument("float_range max".to_string(), string.clone())
        })?;
        let max = split.next().unwrap().parse::<f32>().map_err(|_| {
            CommandArgParseError::InvalidArgument("float_range min".to_string(), string.clone())
        })?;

        Ok(FloatRange { min, max })
    }

    fn len() -> ArgLen {
        ArgLen::Exact(2)
    }

    fn display() -> Parser {
        Parser::FloatRange
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dimension {
    #[default]
    Overworld,
    Nether,
    End,
}

impl CommandArg for Dimension {
    type Result = Dimension;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(match string.to_lowercase().as_str() {
            "overworld" => Dimension::Overworld,
            "nether" => Dimension::Nether,
            "end" => Dimension::End,
            _ => {
                return Err(CommandArgParseError::InvalidArgument(
                    "dimension".to_string(),
                    string,
                ))
            }
        })
    }

    fn display() -> Parser {
        Parser::Dimension
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl CommandArg for GameMode {
    type Result = GameMode;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(match string.to_lowercase().as_str() {
            "survival" => GameMode::Survival,
            "creative" => GameMode::Creative,
            "adventure" => GameMode::Adventure,
            "spectator" => GameMode::Spectator,
            _ => {
                return Err(CommandArgParseError::InvalidArgument(
                    "game_mode".to_string(),
                    string,
                ))
            }
        })
    }

    fn display() -> Parser {
        Parser::GameMode
    }
}

// TODO: Add more time

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Time {
    ticks: i32,
}

impl CommandArg for Time {
    type Result = Time;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(Time {
            ticks: string.parse::<i32>().map_err(|_| {
                CommandArgParseError::InvalidArgument("time".to_string(), string.clone())
            })?,
        })
    }

    fn display() -> Parser {
        Parser::Time
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Uuid;

impl CommandArg for Uuid {
    type Result = String;

    fn arg_from_string(string: String) -> Result<Self::Result, CommandArgParseError> {
        Ok(string)
    }

    fn display() -> Parser {
        Parser::Uuid
    }
}
