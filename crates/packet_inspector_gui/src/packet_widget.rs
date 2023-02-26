use eframe::epaint::{PathShape, RectShape};
use egui::{
    Pos2, Rect, Response, Rgba, Rounding, Sense, Shape, Stroke, TextStyle, Ui, Vec2, Widget,
    WidgetText,
};
use time::OffsetDateTime;

use crate::context::Packet;

pub fn systemtime_strftime(odt: OffsetDateTime) -> String {
    let hour = odt.hour();
    let minute = odt.minute();
    let second = odt.second();
    let millis = odt.millisecond();

    format!("{hour:0>2}:{minute:0>2}:{second:0>2}.{millis:0>4}")
}

#[derive(Clone)]
pub enum PacketDirection {
    ClientToServer,
    ServerToClient,
}

impl PacketDirection {
    fn get_shape(&self, outer_rect: &Rect) -> PathShape {
        let rect = Rect::from_min_size(
            Pos2 {
                x: outer_rect.left() + 6.0,
                y: outer_rect.top() + 8.0,
            },
            Vec2 { x: 8.0, y: 8.0 },
        );

        let color = match self {
            PacketDirection::ServerToClient => Rgba::from_rgb(255.0, 0.0, 0.0),
            PacketDirection::ClientToServer => Rgba::from_rgb(0.0, 255.0, 0.0),
        };

        let points = match self {
            PacketDirection::ServerToClient => vec![
                Pos2 {
                    x: rect.left() + (rect.width() / 2.0),
                    y: rect.top() + rect.height(),
                },
                Pos2 {
                    x: rect.left() + 0.0,
                    y: rect.top(),
                },
                Pos2 {
                    x: rect.left() + rect.width(),
                    y: rect.top(),
                },
            ],
            PacketDirection::ClientToServer => vec![
                Pos2 {
                    x: rect.left() + (rect.width() / 2.0),
                    y: rect.top() + 0.0,
                },
                Pos2 {
                    x: rect.left() + 0.0,
                    y: rect.top() + rect.height(),
                },
                Pos2 {
                    x: rect.left() + rect.width(),
                    y: rect.top() + rect.height(),
                },
            ],
        };

        let mut shape = PathShape::closed_line(points, Stroke::new(2.0, color));
        shape.fill = color.into();

        shape
    }
}

impl Widget for Packet {
    fn ui(self, ui: &mut Ui) -> Response {
        let (mut rect, response) = ui.allocate_at_least(
            Vec2 {
                x: ui.available_width(),
                y: 24.0,
            },
            Sense::click(),
        );

        let fill = match self.selected {
            true => Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 0.4),
            false => Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 0.0),
        };

        if ui.is_rect_visible(rect) {
            ui.painter().add(Shape::Rect(RectShape {
                rect,
                rounding: Rounding::none(),
                fill: fill.into(),
                stroke: Stroke::new(1.0, Rgba::BLACK),
            }));

            let shape = self.direction.get_shape(&rect);
            ui.painter().add(Shape::Path(shape));

            let identifier: WidgetText = format!("0x{:0>2X?}", self.packet_type).into();

            let identifier =
                identifier.into_galley(ui, Some(false), rect.width() - 21.0, TextStyle::Button);

            let label: WidgetText = self.packet_name.into();
            let label = label.into_galley(ui, Some(false), rect.width() - 60.0, TextStyle::Button);

            let timestamp: WidgetText = systemtime_strftime(self.created_at).into();
            let timestamp =
                timestamp.into_galley(ui, Some(false), rect.width() - 60.0, TextStyle::Button);

            identifier.paint_with_fallback_color(
                ui.painter(),
                Pos2 {
                    x: rect.left() + 21.0,
                    y: rect.top() + 6.0,
                },
                ui.visuals().weak_text_color(),
            );

            rect.set_width(rect.width() - 5.0);

            let label_width = label.size().x + 50.0;

            label.paint_with_fallback_color(
                &ui.painter().with_clip_rect(rect),
                Pos2 {
                    x: rect.left() + 55.0,
                    y: rect.top() + 6.0,
                },
                ui.visuals().strong_text_color(),
            );

            timestamp.paint_with_fallback_color(
                &ui.painter().with_clip_rect(rect),
                Pos2 {
                    x: rect.left() + label_width + 8.0,
                    y: rect.top() + 6.0,
                },
                ui.visuals().weak_text_color(),
            );
        }

        response
    }
}
