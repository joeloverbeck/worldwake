use egui::{Align2, Color32, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use worldwake_core::{Permille, ThresholdBand};

const LABEL_WIDTH: f32 = 72.0;
const VALUE_WIDTH: f32 = 34.0;
const GAP: f32 = 8.0;
const BAR_HEIGHT: f32 = 10.0;
const ROW_HEIGHT: f32 = 18.0;

const GREEN: Color32 = Color32::from_rgb(90, 180, 90);
const YELLOW_GREEN: Color32 = Color32::from_rgb(170, 200, 80);
const AMBER: Color32 = Color32::from_rgb(230, 180, 60);
const ORANGE_RED: Color32 = Color32::from_rgb(230, 110, 60);
const RED: Color32 = Color32::from_rgb(220, 60, 60);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeedZone {
    Low,
    Medium,
    High,
    Critical,
    Severe,
}

impl NeedZone {
    #[must_use]
    pub const fn color(self) -> Color32 {
        match self {
            Self::Low => GREEN,
            Self::Medium => YELLOW_GREEN,
            Self::High => AMBER,
            Self::Critical => ORANGE_RED,
            Self::Severe => RED,
        }
    }
}

#[must_use]
pub fn classify_zone(value: Permille, thresholds: &ThresholdBand) -> NeedZone {
    if value < thresholds.low() {
        NeedZone::Low
    } else if value < thresholds.medium() {
        NeedZone::Medium
    } else if value < thresholds.high() {
        NeedZone::High
    } else if value < thresholds.critical() {
        NeedZone::Critical
    } else {
        NeedZone::Severe
    }
}

pub fn need_bar(
    ui: &mut Ui,
    label: &str,
    value: Permille,
    thresholds: &ThresholdBand,
    width: f32,
) -> egui::Response {
    let total_width = LABEL_WIDTH + GAP + width + GAP + VALUE_WIDTH;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(total_width, ROW_HEIGHT), Sense::hover());
    let text_color = ui.visuals().text_color();
    let font = FontId::proportional(12.0);

    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        label,
        font.clone(),
        text_color,
    );

    let bar_left = rect.left() + LABEL_WIDTH + GAP;
    let bar_rect = Rect::from_min_size(
        egui::pos2(bar_left, rect.center().y - BAR_HEIGHT / 2.0),
        Vec2::new(width, BAR_HEIGHT),
    );
    ui.painter()
        .rect_filled(bar_rect, 3, Color32::from_rgb(42, 42, 48));
    ui.painter().rect_stroke(
        bar_rect,
        3,
        Stroke::new(1.0, Color32::from_rgb(82, 82, 90)),
        StrokeKind::Outside,
    );

    let fill_width = width * f32::from(value.value()) / 1000.0;
    if fill_width > 0.0 {
        let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_width, BAR_HEIGHT));
        ui.painter()
            .rect_filled(fill_rect, 3, classify_zone(value, thresholds).color());
    }

    for tick in [
        thresholds.low(),
        thresholds.medium(),
        thresholds.high(),
        thresholds.critical(),
    ] {
        let x = bar_rect.left() + width * f32::from(tick.value()) / 1000.0;
        ui.painter().line_segment(
            [
                egui::pos2(x, bar_rect.top()),
                egui::pos2(x, bar_rect.bottom()),
            ],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(245, 245, 248, 170)),
        );
    }

    if classify_zone(value, thresholds) == NeedZone::Severe {
        let pulse = ui.input(|input| {
            let phase = (input.time * 4.0).sin() as f32;
            0.6 + 0.4 * phase
        });
        let alpha = (pulse.clamp(0.0, 1.0) * 255.0).round() as u8;
        ui.painter().rect_stroke(
            bar_rect.expand(1.0),
            3,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 96, 96, alpha)),
            StrokeKind::Outside,
        );
    }

    ui.painter().text(
        rect.right_center(),
        Align2::RIGHT_CENTER,
        value.value().to_string(),
        font,
        text_color,
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn need_bar_zone_classification() {
        let thresholds = ThresholdBand::new(pm(200), pm(400), pm(650), pm(850)).unwrap();
        let cases = [
            (pm(199), NeedZone::Low, GREEN),
            (pm(200), NeedZone::Medium, YELLOW_GREEN),
            (pm(400), NeedZone::High, AMBER),
            (pm(650), NeedZone::Critical, ORANGE_RED),
            (pm(850), NeedZone::Severe, RED),
        ];

        for (value, zone, color) in cases {
            let actual = classify_zone(value, &thresholds);
            assert_eq!(actual, zone);
            assert_eq!(actual.color(), color);
        }
    }

    const fn pm(value: u16) -> Permille {
        Permille::new_unchecked(value)
    }
}
