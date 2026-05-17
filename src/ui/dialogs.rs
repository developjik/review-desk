use crate::domain::ReviewEvent;
use crate::ui::components::{
    danger_button, inline_error, metadata, primary_button, secondary_button, status_badge,
    status_banner,
};
use crate::ui::state::{AppViewState, DialogState, StatusSeverity};
use crate::ui::theme;
use egui::{Context, RichText, Window};

pub fn render(ctx: &Context, state: &mut AppViewState) {
    let Some(dialog) = state.dialog.clone() else {
        return;
    };

    match dialog {
        DialogState::SubmitConfirm {
            repository,
            pull_request,
            event,
            head_sha,
            stale_check,
        } => {
            Window::new("Submit Review")
                .collapsible(false)
                .resizable(false)
                .min_width(420.0)
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("Final confirmation")
                            .strong()
                            .size(18.0)
                            .color(theme::text_primary()),
                    );
                    metadata(ui, "ReviewDesk will submit this review with your GitHub account.");
                    ui.separator();

                    metadata(ui, format!("Repository: {repository}"));
                    metadata(ui, format!("Pull request: {pull_request}"));
                    metadata(ui, format!("Head SHA: {head_sha}"));
                    ui.horizontal_wrapped(|ui| {
                        metadata(ui, "Event:");
                        match event {
                            ReviewEvent::Comment => status_badge(ui, "comment"),
                            ReviewEvent::Approve => status_badge(ui, "approve"),
                            ReviewEvent::RequestChanges => status_badge(ui, "request changes"),
                        }
                    });
                    metadata(ui, format!("Stale check: {stale_check}"));

                    if event != ReviewEvent::Comment {
                        status_banner(
                            ui,
                            StatusSeverity::Warning,
                            "This verdict changes the PR review state. Confirm only after reading the draft.",
                        );
                    }

                    ui.add_space(theme::SPACE_MD);
                    ui.horizontal(|ui| {
                        if secondary_button(ui, true, "Cancel").clicked() {
                            state.cancel_dialog();
                        }
                        let confirm = match event {
                            ReviewEvent::RequestChanges => {
                                danger_button(ui, true, "Confirm Submit")
                            }
                            _ => primary_button(ui, true, "Confirm Submit"),
                        };
                        if confirm.clicked() {
                            state.confirm_submit_dialog();
                        }
                    });
                });
        }
        DialogState::Submitting {
            repository,
            pull_request,
            ..
        } => {
            Window::new("Submitting")
                .collapsible(false)
                .resizable(false)
                .min_width(360.0)
                .show(ctx, |ui| {
                    status_banner(
                        ui,
                        StatusSeverity::Info,
                        &format!("Submitting review to {repository} {pull_request}"),
                    );
                    if primary_button(ui, true, "Simulate Success").clicked() {
                        state.apply_sample_submitted();
                    }
                    if secondary_button(ui, true, "Simulate Failure").clicked() {
                        state.apply_sample_submit_failed("GitHub submit failed; draft preserved");
                    }
                });
        }
        DialogState::Error { title, message } => {
            Window::new(title)
                .collapsible(false)
                .resizable(false)
                .min_width(320.0)
                .show(ctx, |ui| {
                    inline_error(ui, Some(&message));
                    if secondary_button(ui, true, "Close").clicked() {
                        state.cancel_dialog();
                    }
                });
        }
    }
}
