use reviewdesk::app_core::{
    AiBlockedReason, AiConnectionStatus, AiConnectionStatusView, AiModelView, AiRateLimitSnapshot,
    AppStatusContext, AppStatusInput, AuthConnectionState, CommandRisk, LanguagePreferences,
    Locale, ReasoningEffort, SubmitPreflightInput, SubmitPreflightStatus, allowed_ipc_commands,
    frontend_safe_app_status, frontend_safe_app_status_from_context,
    frontend_safe_app_status_from_input, prepare_submit_review, resolve_review_locale,
    validate_ai_run_configuration, validate_external_url,
};
use reviewdesk::domain::ReviewEvent;

#[test]
fn app_status_reports_oauth_gates_without_serializing_tokens() {
    let status = frontend_safe_app_status(false, false);
    let json = serde_json::to_string(&status).expect("status serializes");

    assert_eq!(status.github, AuthConnectionState::Missing);
    assert_eq!(status.chatgpt, AuthConnectionState::Missing);
    assert!(status.startup_gate_required);
    assert!(json.contains("github"));
    assert!(json.contains("chatgpt"));
    assert!(!json.to_ascii_lowercase().contains("token"));
    assert!(!json.to_ascii_lowercase().contains("secret"));
    assert!(!json.to_ascii_lowercase().contains("authorization"));
}

#[test]
fn app_status_keeps_review_queue_available_when_only_chatgpt_is_missing() {
    let status = frontend_safe_app_status(true, false);

    assert_eq!(status.github, AuthConnectionState::Connected);
    assert_eq!(status.chatgpt, AuthConnectionState::Missing);
    assert!(!status.startup_gate_required);
    assert!(status.review_queue_available);
    assert!(!status.ai_review_available);
    assert_eq!(
        status.ai_blocked_reason.as_deref(),
        Some("codex_chatgpt_auth_required")
    );
}

#[test]
fn strict_auth_mode_blocks_app_shell_until_chatgpt_is_connected() {
    let status = frontend_safe_app_status_from_input(AppStatusInput {
        github: AuthConnectionState::Connected,
        chatgpt: AuthConnectionState::Missing,
        strict_auth_mode: true,
        language_preferences: LanguagePreferences::for_os_locale("en-US"),
    });

    assert!(status.startup_gate_required);
    assert!(!status.capabilities.app_shell_available);
    assert!(!status.capabilities.review_queue_available);
    assert!(!status.capabilities.ai_review_available);
    assert!(
        status
            .capabilities
            .blocked_reasons
            .contains(&"codex_chatgpt_auth_required".to_string())
    );
}

#[test]
fn github_only_default_mode_keeps_manual_review_available() {
    let status = frontend_safe_app_status_from_input(AppStatusInput {
        github: AuthConnectionState::Connected,
        chatgpt: AuthConnectionState::Missing,
        strict_auth_mode: false,
        language_preferences: LanguagePreferences::for_os_locale("ko-KR"),
    });

    assert!(!status.startup_gate_required);
    assert!(status.capabilities.app_shell_available);
    assert!(status.capabilities.review_queue_available);
    assert!(status.capabilities.pr_context_available);
    assert!(status.capabilities.manual_draft_available);
    assert!(status.capabilities.submit_available);
    assert!(!status.capabilities.ai_review_available);
    assert_eq!(status.language_preferences.ui_locale, Locale::Ko);
    assert_eq!(status.language_preferences.review_locale, Locale::Ko);
}

#[test]
fn codex_missing_blocks_only_agent_run_in_default_mode() {
    let status = frontend_safe_app_status_from_context(AppStatusContext {
        github: AuthConnectionState::Connected,
        ai_connection: AiConnectionStatusView::missing(),
        strict_auth_mode: false,
        language_preferences: LanguagePreferences::for_os_locale("en-US"),
        ai_models: Vec::new(),
        selected_model: "gpt-5.5".to_string(),
        selected_reasoning_effort: ReasoningEffort::Medium,
        rate_limit: AiRateLimitSnapshot::unknown(),
        generation_adapter_available: false,
    });

    assert_eq!(status.chatgpt, AuthConnectionState::Missing);
    assert_eq!(status.ai_connection.status, AiConnectionStatus::Missing);
    assert!(!status.startup_gate_required);
    assert!(status.capabilities.app_shell_available);
    assert!(status.capabilities.review_queue_available);
    assert!(status.capabilities.pr_context_available);
    assert!(status.capabilities.manual_draft_available);
    assert!(status.capabilities.submit_available);
    assert!(!status.capabilities.ai_review_available);
    assert_eq!(
        status.ai_connection.blocked_reason,
        Some(AiBlockedReason::CodexChatgptAuthRequired)
    );
    assert_eq!(
        status.ai_blocked_reason.as_deref(),
        Some("codex_chatgpt_auth_required")
    );
}

