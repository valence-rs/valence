use std::hint::black_box;

use criterion::Criterion;
use rand::Rng;
use valence::packet::var_int::VarInt;
use valence::packet::{Decode, Encode};

pub fn var_int(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    c.bench_function("VarInt::encode", |b| {
        b.iter_with_setup(
            || rng.gen(),
            |i| {
                let i: i32 = black_box(i);

                let mut buf = [0; VarInt::MAX_SIZE];
                let _ = black_box(VarInt(i).encode(buf.as_mut_slice()));
            },
        );
    });

    c.bench_function("VarInt::decode", |b| {
        b.iter_with_setup(
            || {
                let mut buf = [0; VarInt::MAX_SIZE];
                VarInt(rng.gen()).encode(buf.as_mut_slice()).unwrap();
                buf
            },
            |buf| {
                let mut r = black_box(buf.as_slice());
                let _ = black_box(VarInt::decode(&mut r));
            },
        )
    });
}
