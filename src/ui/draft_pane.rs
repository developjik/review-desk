use crate::domain::ReviewEvent;
use crate::ui::components::{
    ChecklistState, checklist_row, draft_text_edit, inline_error, metadata, muted, outline_button,
    primary_button, secondary_button, section_title, status_badge, sticky_action_bar,
};
use crate::ui::state::AppViewState;
use crate::ui::theme;
use egui::{ComboBox, RichText, ScrollArea, Ui};

pub fn render(ui: &mut Ui, state: &mut AppViewState) {
    section_title(ui, "Review Draft");
    render_event_selector(ui, state);
    ui.add_space(theme::SPACE_SM);

    let action_bar_height = 118.0;
    let editor_height = (ui.available_height() - action_bar_height).max(260.0);
    ScrollArea::vertical()
        .id_salt("draft_scroll")
        .max_height(editor_height)
        .show(ui, |ui| {
            let response = ui.add(draft_text_edit(&mut state.draft.body));
            if response.changed() {
                state.draft.dirty = true;
            }

            ui.add_space(theme::SPACE_MD);
            render_safety_checklist(ui, state);

            ui.add_space(theme::SPACE_MD);
            metadata(ui, format!("Markdown report: {}", state.draft.report_path));
            if let Some(id) = state.draft.submitted_review_id {
                ui.label(
                    RichText::new(format!("Submitted #{id}"))
                        .strong()
                        .color(theme::status_color("submitted")),
                );
            }
            if let Some(error) = &state.draft.last_error {
                inline_error(ui, Some(error));
            }
        });

    ui.add_space(theme::SPACE_SM);
    render_action_bar(ui, state);
}

fn render_event_selector(ui: &mut Ui, state: &mut AppViewState) {
    ui.horizontal_wrapped(|ui| {
        metadata(ui, "Verdict");
        let mut next_event = state.draft.event;
        ComboBox::from_id_salt("review_event_selector")
            .selected_text(format!("{:?}", state.draft.event))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut next_event, ReviewEvent::Comment, "COMMENT");
                ui.selectable_value(&mut next_event, ReviewEvent::Approve, "APPROVE");
                ui.selectable_value(
                    &mut next_event,
                    ReviewEvent::RequestChanges,
                    "REQUEST_CHANGES",
                );
            });
        if next_event != state.draft.event {
            state.select_review_event(next_event);
        }

        match state.draft.event {
            ReviewEvent::Comment => status_badge(ui, "comment"),
            ReviewEvent::Approve => status_badge(ui, "approve"),
            ReviewEvent::RequestChanges => status_badge(ui, "request changes"),
        }
    });
    if state.draft.event != ReviewEvent::Comment {
        muted(
            ui,
            "APPROVE / REQUEST_CHANGES는 submit confirmation에서 한 번 더 확인합니다.",
        );
    }
}

fn render_safety_checklist(ui: &mut Ui, state: &AppViewState) {
    section_title(ui, "Safety checklist");
    checklist_row(
        ui,
        "GitHub connected",
        pass_fail(state.auth.is_github_connected()),
    );
    checklist_row(
        ui,
        "PR selected",
        pass_fail(state.queue.selected_pr.is_some()),
    );
    checklist_row(ui, "Head SHA not stale", pass_fail(!state.draft.stale));
    checklist_row(
        ui,
        "Draft body not empty",
        pass_fail(!state.draft.body.trim().is_empty()),
    );
    checklist_row(
        ui,
        "Private diff consent satisfied for AI draft",
        if state.draft.generated_by_ai
            && state.run.private_diff_consent_required
            && !state.run.private_diff_consent_accepted
        {
            ChecklistState::Fail
        } else {
            ChecklistState::Pass
        },
    );
    checklist_row(
        ui,
        "APPROVE / REQUEST_CHANGES explicitly selected",
        if state.draft.event == ReviewEvent::Comment || state.draft.explicit_event_selected {
            ChecklistState::Pass
        } else {
            ChecklistState::Fail
        },
    );
    checklist_row(
        ui,
        "Manual draft path available when AI is blocked",
        if state.draft.generated_by_ai {
            ChecklistState::Warning
        } else {
            ChecklistState::Pass
        },
    );
}

fn render_action_bar(ui: &mut Ui, state: &mut AppViewState) {
    let submit_reason = state.submit_disabled_reason();
    sticky_action_bar(ui, |ui| {
        inline_error(ui, submit_reason.map(|reason| reason.label()));
        ui.horizontal_wrapped(|ui| {
            if secondary_button(ui, state.queue.selected_pr.is_some(), "Save Draft").clicked() {
                state.draft.dirty = false;
                state.bottom_status.message = "Draft saved locally".to_string();
            }
            if outline_button(ui, !state.draft.body.is_empty(), "Copy Draft").clicked() {
                ui.ctx().copy_text(state.draft.body.clone());
                state.bottom_status.message = "Draft copied".to_string();
            }
            if primary_button(ui, submit_reason.is_none(), "Submit Review").clicked() {
                state.open_submit_dialog();
            }
        });
    });
}

fn pass_fail(value: bool) -> ChecklistState {
    if value {
        ChecklistState::Pass
    } else {
        ChecklistState::Fail
    }
}
