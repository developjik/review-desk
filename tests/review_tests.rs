use reviewdesk::ai::{AiProvider, BlockedAiProvider, MockAiProvider};
use reviewdesk::domain::{AiProviderStatus, ChangedFile, ReviewRunStatus};
use reviewdesk::review::{
    ReviewPipeline, build_review_input, filter_changed_files, new_analysis_run_id,
    stable_changed_files_hash,
};

#[tokio::test]
async fn blocked_ai_provider_returns_blocked_status() {
    let provider = BlockedAiProvider::new("official ChatGPT OAuth provider unavailable");

    assert_eq!(
        provider.auth_status().await.unwrap(),
        AiProviderStatus::BlockedUnsupportedAuth
    );
}

#[test]
fn filters_generated_and_lock_files_from_review_input() {
    let files = vec![
        ChangedFile::new("src/lib.rs", Some("@@ patch")),
        ChangedFile::new("pnpm-lock.yaml", Some("@@ lock")),
        ChangedFile::new("generated/client.ts", Some("@@ generated")),
    ];

    let filtered = filter_changed_files(&files, &["pnpm-lock.yaml", "generated/**"]).unwrap();

    assert_eq!(filtered.included.len(), 1);
    assert_eq!(filtered.excluded.len(), 2);
    assert_eq!(filtered.included[0].path, "src/lib.rs");
}

#[test]
fn analysis_run_ids_are_unique_for_same_pr_and_diff() {
    let first = new_analysis_run_id("company", "payment-web", 582);
    let second = new_analysis_run_id("company", "payment-web", 582);

    assert_ne!(first, second);
    assert!(first.starts_with("run-company-payment-web-582-"));
    assert!(second.starts_with("run-company-payment-web-582-"));
}

#[test]
fn changed_files_hash_is_stable_for_file_order() {
    let left = vec![
        ChangedFile::new("b.rs", Some("@@ b")),
        ChangedFile::new("a.rs", Some("@@ a")),
    ];
    let right = vec![
        ChangedFile::new("a.rs", Some("@@ a")),
        ChangedFile::new("b.rs", Some("@@ b")),
    ];

    assert_eq!(
        stable_changed_files_hash(&left),
        stable_changed_files_hash(&right)
    );
}

#[tokio::test]
async fn review_pipeline_keeps_manual_draft_path_when_ai_is_blocked() {
    let provider = BlockedAiProvider::new("unsupported auth");
    let input = build_review_input("company/payment-web#582", vec![], vec![]);
    let run = ReviewPipeline::default()
        .generate_or_block(&provider, input)
        .await
        .expect("pipeline result");

    assert_eq!(run.status, ReviewRunStatus::AiBlockedUnsupportedAuth);
    assert!(run.draft.body.is_empty());
    assert_eq!(run.run.error.as_deref(), Some("unsupported auth"));
}

#[tokio::test]
async fn review_pipeline_generates_markdown_report_with_mock_provider() {
    let provider = MockAiProvider::default();
    let input = build_review_input(
        "company/payment-web#582",
        vec![ChangedFile::new("src/lib.rs", Some("@@ patch"))],
        vec!["CI success".to_string()],
    );

    let output = ReviewPipeline::default()
        .generate_or_block(&provider, input)
        .await
        .expect("pipeline result");

    assert_eq!(output.status, ReviewRunStatus::DraftReady);
    assert!(output.markdown.contains("ReviewDesk Report"));
    assert!(output.markdown.contains("src/lib.rs"));
    assert!(output.draft.body.contains("Mock review"));
}
