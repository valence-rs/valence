use glam::DVec3;
use valence_core::game_mode::GameMode;
use valence_core::packet::s2c::play::command_tree::{Parser, StringArg};
use valence_core::translation_key::{
    ARGUMENT_ENTITY_OPTIONS_UNTERMINATED, COMMAND_UNKNOWN_ARGUMENT,
};
use valence_nbt::Compound;

use crate::argument::InclusiveRange;
use crate::parser::{
    parse_array_like, BrigadierArgument, DefaultParsableData, ErrorMessage, Parsable, ParsingError,
};
use crate::position::ANGLE_BOUNDS;
use crate::reader::{EndFilterResult, StrReader};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct EntitySelector<'a> {
    pub basic: BasicEntitySelector,

    /// ### Arguments
    /// - x (float)
    /// - y (float)
    /// - z (float)
    pub position: Option<DVec3>,
    /// ### Arguments
    /// - distance (float range) or (float)
    pub distance: InclusiveRange<f64>,
    /// ### Arguments
    /// - dx (float)
    /// - dy (float)
    /// - dz (float)
    pub volume: Option<DVec3>,

    /// ### Arguments
    /// - scores (contains {} with name = (float range) or (float))
    pub scores: Vec<EntitySelectorScore<'a>>,
    /// ### Arguments
    /// - tag (entity selector flag)
    pub tags: Vec<EntitySelectorFlag<'a>>,
    /// ### Arguments
    /// - team (entity selector flag)
    pub teams: Vec<EntitySelectorFlag<'a>>,

    /// ### Arguments
    /// - name (entity selector flag)
    pub names: Vec<EntitySelectorFlag<'a>>,
    /// ### Arguments
    /// - x_rotation (float range) or (float)
    pub x_rotation: Option<InclusiveRange<f32>>,
    /// ### Arguments
    /// - y_rotation (float range) or (float)
    pub y_rotation: Option<InclusiveRange<f32>>,
    /// ### Arguments
    /// - type (entity selector flag)
    pub types: Vec<EntitySelectorFlag<'a>>,
    /// ### Arguments
    /// - nbt (entity selector flag for snbt)
    pub nbt: Option<(bool, Compound)>,
    /// ### Arguments
    /// - advancements
    pub advancements: Vec<EntitySelectorAdvancement<'a>>,
    /// ### Arguments
    /// - predicate (entity selector flag)
    pub predicate: Vec<EntitySelectorFlag<'a>>,

    pub experience_level: Option<InclusiveRange<i32>>,
    /// ### Indexes
    /// 0. Survival
    /// 1. Creative
    /// 2. Adventure
    /// 3. Spectator
    pub gamemodes: [bool; 4],

    pub sort: EntitySelectorSort,
    pub limit: Option<i32>,
}

impl<'a> EntitySelector<'a> {
    pub fn only_players(&mut self) {
        self.types.push(EntitySelectorFlag {
            value: false,
            name: "minecraft:player",
        });
    }

