//! Zero-cost conversion functions for `valence_nbt`.
//!
//! While working with [`Value`], it is often necessary to convert between
//! collections of signed and unsigned integer types due to API
//! differences. For instance, you may be given a `&[i8]` from
//! [`Value::ByteArray`], but functions like [`Write::write_all`] expect to
//! receive a `&[u8]`.
//!
//! This module provides functions to perform conversions between these types
//! with zero-cost and no `unsafe` code on your part.
//!
//! [`Value`]: crate::Value
//! [`Value::ByteArray`]: crate::Value::ByteArray
//! [`Write::write_all`]: std::io::Write::write_all

use std::mem::ManuallyDrop;

/// Converts a `Vec<u8>` into a `Vec<i8>` without cloning.
#[inline]
pub fn u8_vec_into_i8_vec(vec: Vec<u8>) -> Vec<i8> {
    // SAFETY: Layouts of u8 and i8 are the same and we're being careful not to drop
    // the original vec after calling Vec::from_raw_parts.
    unsafe {
        let mut vec = ManuallyDrop::new(vec);
        Vec::from_raw_parts(vec.as_mut_ptr() as *mut i8, vec.len(), vec.capacity())
    }
}

/// Converts a `Vec<i8>` into a `Vec<u8>` without cloning.
#[inline]
pub fn i8_vec_into_u8_vec(vec: Vec<i8>) -> Vec<u8> {
    // SAFETY: Layouts of u8 and i8 are the same and we're being careful not to drop
    // the original vec after calling Vec::from_raw_parts.
    unsafe {
        let mut vec = ManuallyDrop::new(vec);
        Vec::from_raw_parts(vec.as_mut_ptr() as *mut u8, vec.len(), vec.capacity())
    }
}

/// Converts a `&[u8]` into a `&[i8]`.
#[inline]
pub fn u8_slice_as_i8_slice(slice: &[u8]) -> &[i8] {
    // SAFETY: i8 has the same layout as u8.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const i8, slice.len()) }
}

/// Converts a `&[i8]` into a `&[u8]`.
#[inline]
pub fn i8_slice_as_u8_slice(slice: &[i8]) -> &[u8] {
    // SAFETY: i8 has the same layout as u8.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len()) }
}
