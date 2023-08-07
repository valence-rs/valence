pub mod block;

pub mod item {
    include!(concat!(env!("OUT_DIR"), "/item.rs"));
}

pub mod sound {
    include!(concat!(env!("OUT_DIR"), "/sound.rs"));
}

/// Contains constants for every vanilla packet ID.
pub mod packet_id {
    include!(concat!(env!("OUT_DIR"), "/packet_id.rs"));
}

pub mod chunk_view {
    include!(concat!(env!("OUT_DIR"), "/chunk_view.rs"));
}