    pub fn single(&mut self) {
        self.limit = Some(1);
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EntitySelectorData {
    pub only_players: bool,
    pub single: bool,
}

impl<'a> Parsable<'a> for EntitySelector<'a> {
    type Data = EntitySelectorData;

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        let mut e_sel = match reader.next_char() {
            Some('p') => BasicEntitySelector::NearestPlayer,
            Some('r') => BasicEntitySelector::RandomPlayer,
            Some('a') => BasicEntitySelector::AllPlayers,
            Some('e') => BasicEntitySelector::AllEntities,
            Some('s') => BasicEntitySelector::SelfEntity,
            _ => Err(COMMAND_UNKNOWN_ARGUMENT.empty())?,
        }
        .default_entity_selector();

        if reader.peek_char() == Some('[') {
            reader.next_char();
            reader.skip_recursive_only(' ');
            while reader.skip_only(']').is_none() {
                let name = reader
                    .read_unquoted_str()
                    .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
                let name_cursor = reader.cursor();

                macro_rules! rollback {
                    () => {
                        // SAFETY:
                        // Name cursor is a valid cursor because was get with cursor() method
                        unsafe {
                            reader.set_cursor(name_cursor);
                        }
                        Err(COMMAND_UNKNOWN_ARGUMENT.empty())?;
                    };
                }

                reader.skip_recursive_only(' ');
                reader
                    .skip_only('=')
                    .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
                reader.skip_recursive_only(' ');

                match name {
                    "x" => {
                        e_sel.position.get_or_insert(DVec3::ZERO).x =
                            f64::parse(&f64::DEFAULT_DATA, reader)?
                    }
                    "y" => {
                        e_sel.position.get_or_insert(DVec3::ZERO).y =
                            f64::parse(&f64::DEFAULT_DATA, reader)?
                    }
                    "z" => {
                        e_sel.position.get_or_insert(DVec3::ZERO).z =
                            f64::parse(&f64::DEFAULT_DATA, reader)?
                    }
                    "distance" => {
                        e_sel.distance =
                            InclusiveRange::parse(&InclusiveRange::<f64>::DEFAULT_DATA, reader)?;
                    }
                    "dx" => {
                        e_sel.volume.get_or_insert(DVec3::ZERO).x =
                            f64::parse(&f64::DEFAULT_DATA, reader)?
                    }
                    "dy" => {
                        e_sel.volume.get_or_insert(DVec3::ZERO).y =
                            f64::parse(&f64::DEFAULT_DATA, reader)?
                    }
                    "dz" => {
                        e_sel.volume.get_or_insert(DVec3::ZERO).z =
                            f64::parse(&f64::DEFAULT_DATA, reader)?
                    }
                    "scores" => {
                        reader
                            .skip_only('{')
                            .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
                        reader.skip_recursive_only(' ');
                        parse_array_like::<'}', _>(&(), reader, &mut e_sel.scores)?;
                    }
                    "tag" => {
                        e_sel.tags.push(EntitySelectorFlag::parse(&(), reader)?);
                    }
                    "team" => {
                        e_sel.teams.push(EntitySelectorFlag::parse(&(), reader)?);
                    }
                    "name" => {
                        e_sel.names.push(EntitySelectorFlag::parse(&(), reader)?);
                    }
                    "type" => {
                        e_sel.types.push(EntitySelectorFlag::parse(&(), reader)?);
                    }
                    "predicate" => {
                        e_sel
                            .predicate
                            .push(EntitySelectorFlag::parse(&(), reader)?);
                    }
                    "x_rotation" => {
                        if e_sel.x_rotation.is_some() {
                            rollback!();
                        }
                        e_sel.x_rotation = Some(InclusiveRange::parse(&[ANGLE_BOUNDS; 2], reader)?);
                    }
                    "y_rotation" => {
                        if e_sel.y_rotation.is_some() {
                            rollback!();
                        }
                        e_sel.y_rotation = Some(InclusiveRange::parse(&[ANGLE_BOUNDS; 2], reader)?);
                    }
                    "nbt" => {
                        // TODO
                    }
                    "level" => {
                        if e_sel.experience_level.is_some() {
                            rollback!();
                        }
                        e_sel.experience_level = Some(InclusiveRange::parse(
                            &InclusiveRange::<i32>::DEFAULT_DATA,
                            reader,
                        )?);
                    }
                    "gamemode" => {
                        let value = reader.skip_only('!').is_none();
                        e_sel.gamemodes[GameMode::parse(&(), reader)?.to_index()] = value;
                    }
                    "advancements" => {
                        reader
                            .skip_only('{')
                            .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
                        reader.skip_recursive_only(' ');
                        parse_array_like::<'}', _>(&(), reader, &mut e_sel.advancements)?;
                    }
                    "limit" => {
                        e_sel.limit = Some(i32::parse(&i32::DEFAULT_DATA, reader)?);
                    }
                    "sort" => {
                        e_sel.sort = match reader
                            .read_unquoted_str()
                            .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?
                            .to_ascii_lowercase()
                            .as_str()
                        {
                            "nearest" => EntitySelectorSort::Nearest,
                            "furthest" => EntitySelectorSort::Furthest,
                            "random" => EntitySelectorSort::Random,
                            "arbitrary" => EntitySelectorSort::Arbitrary,
                            _ => Err(COMMAND_UNKNOWN_ARGUMENT.empty())?,
                        }
                    }
                    _ => {
                        rollback!();
                    }
                }
                reader.skip_recursive_only(' ');
                match reader.next_char() {
                    Some(',') => {}
                    Some(']') => {
                        break;
                    }
                    _ => Err(COMMAND_UNKNOWN_ARGUMENT.empty())?,
                }
                reader.skip_recursive_only(' ');
            }
        }

        Ok(e_sel)
    }
}

impl<'a> BrigadierArgument<'a> for EntitySelector<'a> {
    fn brigadier_parser(data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::Entity {
            only_players: data.only_players,
            single: data.single,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum EntitySelectorSort {
    Nearest,
    Furthest,
    Random,
    #[default]
    Arbitrary,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntitySelectorScore<'a> {
    pub name: &'a str,
    pub values: InclusiveRange<i32>,
}

impl<'a> Parsable<'a> for EntitySelectorScore<'a> {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        // TODO check if the error messages are right.
        let name = reader
            .read_unquoted_str()
            .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
        reader.skip_recursive_only(' ');
        reader
            .skip_only('=')
            .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
        reader.skip_recursive_only(' ');
        let values = InclusiveRange::parse(&InclusiveRange::<i32>::DEFAULT_DATA, reader)?;
        Ok(Self { name, values })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntitySelectorFlag<'a> {
    pub value: bool,
    pub name: &'a str,
}

impl<'a> Parsable<'a> for EntitySelectorFlag<'a> {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        let value = reader.skip_only('!').is_none();
        let name = <&'a str>::parse(&StringArg::QuotablePhrase, reader)
            .map_err(|_| ARGUMENT_ENTITY_OPTIONS_UNTERMINATED.empty())?;
        Ok(Self { value, name })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntitySelectorAdvancement<'a> {
    pub name: &'a str,
    pub flag: EntitySelectorAdvancementFlag<'a>,
}

