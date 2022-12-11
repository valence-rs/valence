use std::ops::BitXor;

use valence::nbt::{Compound, List, Value};
use valence::prelude::*;

use crate::error::{DataFormatError, Error, NbtFormatError};

pub enum DataFormat<T> {
    All(T),
    Palette(usize, T),
}

pub fn parse_palette_identities_with_properties<
    T: Copy,
    FT: FnMut(Ident<String>) -> Result<T, Error>,
    FP: FnMut(T, PropName, PropValue) -> Result<T, Error>,
    F: FnMut(DataFormat<T>) -> Result<(), Error>,
>(
    palette_container: Compound,
    min_bits: usize,
    expected_len: usize,
    mut loader: FT,
    mut applicator: FP,
    handler: F,
) -> Result<(), Error> {
    parse_compound_palette(
        palette_container,
        min_bits,
        expected_len,
        |mut nbt| match (nbt.remove("Name"), nbt.remove("Properties")) {
            (Some(Value::String(identity)), None) => loader(Ident::new(identity)?),
            (Some(Value::String(identity)), Some(Value::Compound(properties))) => {
                let mut object = loader(Ident::new(identity)?)?;
                for (property_name_raw, property_value) in &properties {
                    if let Value::String(property_value) = property_value {
                        match (
                            PropName::from_str(property_name_raw),
                            PropValue::from_str(property_value),
                        ) {
                            (Some(name), Some(value)) => {
                                object = applicator(object, name, value)?;
                            }
                            _ => {
                                return Err(Error::DataFormatError(
                                    DataFormatError::PropertyLoadError {
                                        name: property_name_raw.to_string(),
                                        value: property_value.to_string(),
                                    },
                                ))
                            }
                        }
                    } else {
                        return Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                            tag: Some(properties),
                            key: "Name".to_string(),
                        }));
                    }
                }
                Ok(object)
            }
            (Some(_), Some(Value::Compound(_))) => {
                Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                    tag: None,
                    key: "Name".to_string(),
                }))
            }
            (None, Some(Value::Compound(_))) => {
                Err(Error::NbtFormatError(NbtFormatError::MissingKey {
                    tag: None,
                    key: "Name".to_string(),
                }))
            }
            (_, Some(_)) => Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                tag: None,
                key: "Properties".to_string(),
            })),
            (_, None) => Err(Error::NbtFormatError(NbtFormatError::MissingKey {
                tag: None,
                key: "Properties".to_string(),
            })),
        },
        handler,
    )
}

pub fn parse_compound_palette<
    T: Copy,
    FT: FnMut(Compound) -> Result<T, Error>,
    F: FnMut(DataFormat<T>) -> Result<(), Error>,
>(
    mut palette_container: Compound,
    min_bits: usize,
    expected_len: usize,
    mut loader: FT,
    handler: F,
) -> Result<(), Error> {
    match palette_container.remove("palette") {
        Some(Value::List(List::Compound(nbt_palette_vec))) => {
            let iter = nbt_palette_vec.into_iter();
            let mut keys = Vec::<T>::with_capacity(iter.len());
            for tag in iter {
                keys.push(loader(tag)?)
            }
            match palette_container.remove("data") {
                Some(Value::LongArray(data)) => {
                    decode_palette(&keys, Some(data), min_bits, expected_len, handler)
                }
                Some(data) => {
                    palette_container.insert("data", data);
                    Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                        tag: Some(palette_container),
                        key: "data".to_string(),
                    }))
                }
                None => decode_palette(&keys, None, min_bits, expected_len, handler),
            }
        }
        Some(value) => {
            palette_container.insert("palette", value);
            Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                tag: Some(palette_container),
                key: "palette".to_string(),
            }))
        }
        None => Err(Error::NbtFormatError(NbtFormatError::MissingKey {
            tag: Some(palette_container),
            key: "palette".to_string(),
        })),
    }
}

pub fn parse_identity_list_palette<
    T: Copy,
    FT: FnMut(Ident<String>) -> Result<T, Error>,
    F: FnMut(DataFormat<T>) -> Result<(), Error>,
>(
    mut palette_container: Compound,
    min_bits: usize,
    expected_len: usize,
    mut loader: FT,
    handler: F,
) -> Result<(), Error> {
    match palette_container.remove("palette") {
        Some(Value::List(List::String(nbt_palette_vec))) => {
            let iter = nbt_palette_vec.into_iter();
            let mut keys = Vec::<T>::with_capacity(iter.len());
            for tag in iter {
                keys.push(loader(Ident::new(tag)?)?)
            }
            match palette_container.remove("data") {
                Some(Value::LongArray(data)) => {
                    decode_palette(&keys, Some(data), min_bits, expected_len, handler)
                }
                Some(data) => {
                    palette_container.insert("data", data);
                    Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                        tag: Some(palette_container),
                        key: "data".to_string(),
                    }))
                }
                None => decode_palette(&keys, None, min_bits, expected_len, handler),
            }
        }
        Some(value) => {
            palette_container.insert("palette", value);
            Err(Error::NbtFormatError(NbtFormatError::InvalidType {
                tag: Some(palette_container),
                key: "palette".to_string(),
            }))
        }
        None => Err(Error::NbtFormatError(NbtFormatError::MissingKey {
            tag: Some(palette_container),
            key: "palette".to_string(),
        })),
    }
}

pub fn decode_palette<T: Copy, F: (FnMut(DataFormat<T>) -> Result<(), Error>)>(
    source: &Vec<T>,
    data: Option<Vec<i64>>,
    min_bits: usize,
    expected_len: usize,
    mut fun: F,
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