#[test]
fn strict_auth_mode_blocks_app_shell_until_codex_chatgpt_is_connected() {
    let status = frontend_safe_app_status_from_context(AppStatusContext {
        github: AuthConnectionState::Connected,
        ai_connection: AiConnectionStatusView::missing(),
        strict_auth_mode: true,
        language_preferences: LanguagePreferences::for_os_locale("en-US"),
        ai_models: Vec::new(),
        selected_model: "gpt-5.5".to_string(),
        selected_reasoning_effort: ReasoningEffort::Medium,
        rate_limit: AiRateLimitSnapshot::unknown(),
        generation_adapter_available: false,
    });

    assert!(status.startup_gate_required);
    assert!(!status.capabilities.app_shell_available);
    assert!(
        status
            .capabilities
            .blocked_reasons
            .contains(&"codex_chatgpt_auth_required".to_string())
    );
}

#[test]
fn model_capability_blocks_unavailable_models_and_unsupported_reasoning_effort() {
    let models = vec![
        AiModelView::available(
            "gpt-5.5",
            "GPT-5.5",
            vec![
                ReasoningEffort::Minimal,
                ReasoningEffort::Low,
                ReasoningEffort::Medium,
                ReasoningEffort::High,
            ],
            ReasoningEffort::Medium,
        ),
        AiModelView::unavailable(
            "gpt-5.3-codex-spark",
            "GPT-5.3-Codex-Spark",
            "plan_required",
        ),
    ];

    assert_eq!(
        validate_ai_run_configuration(&models, "gpt-5.3-codex-spark", ReasoningEffort::Medium),
        Err(AiBlockedReason::ModelUnavailable)
    );
    assert_eq!(
        validate_ai_run_configuration(&models, "gpt-5.5", ReasoningEffort::Xhigh),
        Err(AiBlockedReason::ReasoningEffortUnavailable)
    );
    assert_eq!(
        validate_ai_run_configuration(&models, "gpt-5.5", ReasoningEffort::Minimal),
        Ok(())
    );
    assert_eq!(
        validate_ai_run_configuration(&models, "gpt-5.5", ReasoningEffort::High),
        Ok(())
    );
}

#[test]
fn language_preferences_default_from_os_locale_and_review_override_priority() {
    let korean = LanguagePreferences::for_os_locale("ko-KR");
    assert_eq!(korean.ui_locale, Locale::Ko);
    assert_eq!(korean.review_locale, Locale::Ko);

    let english = LanguagePreferences::for_os_locale("fr-FR");
    assert_eq!(english.ui_locale, Locale::En);
    assert_eq!(english.review_locale, Locale::En);

    let mut preferences = LanguagePreferences {
        ui_locale: Locale::Ko,
        review_locale: Locale::En,
        repo_review_locale_overrides: std::collections::BTreeMap::new(),
    };
    preferences
        .repo_review_locale_overrides
        .insert("company/payment-web".to_string(), Locale::Ko);

    assert_eq!(
        resolve_review_locale(&preferences, Some("company/payment-web"), Some(Locale::En)),
        Locale::Ko
    );
    assert_eq!(
        resolve_review_locale(&preferences, Some("company/admin"), Some(Locale::Ko)),
        Locale::Ko
    );
    assert_eq!(
        resolve_review_locale(&preferences, Some("company/admin"), None),
        Locale::En
    );
}

#[test]
fn ipc_allowlist_contains_prd_commands_and_marks_submit_as_high_risk() {
    let commands = allowed_ipc_commands();
    let names = commands
        .iter()
        .map(|command| command.name)
        .collect::<Vec<_>>();

    for required in [
        "get_app_status",
        "start_github_oauth",
        "poll_github_oauth",
        "cancel_github_oauth",
        "logout_github",
        "refresh_github_auth_status",
        "start_chatgpt_oauth",
        "poll_chatgpt_oauth",
        "list_repositories",
        "load_review_queue",
        "collect_pr_context",
        "set_private_diff_consent",
        "generate_review_draft",
        "create_draft_from_run",
        "save_review_draft",
        "read_review_draft",
        "list_review_drafts",
        "mark_active_draft",
        "save_draft",
        "prepare_submit_review",
        "confirm_submit_review",
        "open_external_url",
        "get_ai_connection_status",
        "get_codex_bridge_status",
        "start_codex_chatgpt_login",
        "poll_codex_chatgpt_login",
        "cancel_codex_chatgpt_login",
        "read_codex_account",
        "list_ai_models",
        "select_ai_model",
        "read_codex_rate_limits",
        "logout_codex_chatgpt",
        "refresh_ai_account_status",
        "start_agent_run",
        "read_agent_run",
        "cancel_agent_run",
    ] {
        assert!(names.contains(&required), "missing {required}");
    }

    let submit = commands
        .iter()
        .find(|command| command.name == "confirm_submit_review")
        .expect("confirm submit command exists");
    assert_eq!(submit.risk, CommandRisk::WritesRemote);
    let github_logout = commands
        .iter()
        .find(|command| command.name == "logout_github")
        .expect("github logout command exists");
    assert_eq!(github_logout.risk, CommandRisk::WritesLocal);
    for command_name in [
        "create_draft_from_run",
        "save_review_draft",
        "mark_active_draft",
    ] {
        let command = commands
            .iter()
            .find(|command| command.name == command_name)
            .expect("draft write command exists");
        assert_eq!(command.risk, CommandRisk::WritesLocal);
    }
    for command_name in ["read_review_draft", "list_review_drafts"] {
        let command = commands
            .iter()
            .find(|command| command.name == command_name)
            .expect("draft read command exists");
        assert_eq!(command.risk, CommandRisk::ReadOnly);
    }
    assert!(!names.iter().any(|name| name.contains("token")));
}

