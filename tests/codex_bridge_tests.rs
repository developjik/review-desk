use reviewdesk::app_core::{AiBlockedReason, AiConnectionStatus, ReasoningEffort};
use reviewdesk::codex_bridge::{
    CodexAuthMode, CodexJsonRpcMessageKind, CodexProcessEnvPolicy, CodexTransport,
    account_status_from_app_server_response, classify_json_rpc_message,
    extract_codex_exec_draft_from_jsonl, map_app_server_models, payload_contains_forbidden_fields,
    sanitize_codex_diagnostics, version_from_output,
};
use serde_json::json;

#[test]
fn parses_codex_cli_version_from_current_output_format() {
    let version = version_from_output("codex-cli 0.130.0\n").expect("version parsed");

    assert_eq!(version.binary_name, "codex-cli");
    assert_eq!(version.major, 0);
    assert_eq!(version.minor, 130);
    assert_eq!(version.patch, 0);
    assert!(version.supports_app_server());
}

#[test]
fn maps_chatgpt_account_as_connected_without_exposing_email() {
    let status = account_status_from_app_server_response(&json!({
        "account": {
            "type": "chatgpt",
            "email": "reviewer@example.com",
            "planType": "pro"
        },
        "requiresOpenaiAuth": true
    }));

    assert_eq!(status.status, AiConnectionStatus::Connected);
    assert_eq!(status.auth_mode.as_deref(), Some("chatgpt"));
    assert_eq!(status.plan_type.as_deref(), Some("pro"));
    assert_eq!(
        status
            .account
            .as_ref()
            .and_then(|account| account.plan_type.as_deref()),
        Some("pro")
    );
    let serialized = serde_json::to_string(&status).expect("serializes");
    assert!(!serialized.contains("reviewer@example.com"));
}

#[test]
fn blocks_api_key_and_chatgpt_auth_tokens_auth_modes() {
    let api_key = account_status_from_app_server_response(&json!({
        "account": { "type": "apiKey" },
        "requiresOpenaiAuth": false
    }));
    let token_mode = CodexAuthMode::from_raw("chatgptAuthTokens");

    assert_eq!(api_key.status, AiConnectionStatus::Unsupported);
    assert_eq!(
        api_key.blocked_reason,
        Some(AiBlockedReason::CodexAuthModeUnsupported)
    );
    assert_eq!(token_mode, CodexAuthMode::ChatGptAuthTokens);
    assert_eq!(
        token_mode.blocked_reason(),
        Some(AiBlockedReason::CodexAuthModeUnsupported)
    );
}

#[test]
fn maps_app_server_models_and_reasoning_efforts_without_hardcoded_fallbacks() {
    let models = map_app_server_models(&json!({
        "data": [
            {
                "id": "gpt-5.5",
                "displayName": "GPT-5.5",
                "hidden": false,
                "supportedReasoningEfforts": [
                    { "reasoningEffort": "minimal", "description": "Smallest reasoning" },
                    { "reasoningEffort": "low", "description": "Fast" },
                    { "reasoningEffort": "medium", "description": "Balanced" }
                ],
                "defaultReasoningEffort": "medium",
                "inputModalities": ["text", "image"],
                "isDefault": true,
                "upgrade": null,
                "upgradeInfo": null
            },
            {
                "id": "hidden-internal",
                "displayName": "Hidden",
                "hidden": true,
                "supportedReasoningEfforts": [],
                "defaultReasoningEffort": "low",
                "inputModalities": ["text"],
                "isDefault": false,
                "upgrade": null,
                "upgradeInfo": null
            },
            {
                "id": "old-model",
                "displayName": "Old Model",
                "hidden": false,
                "supportedReasoningEfforts": [
                    { "reasoningEffort": "low", "description": "Fast" }
                ],
                "defaultReasoningEffort": "low",
                "inputModalities": ["text"],
                "isDefault": false,
                "upgrade": "gpt-5.4",
                "upgradeInfo": { "model": "gpt-5.4" }
            }
        ]
    }))
    .expect("models map");

    assert_eq!(models.len(), 3);
    assert!(models[0].is_default);
    assert_eq!(
        models[0].supported_reasoning_efforts,
        vec![
            ReasoningEffort::Minimal,
            ReasoningEffort::Low,
            ReasoningEffort::Medium
        ]
    );
    assert!(models[1].hidden);
    assert!(!models[2].available);
    assert_eq!(
        models[2].unavailable_reason.as_deref(),
        Some("upgrade_required")
    );
}