impl<'a> Parsable<'a> for EntitySelectorAdvancement<'a> {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        let name = reader
            .read_until_filter::<false, false>(|ch| match ch {
                '=' | ' ' => EndFilterResult::EndExclude,
                _ => EndFilterResult::Continue,
            })
            .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
        reader.skip_recursive_only(' ');
        reader
            .skip_only('=')
            .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
        reader.skip_recursive_only(' ');
        let flag = EntitySelectorAdvancementFlag::parse(&(), reader)?;
        reader.skip_recursive_only(' ');
        Ok(Self { name, flag })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EntitySelectorAdvancementFlag<'a> {
    Whole(bool),
    Criteria(EntitySelectorFlag<'a>),
}

impl<'a> Parsable<'a> for EntitySelectorAdvancementFlag<'a> {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        Ok(if reader.skip_only('{').is_some() {
            let name = reader
                .read_unquoted_str()
                .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
            reader.skip_recursive_only(' ');
            reader
                .skip_only('=')
                .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
            reader.skip_recursive_only(' ');
            let value = bool::parse(&(), reader)?;
            reader
                .skip_only('}')
                .ok_or_else(|| COMMAND_UNKNOWN_ARGUMENT.empty())?;
            Self::Criteria(EntitySelectorFlag { value, name })
        } else {
            Self::Whole(bool::parse(&(), reader)?)
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum BasicEntitySelector {
    /// @p
    NearestPlayer,
    /// @r
    RandomPlayer,
    /// @a
    AllPlayers,
    /// @e
    #[default]
    AllEntities,
    /// @s
    SelfEntity,
}

impl BasicEntitySelector {
    pub fn default_entity_selector<'a>(&self) -> EntitySelector<'a> {
        let mut result = EntitySelector {
            basic: *self,
            ..Default::default()
        };
        match self {
            BasicEntitySelector::NearestPlayer => {
                result.only_players();
                result.single();
                result.sort = EntitySelectorSort::Nearest;
            }
            BasicEntitySelector::RandomPlayer => {
                result.only_players();
                result.single();
                result.sort = EntitySelectorSort::Random;
            }
            BasicEntitySelector::AllPlayers => result.only_players(),
            _ => {}
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_selector_test() {
        let mut reader = StrReader::new(
            "e[x = 10.0, y = 20.0, z = 30.0, distance = 0..100, dx = 3.0, dy = 3.0, dz = 3.0, \
             scores = {some_score=0,another_score=10..20}, tag=tag1, tag=!tag2, team=team1, \
             team=!team2, name=Jenya705, type=player, x_rotation=-99.0.., y_rotation=0.., \
             advancements={adventure/kill_all_mobs={witch=true}}]",
        );
        let e_sel = EntitySelector::parse(
            &EntitySelectorData {
                only_players: false,
                single: false,
            },
            &mut reader,
        )
        .unwrap();
        assert_eq!(
            e_sel,
            EntitySelector {
                position: Some(DVec3::new(10.0, 20.0, 30.0)),
                distance: InclusiveRange {
                    min: 0.0,
                    max: 100.0,
                },
                volume: Some(DVec3::splat(3.0)),
                scores: vec![
                    EntitySelectorScore {
                        name: "some_score",
                        values: InclusiveRange { min: 0, max: 0 }
                    },
                    EntitySelectorScore {
                        name: "another_score",
                        values: InclusiveRange { min: 10, max: 20 }
                    }
                ],
                tags: vec![
                    EntitySelectorFlag {
                        value: true,
                        name: "tag1"
                    },
                    EntitySelectorFlag {
                        value: false,
                        name: "tag2"
                    }
                ],
                teams: vec![
                    EntitySelectorFlag {
                        value: true,
                        name: "team1"
                    },
                    EntitySelectorFlag {
                        value: false,
                        name: "team2"
                    }
                ],
                names: vec![EntitySelectorFlag {
                    value: true,
                    name: "Jenya705"
                }],
                types: vec![EntitySelectorFlag {
                    value: true,
                    name: "player"
                }],
                x_rotation: Some(InclusiveRange {
                    min: -99.0,
                    max: 180.0
                }),
                y_rotation: Some(InclusiveRange {
                    min: 0.0,
                    max: 180.0
                }),
                advancements: vec![EntitySelectorAdvancement {
                    name: "adventure/kill_all_mobs",
                    flag: EntitySelectorAdvancementFlag::Criteria(EntitySelectorFlag {
                        value: true,
                        name: "witch"
                    })
                }],
                ..Default::default()
            }
        )
    }
}
