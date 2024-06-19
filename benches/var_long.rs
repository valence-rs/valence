use std::hint::black_box;

use rand::Rng;
use divan::Bencher;
use valence::protocol::{Decode, Encode, VarLong};

#[divan::bench]
fn varlong_encode(bencher: Bencher) {
    let mut rng = rand::thread_rng();

    bencher.with_inputs(|| rng.gen()).bench_local_values(|i| {
        let i: i64 = black_box(i);

        let mut buf = [0; VarLong::MAX_SIZE];
        let _ = black_box(VarLong(i).encode(buf.as_mut_slice()));
    });
}

#[divan::bench]
fn varlong_decode(bencher: Bencher) {
    let mut rng = rand::thread_rng();

    bencher.with_inputs(|| {
        let mut buf = [0; VarLong::MAX_SIZE];
        VarLong(rng.gen()).encode(buf.as_mut_slice()).unwrap();
        buf
    }).bench_local_values(|buf| {
        let mut r = black_box(buf.as_slice());
        let _ = black_box(VarLong::decode(&mut r));
    });
}
