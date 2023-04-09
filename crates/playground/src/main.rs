use tracing::Level;
use valence::bevy_app::App;

#[allow(dead_code)]
mod extras;
mod playground;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let mut app = App::new();
    playground::build_app(&mut app);
    app.run();
}
