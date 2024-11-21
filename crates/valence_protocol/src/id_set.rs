use std::io::Write;

use crate::{Decode, Encode, VarInt};

#[derive(Debug, PartialEq, Eq)]
/// Represents a set of IDs in a certain registry, either directly (enumerated
/// IDs) or indirectly (tag name).
///
/// # Variants
///
/// - `NamedSet(String)`: Represents a named set of IDs defined by a tag.
/// - `AdHocSet(Vec<VarInt>)`: Represents an ad-hoc set of IDs enumerated
///   inline.
pub enum IDSet {
    NamedSet(String),
    AdHocSet(Vec<VarInt>),
}

impl Encode for IDSet {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            IDSet::NamedSet(tag_name) => {
                VarInt(0).encode(&mut w)?;
                tag_name.encode(w)
            }
            IDSet::AdHocSet(ids) => {
                VarInt((ids.len() + 1) as i32).encode(&mut w)?;
                for id in ids {
                    id.encode(&mut w)?;
                }
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for IDSet {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let type_id = VarInt::decode(r)?.0;
        if type_id == 0 {
            let tag_name = String::decode(r)?;
            Ok(IDSet::NamedSet(tag_name))
        } else {
            let mut ids = Vec::with_capacity((type_id - 1) as usize);
            for _ in 0..(type_id - 1) {
                ids.push(VarInt::decode(r)?);
            }
            Ok(IDSet::AdHocSet(ids))
        }
    }
}
