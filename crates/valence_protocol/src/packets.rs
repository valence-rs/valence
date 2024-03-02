pub mod handshake_c2s;
pub mod ping_result_s2c;
pub mod query_request_c2s;
pub mod query_response_s2c;
pub mod query_ping_c2s;

pub use handshake_c2s::HandshakeC2s;

/*
macro_rules! declare_packets {
    (
        $(
            $state:ty => {
                $(
                    $side:ty => {
                        $(
                            ($id:expr, $packet:ty)
                        )*
                    }
                )*
            }
        )*
    ) => {
        $(
            $(
                $(
                    impl crate::PacketMeta<$state, $side> for $packet {
                        const ID: i32 = $id;
                    }
                )*
            )*
        )*
    }
}

// macro_rules! decl_body {
//     ($id:expr)
// }

use crate::{Clientbound, Configuration, Handshaking, Login, Serverbound, Status, id};

declare_packets! {
    Handshaking => {
        Clientbound => {
            (id::handshaking::HANDSHAKE_C2S, HandshakeC2s<'_>)
        }
    }
}
*/
