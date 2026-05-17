use reviewdesk::domain::ReviewEvent;
use reviewdesk::ui::state::{
    AppViewState, DialogState, DirectPrValidation, QueueFilter, SubmitDisabledReason, ViewportMode,
    WorkspaceSection, required_snapshot_names, viewport_mode_for_width,
};

#[test]
fn startup_state_disables_github_dependent_actions() {
    let state = AppViewState::startup_auth_required();

    assert!(!state.can_refresh_queue());
    assert!(!state.can_select_pr());
    assert!(!state.can_analyze());
    assert_eq!(
        state.submit_disabled_reason(),
        Some(SubmitDisabledReason::GitHubAuthRequired)
    );
}

#[test]
fn ai_blocked_state_keeps_manual_draft_available_but_disables_analyze() {
    let state = AppViewState::sample_github_connected_ai_blocked();

    assert!(state.can_refresh_queue());
    assert!(state.can_edit_manual_draft());
    assert!(!state.can_analyze());
    assert_eq!(
        state.analyze_disabled_reason().as_deref(),
        Some("AI provider blocked")
    );
}

#[test]
fn queue_filtering_supports_review_requested_and_assigned() {
    let mut state = AppViewState::sample_github_connected_ai_blocked();
    state.queue.filter = QueueFilter::ReviewRequested;
    let requested = state.queue.filtered_items();

    assert_eq!(requested.len(), 1);
    assert!(requested[0].review_requested);

    state.queue.filter = QueueFilter::Assigned;
    let assigned = state.queue.filtered_items();

    assert_eq!(assigned.len(), 1);
    assert!(assigned[0].assigned);
}

#[test]
fn direct_pr_input_validation_reports_invalid_host_or_pattern() {
    assert!(matches!(
        AppViewState::validate_direct_pr_input("https://gitlab.com/company/app/pull/1"),
        DirectPrValidation::Invalid(_)
    ));
    assert!(matches!(
        AppViewState::validate_direct_pr_input("not a pr"),
        DirectPrValidation::Invalid(_)
    ));
    assert!(matches!(
        AppViewState::validate_direct_pr_input("company/payment-web#582"),
        DirectPrValidation::Valid(reference) if reference.to_string() == "company/payment-web#582"
    ));
}

#[test]
fn submit_disabled_reason_covers_empty_stale_missing_github_and_missing_pr() {
    let mut state = AppViewState::startup_auth_required();
    assert_eq!(
        state.submit_disabled_reason(),
        Some(SubmitDisabledReason::GitHubAuthRequired)
    );

    state = AppViewState::sample_github_connected_ai_blocked();
    state.queue.selected_pr = None;
    assert_eq!(
        state.submit_disabled_reason(),
        Some(SubmitDisabledReason::NoPullRequestSelected)
    );

    state.queue.selected_pr = Some("company/payment-web#582".to_string());
    state.draft.body.clear();
    assert_eq!(
        state.submit_disabled_reason(),
        Some(SubmitDisabledReason::DraftBodyEmpty)
    );

    state.draft.body = "Looks good after manual review.".to_string();
    state.draft.stale = true;
    assert_eq!(
        state.submit_disabled_reason(),
        Some(SubmitDisabledReason::DraftStale)
    );
}

#[test]
fn submit_disabled_reason_covers_sso_private_consent_and_explicit_verdict() {
    let mut state = AppViewState::sample_github_connected_ai_blocked();
    state.auth = reviewdesk::ui::state::AuthViewState::SsoRequired {
        message: "Authorize this organization with SSO.".to_string(),
    };
    state.draft.body = "Manual review body".to_string();
    assert_eq!(
        state.submit_disabled_reason(),
        Some(SubmitDisabledReason::SsoRequired)
    );

    state = AppViewState::sample_github_connected_ai_blocked();
    state.draft.body = "AI generated draft body".to_string();
    state.draft.generated_by_ai = true;
    state.run.private_diff_consent_required = true;
    state.run.private_diff_consent_accepted = false;
    assert_eq!(
        state.submit_disabled_reason(),
        Some(SubmitDisabledReason::PrivateDiffConsentRequired)
    );

    state.run.private_diff_consent_accepted = true;
    state.draft.event = ReviewEvent::Approve;
    state.draft.explicit_event_selected = false;
    assert_eq!(
        state.submit_disabled_reason(),
        Some(SubmitDisabledReason::ExplicitVerdictConfirmationRequired)
    );

    state.select_review_event(ReviewEvent::Approve);
    assert_eq!(state.submit_disabled_reason(), None);
}

