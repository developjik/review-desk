use reviewdesk::security::{PrivateDiffConsent, SecretMasker};
use reviewdesk::storage::LocalStore;

#[test]
fn masks_common_secret_patterns_before_ai_input() {
    let input = r#"
GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwx
JWT=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.signature
-----BEGIN PRIVATE KEY-----
secret
-----END PRIVATE KEY-----
SLACK_WEBHOOK=https://hooks.slack.com/services/T000/B000/abcdef
GENERIC_API_KEY=sk-test_1234567890abcdef
"#;

    let masked = SecretMasker::default().mask(input);

    assert!(
        !masked
            .text
            .contains("ghp_1234567890abcdefghijklmnopqrstuvwx")
    );
    assert!(!masked.text.contains("eyJhbGciOiJIUzI1NiJ9"));
    assert!(!masked.text.contains("BEGIN PRIVATE KEY"));
    assert!(!masked.text.contains("https://hooks.slack.com/services"));
    assert!(!masked.text.contains("sk-test_1234567890abcdef"));
    assert!(masked.findings.len() >= 5);
}

#[test]
fn masks_github_oauth_browser_flow_secrets() {
    let input = r#"
Authorization: Bearer gho_abcdefghijklmnopqrstuvwxyz123456
client_secret=super-secret-value
code=gh-code-like-secret-value-1234567890
GITHUB_REFRESH=ghr_abcdefghijklmnopqrstuvwxyz123456
"#;

    let masked = SecretMasker::default().mask(input);

    assert!(!masked.text.contains("gho_abcdefghijklmnopqrstuvwxyz123456"));
    assert!(!masked.text.contains("super-secret-value"));
    assert!(!masked.text.contains("gh-code-like-secret-value-1234567890"));
    assert!(!masked.text.contains("ghr_abcdefghijklmnopqrstuvwxyz123456"));
    assert!(
        masked
            .findings
            .iter()
            .any(|finding| finding.kind == "github_token")
    );
    assert!(
        masked
            .findings
            .iter()
            .any(|finding| finding.kind == "bearer_token")
    );
    assert!(
        masked
            .findings
            .iter()
            .any(|finding| finding.kind == "oauth_client_secret")
    );
    assert!(
        masked
            .findings
            .iter()
            .any(|finding| finding.kind == "oauth_code")
    );
}

#[test]
fn requires_private_diff_consent_before_external_ai_send() {
    let consent = PrivateDiffConsent {
        repository_private: true,
        provider: "codex-app-server".to_string(),
        model: "gpt-5.4".to_string(),
        transmitted_scope: "sanitized diff patches and PR metadata".to_string(),
        accepted: false,
    };

    assert!(consent.required());
    assert!(!consent.can_send_to_ai());
}

#[test]
fn creates_reviewdesk_dir_with_ignore_protection() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store = LocalStore::init(temp.path()).expect("store init");

    assert!(store.root().join(".gitignore").exists());
    assert_eq!(
        std::fs::read_to_string(store.root().join(".gitignore")).unwrap(),
        "*\n"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(store.root())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }
}

#[test]
fn stores_review_reports_and_drafts_under_reviewdesk_only() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store = LocalStore::init(temp.path()).expect("store init");

    let report_path = store
        .save_text("reviews/company-payment-web-582.md", "# Report\n")
        .expect("report save");
    let draft_path = store
        .save_json(
            "drafts/company-payment-web-582.json",
            &serde_json::json!({"body": "draft"}),
        )
        .expect("draft save");

    assert!(report_path.starts_with(store.root()));
    assert!(draft_path.starts_with(store.root()));
    assert_eq!(
        store
            .load_text("reviews/company-payment-web-582.md")
            .expect("report load"),
        "# Report\n"
    );
    assert!(store.save_text("../leak.md", "bad").is_err());
}
