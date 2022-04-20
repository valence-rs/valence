#![allow(clippy::all)]

use std::fmt;

include!(concat!(env!("OUT_DIR"), "/block.rs"));

impl fmt::Debug for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let typ = self.to_type();

        write!(f, "{}", typ.to_str())?;

        let props = typ.props();

        if !props.is_empty() {
            let mut list = f.debug_list();
            for &p in typ.props() {
                struct KeyVal<'a>(&'a str, &'a str);

                impl<'a> fmt::Debug for KeyVal<'a> {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "{}={}", self.0, self.1)
                    }
                }

                list.entry(&KeyVal(p.to_str(), self.get(p).unwrap().to_str()));
            }
            list.finish()
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set_consistency() {
        for typ in BlockType::ALL {
            let block = typ.to_state();

            for &prop in typ.props() {
                let new_block = block.set(prop, block.get(prop).unwrap());
                assert_eq!(new_block, block);
            }
        }
    }
}
