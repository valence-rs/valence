pub mod block;

pub mod attributes {
    /// An attribute modifier operation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum EntityAttributeOperation {
        /// Adds the modifier to the base value.
        Add,
        /// Multiplies the modifier with the base value.
        MultiplyBase,
        /// Multiplies the modifier with the total value.
        MultiplyTotal,
    }

    impl EntityAttributeOperation {
        /// Converts from a raw [`u8`].
        pub fn from_raw(raw: u8) -> Option<Self> {
            match raw {
                0 => Some(Self::Add),
                1 => Some(Self::MultiplyBase),
                2 => Some(Self::MultiplyTotal),
                _ => None,
            }
        }

        /// Converts to a raw [`u8`].
        pub fn to_raw(self) -> u8 {
            match self {
                Self::Add => 0,
                Self::MultiplyBase => 1,
                Self::MultiplyTotal => 2,
            }
        }
    }
}

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

pub mod status_effects {
    include!(concat!(env!("OUT_DIR"), "/status_effects.rs"));
}
