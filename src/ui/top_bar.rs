use crate::domain::{AiProviderStatus, ReasoningDepth};
use crate::ui::components::{
    badge, ghost_button, metadata, primary_button, secondary_button, status_badge,
};
use crate::ui::state::{AppViewState, AuthViewState, QueueViewState, RunViewState};
use crate::ui::theme;
use egui::{ComboBox, RichText, Ui};

pub fn render(ui: &mut Ui, state: &mut AppViewState) {
    let width = ui.available_width();
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("ReviewDesk")
                .strong()
                .size(theme::TEXT_APP_TITLE)
                .color(theme::text_primary()),
        );
        ui.separator();
        metadata(ui, "GitHub");
        badge(ui, state.auth.badge_label());
        if let AuthViewState::GitHubConnected { login } = &state.auth {
            metadata(ui, login.as_str());
        }
        if primary_button(ui, !state.auth.is_github_connected(), "GitHub OAuth").clicked() {
            state.auth = AuthViewState::GitHubConnected {
                login: "developjik".to_string(),
            };
            state.queue = QueueViewState::sample_loaded();
            state.run = RunViewState::sample_ai_blocked();
            state.bottom_status.message = "GitHub connected".to_string();
        }
        ui.separator();
        metadata(ui, "AI");
        let ai_label = match state.provider.status {
            AiProviderStatus::Ready => "Ready",
            AiProviderStatus::AuthRequired => "Auth required",
            AiProviderStatus::BlockedUnsupportedAuth => "Blocked",
            AiProviderStatus::ModelListUnavailable => "Model unavailable",
        };
        status_badge(ui, ai_label);

        if width > 760.0 {
            ComboBox::from_id_salt("model_selector")
                .selected_text(&state.provider.selected_model)
                .show_ui(ui, |ui| {
                    for model in &state.provider.available_models {
                        ui.selectable_value(
                            &mut state.provider.selected_model,
                            model.clone(),
                            model,
                        );
                    }
                });
        }

        if width > 920.0 {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.provider.selected_depth,
                    ReasoningDepth::Fast,
                    "빠름",
                );
                ui.selectable_value(
                    &mut state.provider.selected_depth,
                    ReasoningDepth::Balanced,
                    "균형",
                );
                ui.selectable_value(
                    &mut state.provider.selected_depth,
                    ReasoningDepth::Deep,
                    "깊게",
                );
            });
        }
        ui.separator();
        if secondary_button(ui, state.can_refresh_queue(), "Refresh").clicked() {
            state.queue.loading = false;
            state.bottom_status.message = "Review queue refreshed from local fixture".to_string();
        }
        let _ = ghost_button(ui, true, "Settings");
        if let Some(task) = &state.task {
            metadata(ui, task.label.as_str());
        }
    });
}
