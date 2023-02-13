use std::sync::Arc;

use context::{Context, Packet};
use packet_widget::PacketDirection;

mod context;
mod packet_widget;
mod state;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(800.0, 600.0)),
        decorated: true,
        ..Default::default()
    };

    let context = Arc::new(Context::new());

    context.add(Packet {
        id: 0,
        selected: false,
        direction: PacketDirection::ClientToServer,
        packet_type: 0x2c,
        packet_name: "Some mock packet".into(),
        packet: "raw packet".into(),
    });

    context.add(Packet {
        id: 1,
        selected: false,
        direction: PacketDirection::ServerToClient,
        packet_type: 0x2c,
        packet_name: "Ack Some mock packet".into(),
        packet: "more raw packet".into(),
    });

    context.add(Packet {
        id: 2,
        selected: false,
        direction: PacketDirection::ServerToClient,
        packet_type: 0xab,
        packet_name: "More mock packet".into(),
        packet: "haha".into(),
    });

    context.add(Packet {
        id: 3,
        selected: false,
        direction: PacketDirection::ServerToClient,
        packet_type: 0xff,
        packet_name: "server_send_data".into(),
        packet: "haha".into(),
    });

    context.add(Packet {
        id: 4,
        selected: false,
        direction: PacketDirection::ClientToServer,
        packet_type: 0xff,
        packet_name: "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW".into(),
        packet: "haha".into(),
    });

    eframe::run_native(
        "Valence Packet Inspector",
        native_options,
        Box::new(|cc| Box::new(App::new(cc, context))),
    )?;

    Ok(())
}

struct App<'a> {
    _marker: std::marker::PhantomData<&'a ()>,
    context: Arc<Context>,
    filter: String,
}

impl<'a> App<'a> {
    fn new(_cc: &eframe::CreationContext<'_>, context: Arc<Context>) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            context,
            filter: "".into(),
        }
    }
}

impl<'a> eframe::App for App<'a> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                if ui.text_edit_singleline(&mut self.filter).changed() {
                    self.context.set_filter(self.filter.clone());
                }
            });
        });

        egui::SidePanel::left("side_panel")
            .min_width(200.0)
            .show(ctx, |ui| {
                // scroll container
                ui.heading("Packets");
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for packet in self
                            .context
                            .packets
                            .write()
                            .expect("Poisoned RwLock")
                            .iter_mut()
                        {
                            {
                                let selected = self
                                    .context
                                    .selected_packet
                                    .read()
                                    .expect("Poisoned RwLock");
                                if let Some(idx) = *selected {
                                    if idx == packet.id {
                                        packet.selected(true);
                                    } else {
                                        packet.selected(false);
                                    }
                                } else {
                                    packet.selected(false);
                                }
                            }

                            if ui.add(packet.clone()).clicked() {
                                self.context.set_selected_packet(packet.id);
                                println!("Clicked {}", packet.id);
                            }
                        }
                    });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(idx) = *self
                .context
                .selected_packet
                .read()
                .expect("Poisoned RwLock")
            {
                // get the packet
                let packets = self.context.packets.read().expect("Poisoned RwLock");
                if idx < packets.len() {
                    let packet = &packets[idx];
                    let mut text = packet.packet.clone();

                    let text_editor = egui::TextEdit::multiline(&mut text)
                        .code_editor()
                        .desired_width(ui.available_width())
                        .desired_rows(24);

                    ui.add(text_editor);
                }
            }
        });
    }
}