#[test]
fn submit_preflight_blocks_stale_empty_and_unconfirmed_verdicts() {
    let stale = prepare_submit_review(SubmitPreflightInput {
        github_connected: true,
        write_scope_valid: true,
        sso_required: false,
        pr_open: true,
        pr_merged: false,
        expected_head_sha: "old".to_string(),
        current_head_sha: "new".to_string(),
        draft_body: "Please check the retry loop.".to_string(),
        event: ReviewEvent::Comment,
        explicit_verdict_confirmed: false,
        private_diff_consent_required: false,
        private_diff_consent_accepted: false,
    });
    assert_eq!(stale.status, SubmitPreflightStatus::Blocked);
    assert!(stale.blocked_reasons.contains(&"head_changed".to_string()));
    assert!(stale.confirmation_id.is_none());

    let approve_without_confirmation = prepare_submit_review(SubmitPreflightInput {
        expected_head_sha: "same".to_string(),
        current_head_sha: "same".to_string(),
        draft_body: "Looks good.".to_string(),
        event: ReviewEvent::Approve,
        explicit_verdict_confirmed: false,
        github_connected: true,
        write_scope_valid: true,
        sso_required: false,
        pr_open: true,
        pr_merged: false,
        private_diff_consent_required: false,
        private_diff_consent_accepted: false,
    });
    assert_eq!(
        approve_without_confirmation.status,
        SubmitPreflightStatus::Blocked
    );
    assert!(
        approve_without_confirmation
            .blocked_reasons
            .contains(&"explicit_verdict_confirmation_required".to_string())
    );

    let ready = prepare_submit_review(SubmitPreflightInput {
        expected_head_sha: "same".to_string(),
        current_head_sha: "same".to_string(),
        draft_body: "Looks good after manual review.".to_string(),
        event: ReviewEvent::Comment,
        explicit_verdict_confirmed: false,
        github_connected: true,
        write_scope_valid: true,
        sso_required: false,
        pr_open: true,
        pr_merged: false,
        private_diff_consent_required: false,
        private_diff_consent_accepted: false,
    });
    assert_eq!(ready.status, SubmitPreflightStatus::Ready);
    assert!(ready.blocked_reasons.is_empty());
    assert!(ready.confirmation_id.is_some());
}

#[test]
fn submit_preflight_blocks_missing_private_diff_consent() {
    let blocked = prepare_submit_review(SubmitPreflightInput {
        github_connected: true,
        write_scope_valid: true,
        sso_required: false,
        pr_open: true,
        pr_merged: false,
        expected_head_sha: "same".to_string(),
        current_head_sha: "same".to_string(),
        draft_body: "Generated from AI draft.".to_string(),
        event: ReviewEvent::Comment,
        explicit_verdict_confirmed: false,
        private_diff_consent_required: true,
        private_diff_consent_accepted: false,
    });

    assert_eq!(blocked.status, SubmitPreflightStatus::Blocked);
    assert!(
        blocked
            .blocked_reasons
            .contains(&"private_diff_consent_required".to_string())
    );
}

#[test]
fn submit_preflight_uses_release_reason_codes() {
    let blocked = prepare_submit_review(SubmitPreflightInput {
        github_connected: false,
        write_scope_valid: false,
        sso_required: true,
        pr_open: false,
        pr_merged: true,
        expected_head_sha: "old".to_string(),
        current_head_sha: "new".to_string(),
        draft_body: "   ".to_string(),
        event: ReviewEvent::RequestChanges,
        explicit_verdict_confirmed: false,
        private_diff_consent_required: true,
        private_diff_consent_accepted: false,
    });

    assert_eq!(blocked.status, SubmitPreflightStatus::Blocked);
    for reason in [
        "github_auth_required",
        "github_scope_insufficient",
        "github_sso_required",
        "pr_closed",
        "pr_merged",
        "head_changed",
        "body_empty",
        "explicit_verdict_confirmation_required",
        "private_diff_consent_required",
    ] {
        assert!(
            blocked.blocked_reasons.contains(&reason.to_string()),
            "missing reason {reason}"
        );
    }
}

#[test]
fn external_url_validation_requires_https_and_allowlisted_hosts() {
    assert!(validate_external_url("https://github.com/company/repo/pull/1").is_ok());
    assert!(validate_external_url("https://chatgpt.com/").is_ok());
    assert!(validate_external_url("https://auth.openai.com/codex/device").is_ok());
    assert!(validate_external_url("http://github.com/company/repo").is_err());
    assert!(validate_external_url("https://example.com/company/repo").is_err());
    assert!(validate_external_url("not a url").is_err());
}
