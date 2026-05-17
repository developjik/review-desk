use crate::domain::PullRequestQueueItem;
use crate::ui::components::{
    badge, empty_state, inline_error, metadata, muted, pane_subtitle, secondary_button,
    section_title, skeleton_row,
};
use crate::ui::state::{AppViewState, DirectPrValidation, QueueFilter};
use crate::ui::theme;
use egui::{CollapsingHeader, ComboBox, CornerRadius, Frame, Margin, RichText, ScrollArea, Ui};

pub fn render(ui: &mut Ui, state: &mut AppViewState) {
    ui.vertical(|ui| {
        section_title(ui, "Review Queue");
        pane_subtitle(
            ui,
            "OAuth 계정 기준으로 review requested / assigned PR을 선택합니다.",
        );
        ui.add_space(theme::SPACE_SM);

        ui.add_enabled_ui(state.auth.is_github_connected(), |ui| {
            ui.horizontal(|ui| {
                if secondary_button(ui, state.can_refresh_queue(), "My Queue").clicked() {
                    state.queue.loading = false;
                    state.bottom_status.message = "My review queue refreshed".to_string();
                }
                ComboBox::from_id_salt("repository_selector")
                    .selected_text(
                        state
                            .queue
                            .selected_repository
                            .as_deref()
                            .unwrap_or("Select repository"),
                    )
                    .show_ui(ui, |ui| {
                        for repository in &state.queue.repositories {
                            ui.selectable_value(
                                &mut state.queue.selected_repository,
                                Some(repository.clone()),
                                repository,
                            );
                        }
                    });
            });

            ui.add_space(theme::SPACE_SM);
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut state.queue.filter, QueueFilter::All, "All");
                ui.selectable_value(
                    &mut state.queue.filter,
                    QueueFilter::ReviewRequested,
                    "Review requested",
                );
                ui.selectable_value(&mut state.queue.filter, QueueFilter::Assigned, "Assigned");
            });

            ui.add_space(theme::SPACE_SM);
            ui.text_edit_singleline(&mut state.queue.search)
                .on_hover_text("Search title, repo, or PR number");

            if let Some(error) = &state.queue.error {
                inline_error(ui, Some(error));
            }

            ui.add_space(theme::SPACE_MD);
            if state.queue.loading {
                skeleton_row(ui, "Loading review queue");
            } else {
                render_queue_rows(ui, state);
            }

            ui.add_space(theme::SPACE_MD);
            render_direct_pr_fallback(ui, state);
        });

        if !state.auth.is_github_connected() {
            empty_state(
                ui,
                "GitHub OAuth required",
                "GitHub 연결 후 repository와 PR queue를 불러올 수 있습니다.",
            );
        }
    });
}

fn render_queue_rows(ui: &mut Ui, state: &mut AppViewState) {
    let filtered = state
        .queue
        .filtered_items()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        empty_state(
            ui,
            "No open review requested or assigned PRs",
            "Refresh the queue, change repository, or use Direct PR fallback.",
        );
        return;
    }

    ScrollArea::vertical()
        .id_salt("queue_scroll")
        .show(ui, |ui| {
            for item in filtered {
                let pr_ref = format!("{}/{}#{}", item.owner, item.repo, item.number);
                let selected = state.queue.selected_pr.as_deref() == Some(pr_ref.as_str());
                if queue_row(ui, &item, selected) {
                    state.select_pr(pr_ref);
                }
                ui.add_space(theme::SPACE_XS);
            }
        });
}

fn queue_row(ui: &mut Ui, item: &PullRequestQueueItem, selected: bool) -> bool {
    let fill = if selected {
        theme::selected_fill()
    } else {
        theme::elevated_fill()
    };
    let stroke = if selected {
        theme::stroke_focus()
    } else {
        theme::stroke_subtle()
    };

    let mut clicked = false;
    Frame::new()
        .inner_margin(Margin::symmetric(10, 8))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .show(ui, |ui| {
            ui.set_min_height(theme::QUEUE_ROW_HEIGHT - 12.0);
            ui.horizontal(|ui| {
                let title = format!("#{} {}", item.number, item.title);
                let response = ui.selectable_label(
                    selected,
                    RichText::new(title)
                        .strong()
                        .size(theme::TEXT_ROW_TITLE)
                        .color(theme::text_primary()),
                );
                if response.clicked() {
                    clicked = true;
                }
            });
            ui.horizontal_wrapped(|ui| {
                metadata(
                    ui,
                    format!("{}/{} by {}", item.owner, item.repo, item.author),
                );
                if item.review_requested {
                    badge(ui, "review requested");
                }
                if item.assigned {
                    badge(ui, "assigned");
                }
                if let Some(ci) = &item.ci_status {
                    badge(ui, ci);
                }
                if let Some(count) = item.changed_files_count {
                    muted(ui, format!("{count} files"));
                }
            });
        });

    clicked
}

fn render_direct_pr_fallback(ui: &mut Ui, state: &mut AppViewState) {
    CollapsingHeader::new("Direct PR fallback")
        .default_open(false)
        .show(ui, |ui| {
            pane_subtitle(ui, "Queue에 없는 PR을 임시로 확인할 때만 사용합니다.");
            let response = ui.text_edit_singleline(&mut state.queue.direct_pr_input);
            if response.changed() {
                state.queue.direct_pr_error =
                    match AppViewState::validate_direct_pr_input(&state.queue.direct_pr_input) {
                        DirectPrValidation::Valid(reference) => {
                            state.queue.selected_pr = Some(reference.to_string());
                            None
                        }
                        DirectPrValidation::Invalid(_)
                            if state.queue.direct_pr_input.trim().is_empty() =>
                        {
                            None
                        }
                        DirectPrValidation::Invalid(error) => Some(error),
                    };
            }
            inline_error(ui, state.queue.direct_pr_error.as_deref());
        });
}
