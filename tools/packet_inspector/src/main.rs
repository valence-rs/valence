#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{IconData, ViewportBuilder};

mod tri_checkbox;

mod app;
mod shared_state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(egui::Vec2::new(1024.0, 768.0))
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Valence Packet Inspector",
        native_options,
        Box::new(move |cc| {
            let gui_app = app::GuiApp::new(cc);

            Ok(Box::new(gui_app))
        }),
    )?;

    Ok(())
}

fn load_icon() -> IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../../../assets/logo-256x256.png");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}
pub(crate) mod utils {
    use packet_inspector::Packet as ProxyPacket;
    use valence_protocol::{Decode, Packet};

    include!(concat!(env!("OUT_DIR"), "/packet_to_string.rs"));
}