#[test]
fn submit_dialog_tracks_open_cancel_and_confirm_state() {
    let mut state = AppViewState::sample_github_connected_ai_blocked();
    state.draft.body = "Manual review body".to_string();
    state.draft.event = ReviewEvent::Comment;

    state.open_submit_dialog();
    assert!(matches!(
        state.dialog,
        Some(DialogState::SubmitConfirm { .. })
    ));

    state.cancel_dialog();
    assert!(state.dialog.is_none());

    state.open_submit_dialog();
    state.confirm_submit_dialog();
    assert!(matches!(state.dialog, Some(DialogState::Submitting { .. })));
}

#[test]
fn submit_failed_state_preserves_draft_and_selected_context() {
    let mut state = AppViewState::sample_github_connected_ai_blocked();
    state.draft.body = "Keep this draft after failure".to_string();
    state.draft.event = ReviewEvent::Comment;
    state.draft.report_path = ".reviewdesk/reviews/company-payment-web-582.md".to_string();
    let selected_pr = state.queue.selected_pr.clone();

    state.apply_sample_submit_failed("GitHub rate limit, retry after reset");

    assert_eq!(state.draft.body, "Keep this draft after failure");
    assert_eq!(state.draft.event, ReviewEvent::Comment);
    assert_eq!(
        state.draft.report_path,
        ".reviewdesk/reviews/company-payment-web-582.md"
    );
    assert_eq!(state.queue.selected_pr, selected_pr);
    assert_eq!(
        state.draft.last_error.as_deref(),
        Some("GitHub rate limit, retry after reset")
    );
}

#[test]
fn snapshot_names_cover_prd_required_states() {
    let names = required_snapshot_names();

    for required in [
        "startup_auth_required",
        "github_connected_ai_blocked",
        "queue_empty",
        "queue_loaded_with_selection",
        "direct_pr_invalid",
        "context_collecting",
        "consent_required_private_repo",
        "draft_ready",
        "stale_submit_blocked",
        "submit_confirm_dialog",
        "submit_failed_draft_preserved",
        "submitted_review_id",
        "oauth_required_first_run",
        "queue_loaded_no_pr_selected",
        "pr_context_collecting",
        "ai_ready_consent_required",
        "ai_blocked_manual_draft",
        "draft_generated_needs_review",
        "submit_disabled_stale_head",
        "submit_confirm_approve_explicit",
        "submitted_review_linked",
        "modern_visual_baseline",
        "long_content_resilience",
    ] {
        assert!(names.contains(&required), "missing {required}");
    }
}

#[test]
fn viewport_width_selects_wide_or_narrow_layout() {
    assert_eq!(
        viewport_mode_for_width(ViewportMode::Wide, 1280.0),
        ViewportMode::Wide
    );
    assert_eq!(
        viewport_mode_for_width(ViewportMode::Wide, 1100.0),
        ViewportMode::Wide
    );
    assert_eq!(
        viewport_mode_for_width(ViewportMode::Wide, 800.0),
        ViewportMode::Narrow(WorkspaceSection::Queue)
    );
    assert_eq!(
        viewport_mode_for_width(ViewportMode::Narrow(WorkspaceSection::Draft), 800.0),
        ViewportMode::Narrow(WorkspaceSection::Draft)
    );
}
