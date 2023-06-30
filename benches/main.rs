use criterion::{criterion_group, criterion_main};

mod anvil;
mod block;
mod decode_array;
mod idle;
mod packet;
mod var_int;
mod var_long;

criterion_group! {
    benches,
    // anvil::load,
    block::block,
    decode_array::decode_array,
    idle::idle_update,
    packet::packet,
    var_int::var_int,
    var_long::var_long,
}

criterion_main!(benches);
