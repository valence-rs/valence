use std::io::Read;

use egui::Color32;

use super::{SharedState, Tab, View};

pub struct HexView {}

impl Tab for HexView {
    fn new() -> Self {
        Self {}
    }

    fn name(&self) -> &'static str {
        "Hex Viewer"
    }
}

impl View for HexView {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut SharedState) {
        let mut buf = [0u8; 16];
        let mut count = 0;

        let packets = state.packets.read().unwrap();
        let Some(packet_index) = state.selected_packet else {
            return;
        };

        let bytes = &packets[packet_index].data.as_ref().unwrap();
        let mut file = &bytes.clone()[..];

        egui::Grid::new("hex_grid")
            .spacing([4.0, 1.5])
            .striped(true)
            .min_col_width(0.0)
            .show(ui, |ui| {
                ui.label(" ");
                for i in 0..16 {
                    ui.label(format!("{:02X}", i));
                }
                ui.end_row();
                loop {
                    let bytes_read = file.read(&mut buf).unwrap();
                    if bytes_read == 0 {
                        break;
                    }

                    ui.label(format!("{:08X}", count));
                    for b in buf.iter().take(bytes_read) {
                        ui.colored_label(Color32::from_rgb(255, 255, 255), format!("{:02X}", b));
                    }
                    for _ in 0..16 - bytes_read {
                        ui.label(" ");
                    }
                    ui.label(" ");
                    for b in buf.iter().take(bytes_read) {
                        if *b >= 0x20 && *b <= 0x7e {
                            ui.label(format!("{}", *b as char));
                        } else {
                            ui.label(".");
                        }
                    }

                    ui.end_row();
                    count += bytes_read;
                }
            });
    }
}
