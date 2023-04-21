use std::hint::black_box;

use criterion::Criterion;
use valence::packet::{Decode, Encode};

pub fn decode_array(c: &mut Criterion) {
    let floats = [123.0, 456.0, 789.0];
    let mut buf = [0u8; 24];

    floats.encode(buf.as_mut_slice()).unwrap();

    c.bench_function("<[f64; 3]>::decode", |b| {
        b.iter(|| {
            let mut r = black_box(buf.as_slice());
            let _ = black_box(<[f64; 3]>::decode(&mut r));
        });
    });

    let bytes = [42; 4096];

    c.bench_function("<[u8; 4096]>::decode", |b| {
        b.iter(|| {
            let mut r = black_box(bytes.as_slice());
            let _ = black_box(<[u8; 4096]>::decode(&mut r));
        })
    });
}
