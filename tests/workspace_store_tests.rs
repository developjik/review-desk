use reviewdesk::app_core::{Locale, ReasoningEffort};
use reviewdesk::domain::{
    AnalysisRun, AnalysisRunMode, AnalysisRunStatus, ReviewDraft, ReviewEvent,
};
use reviewdesk::workspace_store::{PrWorkspaceKey, WorkspaceStore};
use std::path::{Path, PathBuf};

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

    assert_owner_only_dir(temp.path().join(".reviewdesk/workspaces"));
    assert_owner_only_dir(temp.path().join(".reviewdesk/workspaces/company"));
    assert_owner_only_dir(
        temp.path()
            .join(".reviewdesk/workspaces/company/payment-web"),
    );
    assert_owner_only_dir(
        temp.path()
            .join(".reviewdesk/workspaces/company/payment-web/582"),
    );
    assert_owner_only_dir(
        temp.path()
            .join(".reviewdesk/workspaces/company/payment-web/582/snapshots"),
    );
    assert_owner_only_dir(
        temp.path()
            .join(".reviewdesk/workspaces/company/payment-web/582/runs"),
    );
    assert_owner_only_dir(
        temp.path()
            .join(".reviewdesk/workspaces/company/payment-web/582/drafts"),
    );
    assert_owner_only_dir(
        temp.path()
            .join(".reviewdesk/workspaces/company/payment-web/582/publish_attempts"),
    );
    assert_owner_only_file(&run_path);
    assert_owner_only_file(&draft_path);
    assert_owner_only_file(
        temp.path()
            .join(".reviewdesk/workspaces/company/payment-web/582/drafts/active.json"),
    );
}

#[test]
fn workspace_store_rejects_path_traversal_in_owner_repo_and_ids() {
    assert!(PrWorkspaceKey::new("../company", "payment-web", 582).is_err());
    assert!(PrWorkspaceKey::new("company", "../payment-web", 582).is_err());

    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");

    for invalid_id in [
        "", "..", "../run-a", "run/a", "run\\a", ".run-a", "run a", "run:a", "run#a", "run\ta",
    ] {
        let run = sample_run(invalid_id);
        assert!(
            store.save_run(&key, &run).is_err(),
            "run_id should be rejected: {invalid_id:?}"
        );

        let draft = sample_draft(invalid_id, "run-a");
        assert!(
            store.save_draft(&key, &draft).is_err(),
            "draft_id should be rejected on save: {invalid_id:?}"
        );
        assert!(
            store.read_draft(&key, invalid_id).is_err(),
            "draft_id should be rejected on read: {invalid_id:?}"
        );
        assert!(
            store.mark_active_draft(&key, invalid_id).is_err(),
            "active draft_id should be rejected: {invalid_id:?}"
        );
    }
}

#[test]
fn workspace_store_rejects_tampered_active_draft_id_on_read() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
    store
        .save_draft(&key, &sample_draft("draft-a", "run-a"))
        .expect("save draft");
    store
        .mark_active_draft(&key, "draft-a")
        .expect("active draft");

    std::fs::write(
        temp.path()
            .join(".reviewdesk/workspaces/company/payment-web/582/drafts/active.json"),
        "\"../draft\"",
    )
    .expect("tamper active draft");

    assert!(store.read_active_draft_id(&key).is_err());
}

#[test]
fn workspace_store_rejects_missing_active_draft_without_creating_dangling_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
    let active_path = active_draft_path(temp.path());

    assert!(store.mark_active_draft(&key, "missing-draft").is_err());
    assert!(!active_path.exists());
}

#[test]
fn workspace_store_rejects_artifacts_that_do_not_match_workspace_key() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");

    let mut run = sample_run("run-a");
    run.owner = "other-company".to_string();
    assert!(store.save_run(&key, &run).is_err());

    let mut run = sample_run("run-a");
    run.repo = "other-repo".to_string();
    assert!(store.save_run(&key, &run).is_err());

    let mut run = sample_run("run-a");
    run.number = 583;
    assert!(store.save_run(&key, &run).is_err());

    let mut draft = sample_draft("draft-a", "run-a");
    draft.owner = "other-company".to_string();
    assert!(store.save_draft(&key, &draft).is_err());

    let mut draft = sample_draft("draft-a", "run-a");
    draft.repo = "other-repo".to_string();
    assert!(store.save_draft(&key, &draft).is_err());

    let mut draft = sample_draft("draft-a", "run-a");
    draft.number = 583;
    assert!(store.save_draft(&key, &draft).is_err());
}

