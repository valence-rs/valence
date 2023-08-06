pub mod block;

pub mod item {
    include!(concat!(env!("OUT_DIR"), "/item.rs"));
}

pub mod translation_key {
    include!(concat!(env!("OUT_DIR"), "/translation_key.rs"));
}

pub mod sound {
    include!(concat!(env!("OUT_DIR"), "/sound.rs"));
}
