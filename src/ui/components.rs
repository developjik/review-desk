use crate::ui::state::StatusSeverity;
use crate::ui::theme;
use egui::{
    Button, Color32, CornerRadius, Frame, Margin, RichText, Stroke, TextEdit, Ui, Vec2, vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Outline,
    Danger,
    Ghost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecklistState {
    Pass,
    Warning,
    Fail,
}

pub fn section_title(ui: &mut Ui, title: &str) {
    ui.add_space(theme::SPACE_XS);
    ui.label(
        RichText::new(title)
            .strong()
            .size(theme::TEXT_SECTION_TITLE)
            .color(theme::text_primary()),
    );
}

pub fn pane_subtitle(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(theme::TEXT_HELPER)
            .color(theme::text_muted()),
    );
}

pub fn metadata(ui: &mut Ui, text: impl Into<String>) {
    ui.label(
        RichText::new(text.into())
            .size(theme::TEXT_METADATA)
            .color(theme::text_secondary()),
    );
}

pub fn muted(ui: &mut Ui, text: impl Into<String>) {
    ui.label(
        RichText::new(text.into())
            .size(theme::TEXT_HELPER)
            .color(theme::text_muted()),
    );
}

pub fn badge(ui: &mut Ui, label: &str) {
    status_badge(ui, label);
}

pub fn status_badge(ui: &mut Ui, label: &str) {
    let color = theme::status_color(label);
    let fill = theme::status_fill(label);
    Frame::new()
        .inner_margin(Margin::symmetric(7, 2))
        .fill(fill)
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.35)))
        .corner_radius(CornerRadius::same(theme::RADIUS_SM))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(11.0).strong().color(color));
        });
}

pub fn status_banner(ui: &mut Ui, severity: StatusSeverity, message: &str) {
    let label = match severity {
        StatusSeverity::Info => "info",
        StatusSeverity::Success => "success",
        StatusSeverity::Warning => "warning",
        StatusSeverity::Error => "error",
    };
    let color = theme::status_color(label);
    Frame::new()
        .inner_margin(Margin::symmetric(10, 8))
        .fill(theme::status_fill(label))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.45)))
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status_badge(ui, label);
                ui.label(
                    RichText::new(message)
                        .size(theme::TEXT_HELPER)
                        .color(theme::text_primary()),
                );
            });
        });
}

pub fn inline_error(ui: &mut Ui, reason: Option<&str>) {
    if let Some(reason) = reason {
        ui.label(
            RichText::new(reason)
                .size(theme::TEXT_HELPER)
                .color(theme::status_color("error")),
        );
    }
}

pub fn disabled_reason(ui: &mut Ui, reason: Option<&str>) {
    inline_error(ui, reason);
}

pub fn action_button(
    ui: &mut Ui,
    enabled: bool,
    label: &str,
    variant: ButtonVariant,
) -> egui::Response {
    let (text_color, fill, stroke, min_width) = match variant {
        ButtonVariant::Primary => (
            Color32::WHITE,
            theme::primary_fill(),
            Stroke::new(1.0, theme::primary_hover_fill()),
            116.0,
        ),
        ButtonVariant::Secondary => (
            theme::text_primary(),
            theme::elevated_fill(),
            theme::stroke_subtle(),
            92.0,
        ),
        ButtonVariant::Outline => (
            theme::text_secondary(),
            Color32::TRANSPARENT,
            theme::stroke_strong(),
            84.0,
        ),
        ButtonVariant::Danger => (
            Color32::WHITE,
            theme::danger_fill(),
            Stroke::new(1.0, theme::danger_fill()),
            116.0,
        ),
        ButtonVariant::Ghost => (
            theme::text_secondary(),
            Color32::TRANSPARENT,
            Stroke::NONE,
            70.0,
        ),
    };

    let button = Button::new(
        RichText::new(label)
            .size(theme::TEXT_HELPER)
            .strong()
            .color(text_color),
    )
    .fill(fill)
    .stroke(stroke)
    .corner_radius(CornerRadius::same(theme::RADIUS_SM))
    .min_size(Vec2::new(min_width, theme::BUTTON_HEIGHT));

    ui.add_enabled(enabled, button)
}

pub fn primary_button(ui: &mut Ui, enabled: bool, label: &str) -> egui::Response {
    action_button(ui, enabled, label, ButtonVariant::Primary)
}

pub fn secondary_button(ui: &mut Ui, enabled: bool, label: &str) -> egui::Response {
    action_button(ui, enabled, label, ButtonVariant::Secondary)
}

