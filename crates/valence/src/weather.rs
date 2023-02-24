/// The weather state representation.
#[derive(Component)]
pub struct Weather {
    /// Contains the raining level.
    /// The `None` value means there is no raining event.
    pub raining: Option<f32>,
    /// Contains the thunder level.
    /// The `None` value means there is no thunder.
    pub thunder: Option<f32>
}