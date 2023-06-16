mod tri_checkbox;

mod app;
mod shared_state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(1024.0, 768.0)),
        decorated: true,
        ..Default::default()
    };

    eframe::run_native(
        "Valence Packet Inspector",
        native_options,
        Box::new(move |cc| {
            let gui_app = app::GuiApp::new(cc);

            Box::new(gui_app)
        }),
    )?;

    Ok(())
}
