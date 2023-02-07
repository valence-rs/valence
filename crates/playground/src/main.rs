use valence_new::prelude::App;

mod extras;
mod playground;

fn main() {
    tracing_subscriber::fmt().init();

    let mut app = App::new();
    playground::build_app(&mut app);
    app.run();
}
