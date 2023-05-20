use valence_block::{BlockKind, BlockState, PropName, PropValue};
use valence_core::packet::s2c::play::command_tree::Parser;
use valence_core::translation_key::{
    ARGUMENT_BLOCK_PROPERTY_INVALID, ARGUMENT_BLOCK_PROPERTY_NOVALUE,
    ARGUMENT_BLOCK_PROPERTY_UNCLOSED, ARGUMENT_BLOCK_PROPERTY_UNKNOWN, ARGUMENT_ID_INVALID,
};
use valence_instance::BlockRef;
use valence_nbt::Compound;

use crate::parser::{BrigadierArgument, ErrorMessage, Parsable, ParsingError};
use crate::reader::StrReader;

#[derive(Debug)]
pub struct BlockPredicate<'a> {
    pub tag: bool,
    pub id: &'a str,
    pub props: Vec<(PropName, PropValue)>,
    pub nbt: Option<Compound>,
}

fn read_props(
    reader: &mut StrReader,
    block_state: &Option<BlockState>,
    block_id: &str,
    mut func: impl FnMut(PropName, PropValue) -> Result<(), ParsingError>,
) -> Result<(), ParsingError> {
    if reader.skip_only('[').is_some() {
        while reader.skip_only(']').is_none() {
            reader.skip_recursive_only(' ');
            let prop_name_str = reader
                .read_unquoted_str()
                .ok_or_else(|| ARGUMENT_BLOCK_PROPERTY_UNCLOSED.empty())?;
            let prop_name = PropName::from_str(prop_name_str)
                .filter(|prop_name| {
                    block_state.map_or(true, |state| state.get(*prop_name).is_some())
                })
                .ok_or_else(|| {
                    ARGUMENT_BLOCK_PROPERTY_UNKNOWN.with(vec![
                        block_id.to_string().into(),
                        prop_name_str.to_string().into(),
                    ])
                })?;
            reader.skip_recursive_only(' ');
            let prop_value_str = reader
                .skip_only('=')
                .and_then(|_| {
                    reader.skip_recursive_only(' ');
                    reader.read_unquoted_str()
                })
                .ok_or_else(|| {
                    ARGUMENT_BLOCK_PROPERTY_NOVALUE.with(vec![
                        prop_name_str.to_string().into(),
                        block_id.to_string().into(),
                    ])
                })?;
            let prop_value = PropValue::from_str(prop_value_str).ok_or_else(|| {
                ARGUMENT_BLOCK_PROPERTY_INVALID.with(vec![
                    block_id.to_string().into(),
                    prop_value_str.to_string().into(),
                    prop_name_str.to_string().into(),
                ])
            })?;
            func(prop_name, prop_value)?;
        }
    }
    Ok(())
}

impl<'a> Parsable<'a> for BlockPredicate<'a> {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        let tag = reader.skip_only('#').is_some();
        let id = reader
            .read_unquoted_str()
            .ok_or_else(|| ARGUMENT_ID_INVALID.empty())?;
        let block_kind = if tag { BlockKind::from_str(id) } else { None };
        let block_state = block_kind.map(|kind| kind.to_state());
        let mut props = vec![];
        read_props(reader, &block_state, id, |name, value| {
            props.push((name, value));
            Ok(())
        })?;

        // TODO snbt read.

        Ok(Self {
            tag,
            id,
            props,
            nbt: None,
        })
    }
}

impl<'a> BrigadierArgument<'a> for BlockPredicate<'a> {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::BlockPredicate
    }
}

impl<'a> BlockPredicate<'a> {
    pub fn test(&self, block: BlockRef) -> bool {
        if self.tag {
            unimplemented!("Tag is not implemented")
        }
        if self.id != block.state().to_kind().to_str() {
            return false;
        }
        for (prop_name, prop_value) in self.props.iter() {
            if block.state().get(*prop_name) != Some(*prop_value) {
                return false;
            }
        }
        if let (Some(bnbt), Some(snbt)) = (block.nbt(), &self.nbt) {
            if !bnbt.contains_compound(snbt) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct BlockStateArgument {
    pub state: BlockState,
    pub nbt: Option<Compound>,
}

impl<'a> Parsable<'a> for BlockStateArgument {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        let id = reader
            .read_unquoted_str()
            .ok_or_else(|| ARGUMENT_ID_INVALID.empty())?;
        let mut state = BlockKind::from_str(id)
            .map(|kind| kind.to_state())
            .ok_or_else(|| ARGUMENT_ID_INVALID.empty())?;
        read_props(reader, &Some(state), id, |prop_name, prop_value| {
            state = state.set(prop_name, prop_value);
            Ok(())
        })?;
        Ok(Self { state, nbt: None })
    }
}

impl<'a> BrigadierArgument<'a> for BlockStateArgument {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::BlockState
    }
}
