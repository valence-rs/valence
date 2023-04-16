#[derive(Copy, Clone, PartialEq, Eq, Default, Debug, Encode, Decode)]
pub enum Hand {
    #[default]
    Main,
    Off,
}