#[test]
fn workspace_store_rejects_duplicate_run_id_without_overwriting() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
    let original = sample_run("run-a");
    let mut duplicate = sample_run("run-a");
    duplicate.model_id = "different-model".to_string();

    store.save_run(&key, &original).expect("save original run");

    assert!(store.save_run(&key, &duplicate).is_err());
    let runs = store.list_runs(&key).expect("runs");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].model_id, "gpt-5.5");
}

#[cfg(unix)]
#[test]
fn workspace_store_rejects_existing_run_path_atomically_without_following_symlink() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
    store
        .save_run(&key, &sample_run("seed-run"))
        .expect("seed workspace dirs");

    let outside_path = temp.path().join("outside-run.json");
    let run_path = temp
        .path()
        .join(".reviewdesk/workspaces/company/payment-web/582/runs/run-a.json");
    symlink(&outside_path, &run_path).expect("symlink run path");

    assert!(store.save_run(&key, &sample_run("run-a")).is_err());
    assert!(!outside_path.exists());
    assert!(
        std::fs::symlink_metadata(&run_path)
            .expect("run path metadata")
            .file_type()
            .is_symlink()
    );
}

#[test]
fn workspace_store_lists_runs_by_created_at() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
    let mut newer = sample_run("run-a");
    newer.created_at = "2026-05-17T00:00:02Z".to_string();
    let mut older = sample_run("run-z");
    older.created_at = "2026-05-17T00:00:01Z".to_string();

    store.save_run(&key, &newer).expect("save newer run");
    store.save_run(&key, &older).expect("save older run");

    let runs = store.list_runs(&key).expect("runs");
    assert_eq!(
        runs.iter()
            .map(|run| run.run_id.as_str())
            .collect::<Vec<_>>(),
        vec!["run-z", "run-a"]
    );
}

#[test]
fn workspace_store_rejects_tampered_loaded_runs_that_do_not_match_workspace_key() {
    for (field, value) in [
        ("owner", serde_json::json!("other-company")),
        ("repo", serde_json::json!("other-repo")),
        ("number", serde_json::json!(583)),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = WorkspaceStore::init(temp.path()).expect("store");
        let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
        let run = sample_run("run-a");
        let run_path = store.save_run(&key, &run).expect("save run");
        tamper_json_field(&run_path, field, value);

        assert!(
            store.list_runs(&key).is_err(),
            "tampered run {field} should be rejected"
        );
    }
}

#[test]
fn workspace_store_rejects_tampered_loaded_drafts_that_do_not_match_workspace_key() {
    for (field, value) in [
        ("owner", serde_json::json!("other-company")),
        ("repo", serde_json::json!("other-repo")),
        ("number", serde_json::json!(583)),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = WorkspaceStore::init(temp.path()).expect("store");
        let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
        let draft = sample_draft("draft-a", "run-a");
        let draft_path = store.save_draft(&key, &draft).expect("save draft");
        tamper_json_field(&draft_path, field, value);

        assert!(
            store.read_draft(&key, "draft-a").is_err(),
            "tampered draft {field} should be rejected"
        );
    }
}

fn active_draft_path(temp_path: impl AsRef<Path>) -> PathBuf {
    temp_path
        .as_ref()
        .join(".reviewdesk/workspaces/company/payment-web/582/drafts/active.json")
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

fn tamper_json_field(path: impl AsRef<Path>, field: &str, value: serde_json::Value) {
    let data = std::fs::read(path.as_ref()).expect("read json");
    let mut json: serde_json::Value = serde_json::from_slice(&data).expect("parse json");
    json[field] = value;
    std::fs::write(
        path.as_ref(),
        serde_json::to_vec_pretty(&json).expect("serialize json"),
    )
    .expect("write tampered json");
}

#[cfg(unix)]
fn assert_owner_only_dir(path: impl AsRef<Path>) {
    use std::os::unix::fs::PermissionsExt;

    let mode = std::fs::metadata(path.as_ref())
        .expect("dir metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o700, "dir mode for {}", path.as_ref().display());
}

#[cfg(not(unix))]
fn assert_owner_only_dir(_path: impl AsRef<Path>) {}

#[cfg(unix)]
fn assert_owner_only_file(path: impl AsRef<Path>) {
    use std::os::unix::fs::PermissionsExt;

    let mode = std::fs::metadata(path.as_ref())
        .expect("file metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "file mode for {}", path.as_ref().display());
}

#[cfg(not(unix))]
fn assert_owner_only_file(_path: impl AsRef<Path>) {}
