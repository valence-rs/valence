use egui::{
    epaint, pos2, vec2, NumExt, Response, Sense, Shape, TextStyle, Ui, Vec2, Widget, WidgetInfo,
    WidgetText, WidgetType,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TriCheckboxState {
    Enabled,
    Partial,
    Disabled,
}

// TODO(emilk): allow checkbox without a text label
/// Boolean on/off control with text label.
///
/// Usually you'd use [`Ui::checkbox`] instead.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_bool = true;
/// // These are equivalent:
/// ui.checkbox(&mut my_bool, "Checked");
/// ui.add(egui::Checkbox::new(&mut my_bool, "Checked"));
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub(crate) struct TriCheckbox<'a> {
    checked: &'a mut TriCheckboxState,
    text: WidgetText,
}

#[allow(unused)]
impl<'a> TriCheckbox<'a> {
    pub(crate) fn new(checked: &'a mut TriCheckboxState, text: impl Into<WidgetText>) -> Self {
        TriCheckbox {
            checked,
            text: text.into(),
        }
    }

    pub(crate) fn without_text(checked: &'a mut TriCheckboxState) -> Self {
        Self::new(checked, WidgetText::default())
    }
}

impl<'a> Widget for TriCheckbox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let TriCheckbox { checked, text } = self;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = spacing.icon_spacing;

        let (text, mut desired_size) = if text.is_empty() {
            (None, vec2(icon_width, 0.0))
        } else {
            let total_extra = vec2(icon_width + icon_spacing, 0.0);

            let wrap_width = ui.available_width() - total_extra.x;
            let text = text.into_galley(ui, None, wrap_width, TextStyle::Button);

            let mut desired_size = total_extra + text.size();
            desired_size = desired_size.at_least(spacing.interact_size);

            (Some(text), desired_size)
        };

        desired_size = desired_size.at_least(Vec2::splat(spacing.interact_size.y));
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            *checked = match *checked {
                TriCheckboxState::Partial | TriCheckboxState::Disabled => TriCheckboxState::Enabled,
                TriCheckboxState::Enabled => TriCheckboxState::Disabled,
            };
            response.mark_changed();
        }
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                *checked == TriCheckboxState::Enabled,
                text.as_ref().map_or("", |x| x.text()),
            )
        });

        if ui.is_rect_visible(rect) {
            // Too colorful
            // let visuals = ui.style().interact_selectable(&response, *checked);
            let visuals = ui.style().interact(&response);
            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
            ui.painter().add(epaint::RectShape {
                rect: big_icon_rect.expand(visuals.expansion),
                rounding: visuals.rounding,
                fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
            });

            match *checked {
                TriCheckboxState::Enabled => {
                    // Check mark:
                    ui.painter().add(Shape::line(
                        vec![
                            pos2(small_icon_rect.left(), small_icon_rect.center().y),
                            pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                            pos2(small_icon_rect.right(), small_icon_rect.top()),
                        ],
                        visuals.fg_stroke,
                    ));
                }
                TriCheckboxState::Partial => {
                    // Minus sign:
                    ui.painter().add(Shape::line(
                        vec![
                            pos2(small_icon_rect.left(), small_icon_rect.center().y),
                            pos2(small_icon_rect.right(), small_icon_rect.center().y),
                        ],
                        visuals.fg_stroke,
                    ));
                }
                TriCheckboxState::Disabled => {}
            }
            if let Some(text) = text {
                let text_pos = pos2(
                    rect.min.x + icon_width + icon_spacing,
                    rect.center().y - 0.5 * text.size().y,
                );
                text.paint_with_visuals(ui.painter(), text_pos, visuals);
            }
        }

        response
    }
}
