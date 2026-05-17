use crate::ui::components::{metadata, status_banner};
use crate::ui::state::AppViewState;
use egui::Ui;

pub fn render(ui: &mut Ui, state: &AppViewState) {
    ui.horizontal_wrapped(|ui| {
        status_banner(
            ui,
            state.bottom_status.severity,
            &state.bottom_status.message,
        );
        if let Some(task) = &state.task {
            metadata(
                ui,
                format!(
                    "Task: {} · cancellable: {} · retryable: {}",
                    task.label, task.cancellable, task.retryable
                ),
            );
            if let Some(error) = &task.last_error {
                metadata(ui, error.as_str());
            }
        }
    });
}
