use reviewdesk::app_core::{Locale, ReasoningEffort};
use reviewdesk::domain::{
    AnalysisRun, AnalysisRunMode, AnalysisRunStatus, ReviewDraft, ReviewEvent,
};
use reviewdesk::workspace_store::{PrWorkspaceKey, WorkspaceStore};

#[test]
fn workspace_store_groups_artifacts_by_pr_with_owner_only_permissions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
    let run = sample_run("run-a");
    let draft = sample_draft("draft-a", &run.run_id);

    let run_path = store.save_run(&key, &run).expect("save run");
    let draft_path = store.save_draft(&key, &draft).expect("save draft");
    store
        .mark_active_draft(&key, &draft.draft_id)
        .expect("active draft");

    assert!(run_path.ends_with(".reviewdesk/workspaces/company/payment-web/582/runs/run-a.json"));
    assert!(
        draft_path.ends_with(".reviewdesk/workspaces/company/payment-web/582/drafts/draft-a.json")
    );
    assert_eq!(store.list_runs(&key).expect("runs").len(), 1);
    assert_eq!(
        store.read_active_draft_id(&key).expect("active"),
        Some("draft-a".to_string())
    );
}

#[test]
fn workspace_store_rejects_path_traversal_in_owner_repo_and_ids() {
    assert!(PrWorkspaceKey::new("../company", "payment-web", 582).is_err());
    assert!(PrWorkspaceKey::new("company", "../payment-web", 582).is_err());
}

fn sample_run(run_id: &str) -> AnalysisRun {
    AnalysisRun {
        run_id: run_id.to_string(),
        owner: "company".to_string(),
        repo: "payment-web".to_string(),
        number: 582,
        head_sha: "head".to_string(),
        diff_hash: "diff".to_string(),
        context_hash: "context".to_string(),
        mode: AnalysisRunMode::Fast,
        model_id: "gpt-5.5".to_string(),
        reasoning_effort: ReasoningEffort::Medium,
        review_language: Locale::En,
        custom_prompt: None,
        prompt_version: "reviewdesk-run-centric-v1".to_string(),
        selected_files: vec!["src/lib.rs".to_string()],
        excluded_files: vec![],
        private_diff_consent_snapshot: true,
        status: AnalysisRunStatus::DraftReady,
        blocked_reason: None,
        draft_seed_body: Some("Looks good.".to_string()),
        findings: vec![],
        created_at: "2026-05-17T00:00:00Z".to_string(),
        completed_at: Some("2026-05-17T00:00:01Z".to_string()),
    }
}

fn sample_draft(draft_id: &str, run_id: &str) -> ReviewDraft {
    ReviewDraft {
        draft_id: draft_id.to_string(),
        owner: "company".to_string(),
        repo: "payment-web".to_string(),
        number: 582,
        source_run_ids: vec![run_id.to_string()],
        base_head_sha: "head".to_string(),
        base_diff_hash: "diff".to_string(),
        verdict: ReviewEvent::Comment,
        body: "Looks good.".to_string(),
        inline_comments: vec![],
        user_edited: true,
        stale: false,
        created_at: "2026-05-17T00:00:00Z".to_string(),
        updated_at: "2026-05-17T00:00:02Z".to_string(),
    }
}