#[test]
fn classifies_json_rpc_response_notification_and_server_request() {
    assert_eq!(
        classify_json_rpc_message(&json!({"id": 1, "result": {"ok": true}})),
        CodexJsonRpcMessageKind::Response
    );
    assert_eq!(
        classify_json_rpc_message(&json!({"method": "account/updated", "params": {}})),
        CodexJsonRpcMessageKind::Notification
    );
    assert_eq!(
        classify_json_rpc_message(&json!({
            "id": 7,
            "method": "account/chatgptAuthTokens/refresh",
            "params": {}
        })),
        CodexJsonRpcMessageKind::ServerRequest
    );
}

#[test]
fn extracts_codex_exec_draft_from_tolerant_jsonl_stream() {
    let jsonl = r#"
{"type":"thread.started","thread_id":"t1"}
{"method":"item/agentMessage/delta","params":{"delta":"첫 번째 문장"}}
{"method":"item/agentMessage/delta","params":{"delta":"과 두 번째 문장"}}
{"type":"turn.completed"}
"#;

    let draft = extract_codex_exec_draft_from_jsonl(jsonl).expect("draft extracted");

    assert_eq!(draft, "첫 번째 문장과 두 번째 문장");
}

#[test]
fn extracts_final_response_text_when_delta_stream_is_absent() {
    let jsonl = r#"
{"type":"item.completed","item":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"최종 리뷰 초안"}]}}
"#;

    let draft = extract_codex_exec_draft_from_jsonl(jsonl).expect("draft extracted");

    assert_eq!(draft, "최종 리뷰 초안");
}

#[test]
fn redacts_and_rejects_codex_token_boundary_violations() {
    let payload = json!({
        "accessToken": "abc",
        "chatgptAuthTokens": { "refreshToken": "def" },
        "path": "/Users/me/.codex/auth.json",
        "Authorization": "Bearer secret"
    });
    let text = payload.to_string();
    let sanitized = sanitize_codex_diagnostics(&text);

    assert!(payload_contains_forbidden_fields(&payload));
    assert!(!sanitized.contains("accessToken"));
    assert!(!sanitized.contains("refreshToken"));
    assert!(!sanitized.contains("Authorization"));
    assert!(!sanitized.contains("auth.json"));
}

#[test]
fn process_env_policy_excludes_token_like_variables() {
    let mut policy = CodexProcessEnvPolicy::default();
    policy.insert_input("PATH", "/usr/bin:/bin");
    policy.insert_input("HOME", "/Users/reviewer");
    policy.insert_input("GITHUB_TOKEN", "ghp_secret");
    policy.insert_input("OPENAI_API_KEY", "sk-secret");
    policy.insert_input("REVIEWDESK_CODEX_BIN", "/usr/local/bin/codex");

    let env = policy.filtered_env();

    assert_eq!(env.get("PATH").map(String::as_str), Some("/usr/bin:/bin"));
    assert_eq!(env.get("HOME").map(String::as_str), Some("/Users/reviewer"));
    assert!(!env.contains_key("GITHUB_TOKEN"));
    assert!(!env.contains_key("OPENAI_API_KEY"));
    assert!(!env.contains_key("REVIEWDESK_CODEX_BIN"));
}

#[test]
fn transport_prefers_app_server_stdio_before_exec_fallback() {
    assert_eq!(CodexTransport::default(), CodexTransport::AppServerStdio);
    assert_eq!(CodexTransport::ExecJson.as_str(), "exec_json");
}
