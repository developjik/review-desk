use crate::domain::ReviewRunStatus;
use crate::ui::components::{
    ChecklistState, badge, checklist_row, disabled_reason, file_row, finding_row, metadata, muted,
    pane_subtitle, secondary_button, section_title, status_banner, summary_metric,
};
use crate::ui::state::{AppViewState, StatusSeverity, WorkspaceTab};
use crate::ui::theme;
use egui::{RichText, ScrollArea, Ui};

pub fn render(ui: &mut Ui, state: &mut AppViewState) {
    render_header(ui, state);
    ui.add_space(theme::SPACE_MD);
    render_summary_strip(ui, state);
    ui.add_space(theme::SPACE_MD);
    render_context_banners(ui, state);
    ui.add_space(theme::SPACE_MD);
    render_tabs(ui, state);
}

fn render_header(ui: &mut Ui, state: &mut AppViewState) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(&state.run.pr_title)
                    .strong()
                    .size(20.0)
                    .color(theme::text_primary()),
            );
            metadata(
                ui,
                format!(
                    "{} · {} · {} · {}",
                    state.run.repository_label,
                    state.run.author,
                    state.run.base_head,
                    state.run.head_sha_short
                ),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let _ = secondary_button(ui, state.queue.selected_pr.is_some(), "Open GitHub");
        });
    });
}

fn render_summary_strip(ui: &mut Ui, state: &AppViewState) {
    ui.horizontal_wrapped(|ui| {
        summary_metric(ui, "Risk", "", Some(&state.run.risk));
        summary_metric(ui, "CI", "", Some(&state.run.ci));
        summary_metric(ui, "Files", state.run.changed_files.to_string(), None);
        summary_metric(
            ui,
            "Diff",
            format!("+{} -{}", state.run.additions, state.run.deletions),
            None,
        );
        summary_metric(ui, "Run", "", Some(&format!("{:?}", state.run.status)));
        summary_metric(
            ui,
            "Suggested",
            "",
            Some(&format!("{:?}", state.run.suggested_event)),
        );
    });
}

fn render_context_banners(ui: &mut Ui, state: &mut AppViewState) {
    if state.run.status == ReviewRunStatus::AiBlockedUnsupportedAuth {
        status_banner(
            ui,
            StatusSeverity::Warning,
            "AI_BLOCKED_UNSUPPORTED_AUTH: 공식 ChatGPT OAuth provider가 필요합니다. 수동 draft 작성, 저장, 복사는 계속 가능합니다.",
        );
    }

    if state.run.private_diff_consent_required && !state.run.private_diff_consent_accepted {
        status_banner(
            ui,
            StatusSeverity::Warning,
            "Private diff consent required before AI analysis.",
        );
        ui.horizontal_wrapped(|ui| {
            badge(ui, "consent required");
            metadata(
                ui,
                format!(
                    "Provider: {} · Model: {} · Scope: sanitized diff patches and PR metadata",
                    state
                        .provider
                        .account_label
                        .as_deref()
                        .unwrap_or("official provider"),
                    state.provider.selected_model
                ),
            );
        });
        ui.checkbox(
            &mut state.run.private_diff_consent_accepted,
            "외부 AI provider 전송 범위를 확인했습니다.",
        );
    }

    disabled_reason(ui, state.analyze_disabled_reason().as_deref());
}

fn render_tabs(ui: &mut Ui, state: &mut AppViewState) {
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(
            &mut state.run.active_tab,
            WorkspaceTab::Overview,
            "Overview",
        );
        ui.selectable_value(
            &mut state.run.active_tab,
            WorkspaceTab::Findings,
            "Findings",
        );
        ui.selectable_value(&mut state.run.active_tab, WorkspaceTab::Files, "Files");
        ui.selectable_value(
            &mut state.run.active_tab,
            WorkspaceTab::Checklist,
            "Checklist",
        );
    });
    ui.separator();

    ScrollArea::vertical()
        .id_salt("workspace_scroll")
        .show(ui, |ui| match state.run.active_tab {
            WorkspaceTab::Overview => render_overview(ui, state),
            WorkspaceTab::Findings => render_findings(ui, state),
            WorkspaceTab::Files => render_files(ui, state),
            WorkspaceTab::Checklist => render_checklist(ui, state),
        });
}

fn render_overview(ui: &mut Ui, state: &AppViewState) {
    section_title(ui, "Review brief");
    pane_subtitle(ui, "먼저 확인할 변경 의도와 위험 신호입니다.");
    ui.add_space(theme::SPACE_SM);
    if state.run.overview.is_empty() {
        muted(ui, "Select a pull request to collect context.");
        return;
    }
    for item in &state.run.overview {
        ui.horizontal_wrapped(|ui| {
            badge(ui, "info");
            ui.label(RichText::new(item).color(theme::text_primary()));
        });
        ui.add_space(theme::SPACE_XS);
    }
}

fn render_findings(ui: &mut Ui, state: &AppViewState) {
    section_title(ui, "Findings");
    pane_subtitle(
        ui,
        "Severity, confidence, evidence, suggested action을 분리해 표시합니다.",
    );
    ui.add_space(theme::SPACE_SM);
    if state.run.findings.is_empty() {
        muted(ui, "No findings available.");
        return;
    }
    for finding in &state.run.findings {
        finding_row(
            ui,
            &finding.severity,
            &finding.confidence,
            &finding.title,
            &finding.evidence,
            &finding.action,
            finding.selected_for_draft,
        );
        ui.add_space(theme::SPACE_SM);
    }
}

fn render_files(ui: &mut Ui, state: &AppViewState) {
    section_title(ui, "Changed and excluded files");
    ui.add_space(theme::SPACE_SM);
    if state.run.files.is_empty() {
        muted(ui, "No changed files loaded.");
        return;
    }
    for file in &state.run.files {
        file_row(ui, &file.status, &file.path, &file.reason, file.partial);
        ui.add_space(theme::SPACE_XS);
    }
}

fn render_checklist(ui: &mut Ui, state: &AppViewState) {
    section_title(ui, "Human checklist");
    pane_subtitle(ui, "AI 결과와 별도로 사용자가 직접 확인할 항목입니다.");
    ui.add_space(theme::SPACE_SM);
    for item in &state.run.checklist {
        checklist_row(ui, item, ChecklistState::Warning);
        ui.add_space(theme::SPACE_XS);
    }
    if let Some(error) = &state.run.error {
        status_banner(ui, StatusSeverity::Warning, error);
    }
}
