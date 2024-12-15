use super::{SharedState, Tab, View};
use crate::app::packet_list::systemtime_strftime;
use eframe::emath::{Pos2, Vec2};
use eframe::epaint::text::TextWrapMode;
use eframe::epaint::{Color32, Rgba, Shape, Stroke};
use egui::{Response, Sense, TextStyle, Ui, WidgetText};
use packet_inspector::Packet;

pub(crate) struct FailedPackets;

impl Tab for FailedPackets {
    fn new() -> Self {
        Self {}
    }

    fn name(&self) -> &'static str {
        "Failed Packets"
    }
}

impl View for FailedPackets {
    fn ui(&mut self, ui: &mut Ui, state: &mut SharedState) {
        let packets = state.failed_packets.read().unwrap();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(!state.update_scroll)
            .show(ui, |ui| {
                for (packet, i) in packets.iter() {
                    if let Some(filtered) = state.packet_filter.get(packet) {
                        if !filtered {
                            continue;
                        }
                    }

                    let selected = {
                        if let Some(selected) = state.selected_packet {
                            selected == *i
                        } else {
                            false
                        }
                    };

                    let widget = draw_packet_widget(ui, packet, selected);

                    if state.update_scroll && state.selected_packet == Some(*i) {
                        state.update_scroll = false;
                        ui.scroll_to_rect(widget.rect, None);
                    }

                    if widget.clicked() {
                        state.selected_packet = Some(*i);
                    }
                }
            });
    }
}
fn draw_packet_widget(ui: &mut Ui, packet: &Packet, selected: bool) -> Response {
    let (mut rect, response) = ui.allocate_at_least(
        Vec2 {
            x: ui.available_width(),
            y: 24.0,
        },
        Sense::click(),
    ); // this should give me a new rect inside the scroll area... no?

    let fill = if selected {
        Rgba::from_rgba_premultiplied(0.1, 0.1, 0.1, 0.5)
    } else {
        Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 0.0)
    };

    let text_color: Color32 = if selected {
        Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 1.0).into()
    } else {
        ui.visuals().strong_text_color()
    };

    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect(rect, 0.0, fill, Stroke::new(1.0, Rgba::BLACK));

        let shape = crate::app::packet_list::get_triangle(packet.side, &rect);
        ui.painter().add(Shape::Path(shape));

        let identifier: WidgetText = format!("0x{:0>2X?}", packet.id).into();

        let identifier = identifier.into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            rect.width() - 21.0,
            TextStyle::Button,
        );

        let label: WidgetText = packet.name.into();
        let label = label.into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            rect.width() - 60.0,
            TextStyle::Button,
        );

        let timestamp: WidgetText = systemtime_strftime(packet.timestamp.unwrap()).into();
        let timestamp = timestamp.into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            rect.width() - 60.0,
            TextStyle::Button,
        );

        let id_and_timestamp_color = if selected {
            text_color
        } else {
            ui.visuals().weak_text_color()
        };

        ui.painter().galley(
            Pos2 {
                x: rect.left() + 21.0,
                y: rect.top() + 6.0,
            },
            identifier,
            id_and_timestamp_color,
        );

        rect.set_width(rect.width() - 5.0);

        let label_width = label.size().x + 50.0;
        ui.painter().galley(
            Pos2 {
                x: rect.left() + 55.0,
                y: rect.top() + 6.0,
            },
            label,
            text_color,
        );

        ui.painter().galley(
            Pos2 {
                x: rect.left() + label_width + 8.0,
                y: rect.top() + 6.0,
            },
            timestamp,
            id_and_timestamp_color,
        );
    }

    response
}
