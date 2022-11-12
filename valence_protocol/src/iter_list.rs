use std::io::Write;

use anyhow::bail;

use crate::var_int::VarInt;
use crate::{Encode, Result};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct IterList<I>(pub I);

impl<I, T> Encode for IterList<I>
where
    I: ExactSizeIterator<Item = T> + Clone,
    T: Encode,
{
    fn encode(&self, mut w: impl Write) -> Result<()> {
        let Ok(len) = self.0.len().try_into() else {
            bail!("iterator length exceeds i32::MAX");
        };

        VarInt(len).encode(&mut w)?;

        for t in self.0.clone() {
            t.encode(&mut w)?;
        }

        Ok(())
    }

    fn encoded_len(&self) -> usize {
        VarInt(self.0.len() as i32).encoded_len()
            + self.0.clone().map(|t| t.encoded_len()).sum::<usize>()
    }
}
