pub mod packets;
pub mod protocol;

/// Used only by macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use crate::protocol::Packet;
}

extern crate self as valence_packet;