use std::ops::BitXor;

use crate::error::{DataFormatError, Error};

pub enum DataFormat<T> {
    All(T),
    Palette(usize, T),
}

pub fn parse_palette<T: Copy, F: (FnMut(DataFormat<T>) -> Result<(), Error>)>(
    source: &Vec<T>,
    data: Option<Vec<i64>>,
    min_bits: usize,
    expected_len: usize,
    fun: &mut F,
) -> Result<(), Error> {
    let palette_len = source.len();
    if palette_len == 0 {
        return Err(crate::error::Error::DataFormatError(
            DataFormatError::InvalidPalette,
        ));
    }
    if let Some(data) = data {
        if palette_len < 2 || data.is_empty() {
            fun(DataFormat::All(source[0]))?;
            Ok(())
        } else {
            let choice_len = palette_len - 1; //Corrects for the absence of a non-choice: null is not an option.
            let bits_per_index = usize::max(
                (usize::BITS - choice_len.leading_zeros()) as usize,
                min_bits,
            );
            let entries_per_integer = i64::BITS as usize / bits_per_index;

            let mut entry_mask = (u64::MAX << bits_per_index).bitxor(u64::MAX);
            let mut mask_fields: Vec<(u64, usize)> = vec![(0u64, 0usize); entries_per_integer];
            for (i, mask_field) in mask_fields.iter_mut().enumerate() {
                *mask_field = (entry_mask, (i * bits_per_index));
                entry_mask <<= bits_per_index;
            }

            let mut index: usize = 0;
            for integer in data {
                let integer = integer as u64;
                for (mask, rev_shift) in &mask_fields {
                    let palette_index_unshifted = (integer & mask) as usize;
                    let palette_index_shifted = palette_index_unshifted >> rev_shift;

                    if palette_index_shifted > choice_len {
                        return Err(crate::error::Error::DataFormatError(
                            DataFormatError::InvalidPalette,
                        ));
                    } else {
                        fun(DataFormat::Palette(index, source[palette_index_shifted]))?;
                        index += 1;
                        // Prevents interpreting the rest of the long as data.
                        if index == expected_len {
                            return Ok(());
                        }
                    }
                }
            }
            Ok(())
        }
    } else {
        fun(DataFormat::All(source[0]))?;
        Ok(())
    }
}
