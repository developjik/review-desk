use egui::{Color32, Stroke};

pub const LEFT_PANE_WIDTH: f32 = 328.0;
pub const RIGHT_PANE_WIDTH: f32 = 408.0;
pub const NARROW_BREAKPOINT: f32 = 960.0;

pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 16.0;

pub const RADIUS_SM: u8 = 5;
pub const RADIUS_MD: u8 = 7;

pub const TOP_BAR_HEIGHT: f32 = 58.0;
pub const BOTTOM_STATUS_HEIGHT: f32 = 42.0;
pub const QUEUE_ROW_HEIGHT: f32 = 60.0;
pub const BUTTON_HEIGHT: f32 = 32.0;

pub const TEXT_APP_TITLE: f32 = 18.0;
pub const TEXT_SECTION_TITLE: f32 = 14.0;
pub const TEXT_ROW_TITLE: f32 = 14.0;
pub const TEXT_METADATA: f32 = 12.0;
pub const TEXT_HELPER: f32 = 12.0;

pub fn app_background() -> Color32 {
    Color32::from_rgb(238, 241, 245)
}

pub fn panel_fill() -> Color32 {
    Color32::from_rgb(248, 249, 251)
}

pub fn elevated_fill() -> Color32 {
    Color32::from_rgb(255, 255, 255)
}

pub fn input_fill() -> Color32 {
    Color32::from_rgb(252, 253, 255)
}

pub fn hover_fill() -> Color32 {
    Color32::from_rgb(239, 244, 252)
}

pub fn selected_fill() -> Color32 {
    Color32::from_rgb(226, 236, 251)
}

pub fn disabled_fill() -> Color32 {
    Color32::from_rgb(240, 242, 245)
}

pub fn text_primary() -> Color32 {
    Color32::from_rgb(31, 41, 55)
}

pub fn text_secondary() -> Color32 {
    Color32::from_rgb(78, 89, 105)
}

pub fn text_muted() -> Color32 {
    Color32::from_rgb(112, 123, 138)
}

pub fn stroke_subtle() -> Stroke {
    Stroke::new(1.0, Color32::from_rgb(219, 225, 232))
}

pub fn stroke_strong() -> Stroke {
    Stroke::new(1.0, Color32::from_rgb(164, 176, 194))
}

pub fn stroke_focus() -> Stroke {
    Stroke::new(1.5, Color32::from_rgb(47, 111, 198))
}

pub fn status_color(label: &str) -> Color32 {
    match label.to_ascii_lowercase().as_str() {
        "connected" | "ready" | "pass" | "submitted" | "low" | "success" => {
            Color32::from_rgb(38, 123, 75)
        }
        "blocked" | "auth required" | "pending" | "medium" | "warning" | "consent required"
        | "ai blocked" => Color32::from_rgb(174, 107, 28),
        "scope missing" | "sso required" | "fail" | "stale" | "high" | "error"
        | "request changes" => Color32::from_rgb(180, 54, 61),
        "selected" | "draft" | "comment" | "info" | "approve" => Color32::from_rgb(47, 111, 198),
        _ => Color32::from_rgb(82, 95, 115),
    }
}

pub fn status_fill(label: &str) -> Color32 {
    match label.to_ascii_lowercase().as_str() {
        "connected" | "ready" | "pass" | "submitted" | "low" | "success" => {
            Color32::from_rgb(230, 246, 237)
        }
        "blocked" | "auth required" | "pending" | "medium" | "warning" | "consent required"
        | "ai blocked" => Color32::from_rgb(255, 244, 224),
        "scope missing" | "sso required" | "fail" | "stale" | "high" | "error"
        | "request changes" => Color32::from_rgb(253, 232, 232),
        "selected" | "draft" | "comment" | "info" | "approve" => Color32::from_rgb(230, 239, 253),
        _ => Color32::from_rgb(238, 242, 247),
    }
}

pub fn primary_fill() -> Color32 {
    Color32::from_rgb(34, 104, 201)
}

pub fn primary_hover_fill() -> Color32 {
    Color32::from_rgb(28, 88, 172)
}

pub fn danger_fill() -> Color32 {
    Color32::from_rgb(180, 54, 61)
}
