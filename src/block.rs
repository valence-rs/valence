#![allow(clippy::all)]

include!(concat!(env!("OUT_DIR"), "/block.rs"));

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
