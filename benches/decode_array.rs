use std::hint::black_box;

use divan::Bencher;
use valence::protocol::{Decode, Encode};

#[divan::bench]
pub fn decode_small_array(bencher: Bencher) {
    let floats = [123.0, 456.0, 789.0];
    let mut buf = [0_u8; 24];

    floats.encode(buf.as_mut_slice()).unwrap();

    bencher.bench(|| {
        let mut r = black_box(buf.as_slice());
        let _ = black_box(<[f64; 3]>::decode(&mut r));
    });
}

#[divan::bench]
pub fn decode_large_array(bencher: Bencher) {
    let bytes = [42; 4096];

    bencher.bench(|| {
            let mut r = black_box(bytes.as_slice());
            let _ = black_box(<[u8; 4096]>::decode(&mut r));
    });
}