pub fn outline_button(ui: &mut Ui, enabled: bool, label: &str) -> egui::Response {
    action_button(ui, enabled, label, ButtonVariant::Outline)
}

pub fn ghost_button(ui: &mut Ui, enabled: bool, label: &str) -> egui::Response {
    action_button(ui, enabled, label, ButtonVariant::Ghost)
}

pub fn danger_button(ui: &mut Ui, enabled: bool, label: &str) -> egui::Response {
    action_button(ui, enabled, label, ButtonVariant::Danger)
}

pub fn empty_state(ui: &mut Ui, title: &str, body: &str) {
    Frame::new()
        .inner_margin(Margin::symmetric(12, 12))
        .fill(theme::elevated_fill())
        .stroke(theme::stroke_subtle())
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong().color(theme::text_primary()));
            muted(ui, body);
        });
}

pub fn skeleton_row(ui: &mut Ui, label: &str) {
    Frame::new()
        .inner_margin(Margin::symmetric(10, 8))
        .fill(theme::disabled_fill())
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .show(ui, |ui| {
            muted(ui, label);
        });
}

pub fn summary_metric(
    ui: &mut Ui,
    label: &str,
    value: impl Into<String>,
    badge_label: Option<&str>,
) {
    Frame::new()
        .inner_margin(Margin::symmetric(10, 7))
        .fill(theme::elevated_fill())
        .stroke(theme::stroke_subtle())
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                muted(ui, label);
                if let Some(badge_label) = badge_label {
                    status_badge(ui, badge_label);
                } else {
                    ui.label(
                        RichText::new(value.into())
                            .strong()
                            .size(theme::TEXT_ROW_TITLE)
                            .color(theme::text_primary()),
                    );
                }
            });
        });
}

pub fn finding_row(
    ui: &mut Ui,
    severity: &str,
    confidence: &str,
    title: &str,
    evidence: &str,
    action: &str,
    selected_for_draft: bool,
) {
    Frame::new()
        .inner_margin(Margin::symmetric(10, 9))
        .fill(theme::elevated_fill())
        .stroke(theme::stroke_subtle())
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status_badge(ui, severity);
                status_badge(ui, confidence);
                if selected_for_draft {
                    status_badge(ui, "draft");
                }
            });
            ui.add_space(theme::SPACE_XS);
            ui.label(RichText::new(title).strong().color(theme::text_primary()));
            metadata(ui, format!("Evidence: {evidence}"));
            metadata(ui, format!("Action: {action}"));
        });
}

pub fn file_row(ui: &mut Ui, status: &str, path: &str, reason: &str, partial: bool) {
    Frame::new()
        .inner_margin(Margin::symmetric(9, 6))
        .fill(theme::elevated_fill())
        .stroke(theme::stroke_subtle())
        .corner_radius(CornerRadius::same(theme::RADIUS_SM))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status_badge(ui, status);
                ui.label(
                    RichText::new(path)
                        .monospace()
                        .size(theme::TEXT_METADATA)
                        .color(theme::text_primary()),
                );
                if partial {
                    status_badge(ui, "partial");
                }
            });
            muted(ui, reason);
        });
}

pub fn checklist_row(ui: &mut Ui, label: &str, state: ChecklistState) {
    let (badge_label, fill) = match state {
        ChecklistState::Pass => ("pass", theme::status_fill("pass")),
        ChecklistState::Warning => ("warning", theme::status_fill("warning")),
        ChecklistState::Fail => ("blocked", theme::status_fill("error")),
    };
    Frame::new()
        .inner_margin(Margin::symmetric(9, 6))
        .fill(fill)
        .stroke(theme::stroke_subtle())
        .corner_radius(CornerRadius::same(theme::RADIUS_SM))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status_badge(ui, badge_label);
                ui.label(
                    RichText::new(label)
                        .size(theme::TEXT_HELPER)
                        .color(theme::text_primary()),
                );
            });
        });
}

pub fn sticky_action_bar(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    Frame::new()
        .inner_margin(Margin::symmetric(10, 9))
        .fill(theme::elevated_fill())
        .stroke(theme::stroke_subtle())
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .show(ui, add_contents);
}

pub fn draft_text_edit<'a>(body: &'a mut String) -> TextEdit<'a> {
    TextEdit::multiline(body)
        .desired_rows(14)
        .hint_text("Write the top-level review body here.")
        .min_size(vec2(0.0, 240.0))
}
