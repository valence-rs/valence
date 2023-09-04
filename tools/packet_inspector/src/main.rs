#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tri_checkbox;

mod app;
mod shared_state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions {
        icon_data: Some(load_icon()),
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

fn load_icon() -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../../../assets/logo-256x256.png");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}
