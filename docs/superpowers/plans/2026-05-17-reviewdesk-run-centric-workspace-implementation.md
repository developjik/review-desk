# ReviewDesk Run-Centric PR Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first run-centric ReviewDesk slice: durable repeated analysis runs, durable per-PR review drafts, inline comment validation, exact publish preview, and GitHub review submission with selected inline comments.

**Architecture:** Keep the existing GitHub, Codex, and Tauri shell boundaries. Add focused Rust modules for workspace persistence, inline comment validation, and publish payload/preflight logic, then split the React UI into a small shell plus focused workspace components. The first slice proves the run-centric data model before adding post-v1 automation.

**Tech Stack:** Rust 2024, Tauri 2, React 19, TypeScript, Vite, Vitest, Testing Library, httpmock, serde JSON local storage under `.reviewdesk/`.

---

## Pre-Development Consensus

Four review agents examined the approved design and local codebase. Their shared verdict is:

- Development is **go** only after the implementation plan locks the run-centric first slice.
- Development is **no-go** for a broad PRD 0.1.0 UI rewrite or a direct `App.tsx` rewrite.
- Backend/API work must define durable runs, durable drafts, inline comment validation, exact publish payloads, and publish attempt persistence before frontend polish.
- Frontend work must split state and presentation so the approved design does not collapse into one global draft string and one current agent status.
- Tests for run uniqueness, draft preservation, inline mapping, stale preflight, and exact publish payload must lead the implementation.

## V1 Scope Locked

V1 includes:

- Assigned PR queue remains available.
- Selected PR workspace remains the primary work surface.
- Manual analysis runs with `fast`, `deep`, `security`, `tests`, and `custom` modes.
- Multiple durable analysis runs per PR, including repeated runs against the same head SHA and diff hash.
- Per-PR durable review drafts.
- Drafts store source run ids, base head SHA, base diff hash, verdict, body, inline comment drafts, edit state, and stale state.
- Inline comment drafts support edit, select, deselect, dismiss, and validation.
- Body-empty publish is allowed only when at least one selected inline comment is valid.
- Publish preflight blocks head SHA mismatch, diff hash mismatch, closed or merged PRs, missing auth/scope/SSO, empty payload, and invalid inline mappings.
- Publish preview and confirm submit use the same payload structure.
- Publish attempts are stored on success and failure.

## Post-V1 Scope

Post-v1 includes:

- Automatic background triage on app open.
- Draft inbox as a secondary surface.
- Advanced run comparison UI.
- Sophisticated merge UI for combining multiple run outputs.
- Suggested changes and multi-line GitHub suggestion blocks.
- Branch code editing, auto-fix, auto-merge, and auto-approve.
- Model benchmarking, run quality scoring, and analytics.

## Locked Product Decisions

- `AnalysisRun` is append-first. A run is never overwritten by another run.
- `ReviewDraft` is mutable user work. Late AI output never overwrites a user-edited draft.
- When a run completes while the active draft is dirty, the UI offers `Create draft from run` without replacing the active draft.
- PR switching auto-saves local draft state through `save_review_draft`; there is also an explicit Save action.
- Queue sections are `needs_review`, `draft_ready`, `running`, `blocked_stale`, and `published`.
- Run quick actions are visible buttons for `Fast`, `Deep`, `Security`, and `Tests`; `Custom` opens a drawer with prompt, model, reasoning, language, and file scope.
- V1 inline comments support current-diff new-side single-line comments plus optional same-side ranges. Old-side comments and GitHub suggestion blocks are post-v1.
- Publish confirmation is a modal launched from the Safety tab.
- Narrow desktop layouts must keep Draft & Publish reachable; the right panel can stack below the diff but must not disappear.

## Locked Storage Contract

All new artifacts live under:

```text
.reviewdesk/workspaces/{owner_slug}/{repo_slug}/{pr_number}/
  snapshots/
  runs/{run_id}.json
  drafts/{draft_id}.json
  drafts/active.json
  publish_attempts/{attempt_id}.json
```

Rules:

- `owner_slug`, `repo_slug`, `run_id`, `draft_id`, and `attempt_id` are path-safe slugs.
- `LocalStore::save_json` and `LocalStore::load_json` remain the only low-level JSON IO primitives.
- New workspace helpers reject absolute paths and `..`.
- Runs and publish attempts are append-first.
- Drafts are mutable and keep `updated_at`, `source_run_ids`, `base_head_sha`, and `base_diff_hash`.
- Legacy `.reviewdesk/runs/*.json`, `.reviewdesk/reviews/*.md`, and `.reviewdesk/drafts/*.md` remain readable only through existing paths; v1 does not migrate them into workspaces.

## Locked API Contract

New command names:

- `list_analysis_runs`
- `start_analysis_run`
- `read_analysis_run`
- `cancel_analysis_run`
- `archive_analysis_run`
- `create_draft_from_run`
- `save_review_draft`
- `read_review_draft`
- `list_review_drafts`
- `mark_active_draft`
- `validate_inline_comments`

Changed command names:

- `prepare_submit_review` accepts the exact publish payload, including selected inline comments.
- `confirm_submit_review` accepts the same exact publish payload plus `confirmation_id`.

Legacy compatibility:

- `start_agent_run`, `read_agent_run`, `cancel_agent_run`, and `generate_review_draft` remain during the first migration pass and wrap the new analysis run implementation.
- `save_draft` remains during the first migration pass and writes a `ReviewDraft` when enough PR context is supplied.

## Locked Data Types

Use these Rust domain shapes as the canonical starting point.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisRunMode {
    Fast,
    Deep,
    Security,
    Tests,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisRunStatus {
    Queued,
    Running,
    DraftReady,
    Failed,
    Cancelled,
    Stale,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisRun {
    pub run_id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub head_sha: String,
    pub diff_hash: String,
    pub context_hash: String,
    pub mode: AnalysisRunMode,
    pub model_id: String,
    pub reasoning_effort: ReasoningEffort,
    pub review_language: Locale,
    pub custom_prompt: Option<String>,
    pub prompt_version: String,
    pub selected_files: Vec<String>,
    pub excluded_files: Vec<String>,
    pub private_diff_consent_snapshot: bool,
    pub status: AnalysisRunStatus,
    pub blocked_reason: Option<AiBlockedReason>,
    pub draft_seed_body: Option<String>,
    pub findings: Vec<ReviewFinding>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InlineMappingStatus {
    Valid,
    MissingPath,
    InvalidSide,
    InvalidLine,
    StaleDiff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineCommentDraft {
    pub id: String,
    pub path: String,
    pub side: String,
    pub line: u64,
    pub start_line: Option<u64>,
    pub start_side: Option<String>,
    pub body: String,
    pub severity: Option<String>,
    pub confidence: Option<u8>,
    pub source_run_id: Option<String>,
    pub source_finding_id: Option<String>,
    pub selected_for_publish: bool,
    pub dismissed: bool,
    pub user_edited: bool,
    pub mapping_status: InlineMappingStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewDraft {
    pub draft_id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub source_run_ids: Vec<String>,
    pub base_head_sha: String,
    pub base_diff_hash: String,
    pub verdict: ReviewEvent,
    pub body: String,
    pub inline_comments: Vec<InlineCommentDraft>,
    pub user_edited: bool,
    pub stale: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPublishPayload {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub expected_head_sha: String,
    pub expected_diff_hash: String,
    pub event: ReviewEvent,
    pub body: String,
    pub inline_comments: Vec<InlineCommentDraft>,
    pub explicit_verdict_confirmed: bool,
    pub private_diff_consent_required: bool,
    pub private_diff_consent_accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishAttempt {
    pub attempt_id: String,
    pub draft_id: Option<String>,
    pub confirmation_id: String,
    pub payload: ReviewPublishPayload,
    pub github_account: Option<String>,
    pub status: String,
    pub github_review_id: Option<u64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}
```

## File Structure

Create:

- `src/workspace_store.rs`: per-PR workspace paths and append/read helpers for runs, drafts, and publish attempts.
- `src/inline_comments.rs`: inline comment validation and GitHub comment payload conversion.
- `src/publish.rs`: publish payload, preflight blockers, confirmation id, and publish attempt helpers.
- `tests/workspace_store_tests.rs`: workspace persistence and path safety.
- `tests/inline_comments_tests.rs`: inline mapping validation.
- `tests/publish_tests.rs`: preflight, confirmation id, and payload exactness.
- `ui/src/lib/workspace-view-models.ts`: frontend run/draft/queue section derivation.
- `ui/src/lib/workspace-state.ts`: reducer-style selected PR, run, draft, inline, and publish state transitions.
- `ui/src/lib/publish-safety.ts`: frontend publish payload and blocker derivation.
- `ui/src/lib/inline-comments.ts`: frontend inline edit/select/dismiss helpers.
- `ui/src/lib/workspace-view-models.test.ts`
- `ui/src/lib/publish-safety.test.ts`
- `ui/src/lib/inline-comments.test.ts`
- `ui/src/components/AppShell.tsx`
- `ui/src/components/AssignedPrQueue.tsx`
- `ui/src/components/PrWorkspace.tsx`
- `ui/src/components/PrHeader.tsx`
- `ui/src/components/ChangedFileList.tsx`
- `ui/src/components/DiffReader.tsx`
- `ui/src/components/RunShelf.tsx`
- `ui/src/components/RunHistory.tsx`
- `ui/src/components/DraftPublishPanel.tsx`
- `ui/src/components/DraftBodyTab.tsx`
- `ui/src/components/InlineCommentsTab.tsx`
- `ui/src/components/SafetyTab.tsx`
- `ui/src/components/PublishConfirmation.tsx`

Modify:

- `src/lib.rs`: export new Rust modules.
- `src/domain.rs`: add canonical run, draft, inline, and publish domain types.
- `src/review.rs`: return or map generated output into `AnalysisRun` and `ReviewDraft` seeds.
- `src/storage.rs`: initialize `.reviewdesk/workspaces` and keep owner-only permissions.
- `src/github.rs`: add inline `comments` to `ReviewSubmitRequest`.
- `src/app_core.rs`: extend submit preflight input/view, allowed commands, and confirmation id.
- `src-tauri/src/commands.rs`: wire new commands and legacy wrappers.
- `src-tauri/src/main.rs`: register new commands.
- `src-tauri/build.rs`: generate command permissions for new commands.
- `src-tauri/capabilities/main.json`: allow new generated permissions.
- `ui/src/App.tsx`: reduce to setup/shell orchestration after components exist.
- `ui/src/lib/view-models.ts`: re-export or host shared frontend types that remain app-wide.
- `ui/src/lib/ipc.ts`: add new command wrappers and keep legacy wrappers.
- `ui/src/lib/i18n.ts`: add labels for run modes, queue sections, inline states, blockers, and publish modal.
- `ui/src/App.test.tsx`: add integration coverage for run history, draft preservation, stale warning, and publish preview.
- `ui/src/styles.css`: add responsive layout rules so Draft & Publish stacks instead of disappearing.

Do not expand `src/ui/*` egui files for this Tauri React slice.

## Task 1: Domain Types And Storage Layout

**Files:**
- Modify: `src/domain.rs`
- Modify: `src/storage.rs`
- Create: `src/workspace_store.rs`
- Modify: `src/lib.rs`
- Create: `tests/workspace_store_tests.rs`

- [ ] **Step 1: Write failing workspace storage tests**

Add `tests/workspace_store_tests.rs`:

```rust
use reviewdesk::domain::{AnalysisRun, AnalysisRunMode, AnalysisRunStatus, ReviewEvent, ReviewDraft};
use reviewdesk::workspace_store::{PrWorkspaceKey, WorkspaceStore};
use reviewdesk::app_core::{Locale, ReasoningEffort};

#[test]
fn workspace_store_groups_artifacts_by_pr_with_owner_only_permissions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
    let run = sample_run("run-a");
    let draft = sample_draft("draft-a", &run.run_id);

    let run_path = store.save_run(&key, &run).expect("save run");
    let draft_path = store.save_draft(&key, &draft).expect("save draft");
    store.mark_active_draft(&key, &draft.draft_id).expect("active draft");

    assert!(run_path.ends_with(".reviewdesk/workspaces/company/payment-web/582/runs/run-a.json"));
    assert!(draft_path.ends_with(".reviewdesk/workspaces/company/payment-web/582/drafts/draft-a.json"));
    assert_eq!(store.list_runs(&key).expect("runs").len(), 1);
    assert_eq!(store.read_active_draft_id(&key).expect("active"), Some("draft-a".to_string()));
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test workspace_store --test workspace_store_tests`

Expected: FAIL with unresolved import `reviewdesk::workspace_store`.

- [ ] **Step 3: Add domain types and module export**

In `src/lib.rs`, add:

```rust
pub mod workspace_store;
pub mod inline_comments;
pub mod publish;
```

In `src/domain.rs`, add the canonical types from **Locked Data Types**. Use `crate::app_core::{Locale, ReasoningEffort}` for run settings so frontend and backend naming stays aligned.

- [ ] **Step 4: Implement workspace store helpers**

Create `src/workspace_store.rs` with:

```rust
use crate::domain::{AnalysisRun, ReviewDraft, Result, ReviewDeskError};
use crate::storage::LocalStore;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrWorkspaceKey {
    owner: String,
    repo: String,
    number: u64,
}

impl PrWorkspaceKey {
    pub fn new(owner: &str, repo: &str, number: u64) -> Result<Self> {
        Ok(Self {
            owner: safe_segment(owner)?,
            repo: safe_segment(repo)?,
            number,
        })
    }

    fn prefix(&self) -> String {
        format!("workspaces/{}/{}/{}", self.owner, self.repo, self.number)
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceStore {
    local: LocalStore,
}

impl WorkspaceStore {
    pub fn init(project_root: impl AsRef<Path>) -> Result<Self> {
        Ok(Self { local: LocalStore::init(project_root)? })
    }

    pub fn save_run(&self, key: &PrWorkspaceKey, run: &AnalysisRun) -> Result<PathBuf> {
        self.local.save_json(&format!("{}/runs/{}.json", key.prefix(), safe_segment(&run.run_id)?), run)
    }

    pub fn list_runs(&self, key: &PrWorkspaceKey) -> Result<Vec<AnalysisRun>> {
        let dir = self.local.root().join(format!("{}/runs", key.prefix()));
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut runs = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) == Some("json") {
                let relative = path.strip_prefix(self.local.root()).map_err(|_| ReviewDeskError::InvalidPath(path.display().to_string()))?;
                runs.push(self.local.load_json(&relative.to_string_lossy())?);
            }
        }
        runs.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(runs)
    }

    pub fn save_draft(&self, key: &PrWorkspaceKey, draft: &ReviewDraft) -> Result<PathBuf> {
        self.local.save_json(&format!("{}/drafts/{}.json", key.prefix(), safe_segment(&draft.draft_id)?), draft)
    }

    pub fn read_draft(&self, key: &PrWorkspaceKey, draft_id: &str) -> Result<ReviewDraft> {
        self.local.load_json(&format!("{}/drafts/{}.json", key.prefix(), safe_segment(draft_id)?))
    }

    pub fn mark_active_draft(&self, key: &PrWorkspaceKey, draft_id: &str) -> Result<PathBuf> {
        self.local.save_json(&format!("{}/drafts/active.json", key.prefix()), &safe_segment(draft_id)?)
    }

    pub fn read_active_draft_id(&self, key: &PrWorkspaceKey) -> Result<Option<String>> {
        let path = format!("{}/drafts/active.json", key.prefix());
        match self.local.load_json::<String>(&path) {
            Ok(value) => Ok(Some(value)),
            Err(crate::domain::ReviewDeskError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn safe_segment(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.starts_with('.')
    {
        return Err(ReviewDeskError::InvalidPath(value.to_string()));
    }
    Ok(trimmed
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') { ch } else { '-' })
        .collect())
}
```

In `src/storage.rs`, include `workspaces` in the initialized child directories.

- [ ] **Step 5: Run storage tests**

Run: `cargo test workspace_store --test workspace_store_tests`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/domain.rs src/lib.rs src/storage.rs src/workspace_store.rs tests/workspace_store_tests.rs
git commit -m "feat: add run-centric workspace storage"
```

## Task 2: Analysis Run Identity And Legacy Agent Wrapper

**Files:**
- Modify: `src/review.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src/app_core.rs`
- Test: `tests/review_tests.rs`

- [ ] **Step 1: Write failing run identity tests**

Add to `tests/review_tests.rs`:

```rust
use reviewdesk::domain::{AnalysisRunMode, AnalysisRunStatus};
use reviewdesk::review::{new_analysis_run_id, stable_changed_files_hash};

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
        reviewdesk::domain::ChangedFile::new("b.rs", Some("@@ b")),
        reviewdesk::domain::ChangedFile::new("a.rs", Some("@@ a")),
    ];
    let right = vec![
        reviewdesk::domain::ChangedFile::new("a.rs", Some("@@ a")),
        reviewdesk::domain::ChangedFile::new("b.rs", Some("@@ b")),
    ];
    assert_eq!(stable_changed_files_hash(&left), stable_changed_files_hash(&right));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test analysis_run_ids_are_unique changed_files_hash_is_stable`

Expected: FAIL with missing functions.

- [ ] **Step 3: Implement unique run ids and stable hash**

In `src/review.rs`, add:

```rust
pub fn new_analysis_run_id(owner: &str, repo: &str, number: u64) -> String {
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let entropy = format!("{:x}", sha2::Sha256::digest(format!("{owner}/{repo}#{number}/{timestamp}").as_bytes()));
    format!(
        "run-{}-{}-{}-{}-{}",
        safe_id(owner),
        safe_id(repo),
        number,
        timestamp,
        &entropy[..8]
    )
}

pub fn stable_changed_files_hash(files: &[crate::domain::ChangedFile]) -> String {
    let mut normalized = files
        .iter()
        .map(|file| (file.path.as_str(), file.patch.as_deref().unwrap_or("")))
        .collect::<Vec<_>>();
    normalized.sort_by(|left, right| left.0.cmp(right.0));
    let mut hasher = sha2::Sha256::new();
    for (path, patch) in normalized {
        hasher.update(path.as_bytes());
        hasher.update(b"\0");
        hasher.update(patch.as_bytes());
        hasher.update(b"\0");
    }
    format!("{:x}", hasher.finalize())
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '-' })
        .collect()
}
```

Add needed imports at top of `src/review.rs`:

```rust
use sha2::{Digest, Sha256};
```

- [ ] **Step 4: Replace deterministic run id in Tauri command**

In `src-tauri/src/commands.rs`, replace the existing `run_id: format!("run-...diff...")` construction in `run_agent_review` with:

```rust
let run_id = reviewdesk::review::new_analysis_run_id(&request.owner, &request.repo, request.number);
let diff_hash = reviewdesk::review::stable_changed_files_hash(&request.files);
```

Map legacy `AgentRunRecordView` from the new identity while keeping existing response fields.

- [ ] **Step 5: Run run identity tests**

Run: `cargo test analysis_run_ids_are_unique changed_files_hash_is_stable`

Expected: PASS.

- [ ] **Step 6: Run current review pipeline tests**

Run: `cargo test --test review_tests`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/review.rs src-tauri/src/commands.rs tests/review_tests.rs
git commit -m "feat: make analysis runs unique"
```

## Task 3: Draft Persistence And Source Run Links

**Files:**
- Modify: `src/domain.rs`
- Modify: `src/workspace_store.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/app_core.rs`
- Test: `tests/workspace_store_tests.rs`
- Test: `tests/app_core_tests.rs`

- [ ] **Step 1: Write failing draft persistence tests**

Add to `tests/workspace_store_tests.rs`:

```rust
#[test]
fn draft_created_from_multiple_runs_preserves_source_run_ids() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = WorkspaceStore::init(temp.path()).expect("store");
    let key = PrWorkspaceKey::new("company", "payment-web", 582).expect("key");
    let mut draft = sample_draft("draft-combined", "run-a");
    draft.source_run_ids.push("run-b".to_string());

    store.save_draft(&key, &draft).expect("save draft");
    store.mark_active_draft(&key, "draft-combined").expect("active");
    let loaded = store.read_draft(&key, "draft-combined").expect("read draft");

    assert_eq!(loaded.source_run_ids, vec!["run-a".to_string(), "run-b".to_string()]);
    assert_eq!(store.read_active_draft_id(&key).expect("active"), Some("draft-combined".to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails where helpers are incomplete**

Run: `cargo test draft_created_from_multiple_runs --test workspace_store_tests`

Expected: FAIL until `read_draft` and active draft helpers are complete.

- [ ] **Step 3: Add command DTOs**

In `src-tauri/src/commands.rs`, add:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDraftFromRunRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub source_run_ids: Vec<String>,
    pub base_head_sha: String,
    pub base_diff_hash: String,
    pub body: String,
    pub event: ReviewEvent,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveReviewDraftRequest {
    pub draft: ReviewDraft,
    pub mark_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadReviewDraftRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub draft_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListReviewDraftsRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}
```

Add command functions `create_draft_from_run`, `save_review_draft`, `read_review_draft`, `list_review_drafts`, and `mark_active_draft`. Use `WorkspaceStore::init(current_dir)` and `PrWorkspaceKey::new`.

- [ ] **Step 4: Register and allow draft commands**

Add the new commands to:

- `src-tauri/src/main.rs`
- `src-tauri/build.rs`
- `src-tauri/capabilities/main.json`
- `src/app_core.rs::allowed_ipc_commands`

Command risk levels:

- `create_draft_from_run`: `writes_local`
- `save_review_draft`: `writes_local`
- `read_review_draft`: `read_only`
- `list_review_drafts`: `read_only`
- `mark_active_draft`: `writes_local`

- [ ] **Step 5: Run command allowlist tests**

Run: `cargo test tauri_build_manifest_lists_the_same_safe_commands_as_app_core tauri_capability_allows_all_safe_reviewdesk_commands`

Expected: PASS.

- [ ] **Step 6: Run draft storage tests**

Run: `cargo test draft_created_from_multiple_runs --test workspace_store_tests`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/domain.rs src/workspace_store.rs src-tauri/src/commands.rs src-tauri/src/main.rs src-tauri/build.rs src-tauri/capabilities/main.json src/app_core.rs tests/workspace_store_tests.rs tests/app_core_tests.rs
git commit -m "feat: persist review drafts per PR"
```

## Task 4: Inline Comment Validation

**Files:**
- Create: `src/inline_comments.rs`
- Modify: `src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/app_core.rs`
- Test: `tests/inline_comments_tests.rs`

- [ ] **Step 1: Write failing inline validation tests**

Create `tests/inline_comments_tests.rs`:

```rust
use reviewdesk::domain::{ChangedFile, InlineCommentDraft, InlineMappingStatus};
use reviewdesk::inline_comments::validate_inline_comments;

#[test]
fn validates_selected_inline_comment_on_added_line() {
    let files = vec![ChangedFile::new("src/lib.rs", Some("@@ -1,2 +1,3 @@\n context\n+added\n unchanged"))];
    let comments = vec![inline("src/lib.rs", "RIGHT", 2, true)];
    let validated = validate_inline_comments(&comments, &files, "diff").expect("validated");
    assert_eq!(validated[0].mapping_status, InlineMappingStatus::Valid);
}

#[test]
fn blocks_selected_inline_comment_with_missing_path() {
    let files = vec![ChangedFile::new("src/lib.rs", Some("@@ -1,1 +1,1 @@\n unchanged"))];
    let comments = vec![inline("src/missing.rs", "RIGHT", 1, true)];
    let validated = validate_inline_comments(&comments, &files, "diff").expect("validated");
    assert_eq!(validated[0].mapping_status, InlineMappingStatus::MissingPath);
}

fn inline(path: &str, side: &str, line: u64, selected: bool) -> InlineCommentDraft {
    InlineCommentDraft {
        id: format!("{path}:{line}"),
        path: path.to_string(),
        side: side.to_string(),
        line,
        start_line: None,
        start_side: None,
        body: "Please check this line.".to_string(),
        severity: Some("medium".to_string()),
        confidence: Some(80),
        source_run_id: Some("run-a".to_string()),
        source_finding_id: None,
        selected_for_publish: selected,
        dismissed: false,
        user_edited: false,
        mapping_status: InlineMappingStatus::Valid,
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test inline_comment --test inline_comments_tests`

Expected: FAIL with unresolved module `inline_comments`.

- [ ] **Step 3: Implement validation**

Create `src/inline_comments.rs`:

```rust
use crate::domain::{ChangedFile, InlineCommentDraft, InlineMappingStatus, Result};

pub fn validate_inline_comments(
    comments: &[InlineCommentDraft],
    files: &[ChangedFile],
    _diff_hash: &str,
) -> Result<Vec<InlineCommentDraft>> {
    let mut validated = Vec::new();
    for comment in comments {
        let mut next = comment.clone();
        next.mapping_status = validate_one(comment, files);
        validated.push(next);
    }
    Ok(validated)
}

fn validate_one(comment: &InlineCommentDraft, files: &[ChangedFile]) -> InlineMappingStatus {
    if !comment.selected_for_publish || comment.dismissed {
        return InlineMappingStatus::Valid;
    }
    if comment.side != "RIGHT" {
        return InlineMappingStatus::InvalidSide;
    }
    let Some(file) = files.iter().find(|file| file.path == comment.path) else {
        return InlineMappingStatus::MissingPath;
    };
    let Some(patch) = file.patch.as_deref() else {
        return InlineMappingStatus::MissingPath;
    };
    if patch_has_right_line(patch, comment.line) {
        InlineMappingStatus::Valid
    } else {
        InlineMappingStatus::InvalidLine
    }
}

fn patch_has_right_line(patch: &str, target: u64) -> bool {
    let mut new_line = 0_u64;
    for raw in patch.lines() {
        if raw.starts_with("@@") {
            new_line = parse_new_start(raw).unwrap_or(0);
            continue;
        }
        if raw.starts_with('-') {
            continue;
        }
        if raw.starts_with('+') || raw.starts_with(' ') || (!raw.starts_with('\\') && !raw.is_empty()) {
            if new_line == target {
                return true;
            }
            new_line += 1;
        }
    }
    false
}

fn parse_new_start(header: &str) -> Option<u64> {
    let plus = header.split_whitespace().find(|part| part.starts_with('+'))?;
    let start = plus.trim_start_matches('+').split(',').next()?;
    start.parse().ok()
}
```

- [ ] **Step 4: Add Tauri validation command**

Add `ValidateInlineCommentsRequest` and `validate_inline_comments` command in `src-tauri/src/commands.rs`.

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ValidateInlineCommentsRequest {
    pub comments: Vec<InlineCommentDraft>,
    pub files: Vec<ChangedFile>,
    pub diff_hash: String,
}
```

Register command and allowlist it as `read_only`.

- [ ] **Step 5: Run tests**

Run: `cargo test inline_comment --test inline_comments_tests`

Expected: PASS.

Run: `cargo test tauri_capability_allows_all_safe_reviewdesk_commands`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/inline_comments.rs src/lib.rs src-tauri/src/commands.rs src-tauri/src/main.rs src-tauri/build.rs src-tauri/capabilities/main.json src/app_core.rs tests/inline_comments_tests.rs
git commit -m "feat: validate inline review comments"
```

## Task 5: Publish Payload, Preflight, And GitHub Inline Submit

**Files:**
- Create: `src/publish.rs`
- Modify: `src/app_core.rs`
- Modify: `src/github.rs`
- Modify: `src-tauri/src/commands.rs`
- Test: `tests/publish_tests.rs`
- Test: `tests/github_tests.rs`

- [ ] **Step 1: Write failing publish tests**

Create `tests/publish_tests.rs`:

```rust
use reviewdesk::domain::{InlineMappingStatus, ReviewEvent, ReviewPublishPayload};
use reviewdesk::publish::{prepare_review_publish, PreparedReviewPublishStatus};

#[test]
fn submit_preflight_allows_body_empty_when_inline_comments_selected() {
    let payload = payload("", InlineMappingStatus::Valid, true);
    let view = prepare_review_publish(payload, true, true, false, true, false, "head", "diff");
    assert_eq!(view.status, PreparedReviewPublishStatus::Ready);
    assert!(view.confirmation_id.is_some());
}

#[test]
fn submit_preflight_blocks_invalid_inline_mappings() {
    let payload = payload("Looks good except one issue.", InlineMappingStatus::InvalidLine, true);
    let view = prepare_review_publish(payload, true, true, false, true, false, "head", "diff");
    assert_eq!(view.status, PreparedReviewPublishStatus::Blocked);
    assert!(view.blocked_reasons.contains(&"inline_mapping_invalid".to_string()));
}

#[test]
fn submit_confirmation_id_changes_when_inline_payload_changes() {
    let first = prepare_review_publish(payload("Body", InlineMappingStatus::Valid, true), true, true, false, true, false, "head", "diff");
    let second = prepare_review_publish(payload("Body changed", InlineMappingStatus::Valid, true), true, true, false, true, false, "head", "diff");
    assert_ne!(first.confirmation_id, second.confirmation_id);
}

fn payload(body: &str, mapping: InlineMappingStatus, selected: bool) -> ReviewPublishPayload {
    ReviewPublishPayload {
        owner: "company".to_string(),
        repo: "payment-web".to_string(),
        number: 582,
        expected_head_sha: "head".to_string(),
        expected_diff_hash: "diff".to_string(),
        event: ReviewEvent::Comment,
        body: body.to_string(),
        inline_comments: vec![reviewdesk::domain::InlineCommentDraft {
            id: "inline-1".to_string(),
            path: "src/lib.rs".to_string(),
            side: "RIGHT".to_string(),
            line: 12,
            start_line: None,
            start_side: None,
            body: "Please check this.".to_string(),
            severity: None,
            confidence: None,
            source_run_id: None,
            source_finding_id: None,
            selected_for_publish: selected,
            dismissed: false,
            user_edited: true,
            mapping_status: mapping,
        }],
        explicit_verdict_confirmed: false,
        private_diff_consent_required: false,
        private_diff_consent_accepted: false,
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test publish --test publish_tests`

Expected: FAIL with unresolved module `publish`.

- [ ] **Step 3: Implement publish preflight**

Create `src/publish.rs` with `ReviewPublishPayload` imported from `domain`, `PreparedReviewPublishStatus`, `PreparedReviewPublishView`, and `prepare_review_publish`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedReviewPublishStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedReviewPublishView {
    pub status: PreparedReviewPublishStatus,
    pub blocked_reasons: Vec<String>,
    pub confirmation_id: Option<String>,
    pub payload: ReviewPublishPayload,
}
```

Reuse existing blocker strings and add:

- `diff_changed`
- `inline_mapping_invalid`
- `payload_empty`

Confirmation id must hash:

- owner
- repo
- number
- expected head SHA
- expected diff hash
- event
- body
- selected inline comment path, side, line, start line, body

- [ ] **Step 4: Extend GitHub review submit payload**

In `src/github.rs`, replace `ReviewSubmitRequest` with:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewSubmitRequest {
    pub commit_id: String,
    pub event: ReviewEvent,
    pub body: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<ReviewSubmitComment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewSubmitComment {
    pub path: String,
    pub side: String,
    pub line: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_side: Option<String>,
    pub body: String,
}
```

Update existing tests to pass `comments: vec![]`.

- [ ] **Step 5: Add GitHub inline payload test**

Add to `tests/github_tests.rs`:

```rust
#[tokio::test]
async fn submits_review_body_with_inline_comments_payload() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST)
            .path("/repos/company/payment-web/pulls/582/reviews")
            .json_body_obj(&serde_json::json!({
                "commit_id": "sha",
                "event": "REQUEST_CHANGES",
                "body": "Please fix the retry path.",
                "comments": [{
                    "path": "src/lib.rs",
                    "side": "RIGHT",
                    "line": 12,
                    "body": "This retry can double-submit."
                }]
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({"id": 99}));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");
    let review = client
        .submit_review(
            "company",
            "payment-web",
            582,
            &ReviewSubmitRequest {
                commit_id: "sha".to_string(),
                event: ReviewEvent::RequestChanges,
                body: "Please fix the retry path.".to_string(),
                comments: vec![ReviewSubmitComment {
                    path: "src/lib.rs".to_string(),
                    side: "RIGHT".to_string(),
                    line: 12,
                    start_line: None,
                    start_side: None,
                    body: "This retry can double-submit.".to_string(),
                }],
            },
        )
        .await
        .expect("review submit");

    assert_eq!(review.id, 99);
}
```

- [ ] **Step 6: Wire Tauri prepare/confirm submit**

In `src-tauri/src/commands.rs`, update `ConfirmSubmitReviewRequest` to hold `payload: ReviewPublishPayload`. `prepare_submit_review` should call the new publish preflight. `confirm_submit_review` should:

1. Fetch current snapshot.
2. Recompute preflight with current head and diff hash.
3. Compare confirmation id.
4. Convert selected inline comments into `ReviewSubmitComment`.
5. Submit `ReviewSubmitRequest`.
6. Save `PublishAttempt` with success or failure.

- [ ] **Step 7: Run publish tests**

Run: `cargo test publish --test publish_tests`

Expected: PASS.

Run: `cargo test submits_review_body_with_inline_comments_payload --test github_tests`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/publish.rs src/app_core.rs src/github.rs src-tauri/src/commands.rs tests/publish_tests.rs tests/github_tests.rs
git commit -m "feat: publish reviews with inline comments"
```

## Task 6: Frontend Types, IPC, And View Models

**Files:**
- Modify: `ui/src/lib/view-models.ts`
- Modify: `ui/src/lib/ipc.ts`
- Create: `ui/src/lib/workspace-view-models.ts`
- Create: `ui/src/lib/inline-comments.ts`
- Create: `ui/src/lib/publish-safety.ts`
- Create: `ui/src/lib/workspace-view-models.test.ts`
- Create: `ui/src/lib/inline-comments.test.ts`
- Create: `ui/src/lib/publish-safety.test.ts`

- [ ] **Step 1: Write failing frontend view-model tests**

Create `ui/src/lib/workspace-view-models.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { groupRunCentricQueue, isDraftStale } from "./workspace-view-models";

describe("workspace view models", () => {
  it("groups queue into run-centric sections", () => {
    const sections = groupRunCentricQueue([
      queueItem({ number: 1 }),
      queueItem({ number: 2, latest_run_status: "running" }),
      queueItem({ number: 3, active_draft_id: "draft-3" }),
      queueItem({ number: 4, stale_status: "head_changed" }),
      queueItem({ number: 5, published_review_id: 99 }),
    ]);
    expect(sections.map((section) => section.id)).toEqual([
      "needs_review",
      "draft_ready",
      "running",
      "blocked_stale",
      "published",
    ]);
  });

  it("derives stale draft from head and diff mismatch", () => {
    expect(isDraftStale({ base_head_sha: "old", base_diff_hash: "diff" }, "new", "diff")).toBe(true);
    expect(isDraftStale({ base_head_sha: "head", base_diff_hash: "old" }, "head", "new")).toBe(true);
    expect(isDraftStale({ base_head_sha: "head", base_diff_hash: "diff" }, "head", "diff")).toBe(false);
  });
});
```

Create `ui/src/lib/inline-comments.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { selectInlineComment, dismissInlineComment } from "./inline-comments";

describe("inline comment helpers", () => {
  it("selects and dismisses inline comments without changing body text", () => {
    const comment = { id: "c1", body: "Check this", selected_for_publish: false, dismissed: false };
    expect(selectInlineComment(comment, true).selected_for_publish).toBe(true);
    const dismissed = dismissInlineComment(comment);
    expect(dismissed.dismissed).toBe(true);
    expect(dismissed.body).toBe("Check this");
  });
});
```

Create `ui/src/lib/publish-safety.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { buildPublishPayload, derivePublishBlockers } from "./publish-safety";

describe("publish safety", () => {
  it("allows empty body when one valid inline comment is selected", () => {
    const payload = buildPublishPayload({
      owner: "company",
      repo: "payment-web",
      number: 582,
      expected_head_sha: "head",
      expected_diff_hash: "diff",
      event: "COMMENT",
      body: "",
      inline_comments: [{ id: "c1", selected_for_publish: true, dismissed: false, mapping_status: "valid", body: "Fix this" }],
      explicit_verdict_confirmed: false,
      private_diff_consent_required: false,
      private_diff_consent_accepted: false,
    });
    expect(derivePublishBlockers(payload, { current_head_sha: "head", current_diff_hash: "diff" })).toEqual([]);
  });

  it("blocks unresolved inline mappings", () => {
    const payload = buildPublishPayload({
      owner: "company",
      repo: "payment-web",
      number: 582,
      expected_head_sha: "head",
      expected_diff_hash: "diff",
      event: "COMMENT",
      body: "Body",
      inline_comments: [{ id: "c1", selected_for_publish: true, dismissed: false, mapping_status: "invalid_line", body: "Fix this" }],
      explicit_verdict_confirmed: false,
      private_diff_consent_required: false,
      private_diff_consent_accepted: false,
    });
    expect(derivePublishBlockers(payload, { current_head_sha: "head", current_diff_hash: "diff" })).toContain("inline_mapping_invalid");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- ui/src/lib/workspace-view-models.test.ts ui/src/lib/inline-comments.test.ts ui/src/lib/publish-safety.test.ts`

Expected: FAIL with missing modules.

- [ ] **Step 3: Add frontend model modules**

Implement the three modules with pure functions only. Keep TypeScript names aligned with Rust serde names:

- `AnalysisRunView`
- `ReviewDraftView`
- `InlineCommentDraftView`
- `ReviewPublishPayloadView`
- `RunCentricQueueSection`

Use snake_case properties for IPC payloads and camelCase only for component-only derived view props.

- [ ] **Step 4: Add IPC wrappers**

In `ui/src/lib/ipc.ts`, add wrappers for the new command names and update `SubmitPreflightInput`, `ConfirmSubmitReviewInput`, and `SubmittedReviewView` to use the exact publish payload.

- [ ] **Step 5: Run frontend model tests**

Run: `npm test -- ui/src/lib/workspace-view-models.test.ts ui/src/lib/inline-comments.test.ts ui/src/lib/publish-safety.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add ui/src/lib/view-models.ts ui/src/lib/ipc.ts ui/src/lib/workspace-view-models.ts ui/src/lib/inline-comments.ts ui/src/lib/publish-safety.ts ui/src/lib/workspace-view-models.test.ts ui/src/lib/inline-comments.test.ts ui/src/lib/publish-safety.test.ts
git commit -m "feat: add workspace frontend models"
```

## Task 7: Frontend State Split And Workspace Components

**Files:**
- Create: `ui/src/lib/workspace-state.ts`
- Create: component files listed in **File Structure**
- Modify: `ui/src/App.tsx`
- Modify: `ui/src/App.test.tsx`
- Modify: `ui/src/styles.css`

- [ ] **Step 1: Write failing App integration tests**

Add to `ui/src/App.test.tsx`:

```ts
it("shows multiple runs without overwriting the active draft", async () => {
  const queue = sampleQueue();
  vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
  vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
  vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
  vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));
  vi.mocked(ipc.listAnalysisRuns).mockResolvedValue([
    sampleAnalysisRun({ run_id: "run-a", mode: "fast" }),
    sampleAnalysisRun({ run_id: "run-b", mode: "deep" }),
  ]);
  vi.mocked(ipc.readReviewDraft).mockResolvedValue(sampleReviewDraft({ draft_id: "draft-a", body: "User edited draft" }));

  render(<App />);

  expect(await screen.findByText("Run history")).toBeTruthy();
  expect(await screen.findByText("run-a")).toBeTruthy();
  expect(await screen.findByText("run-b")).toBeTruthy();
  expect(await screen.findByDisplayValue("User edited draft")).toBeTruthy();
});
```

Add helper mocks in the test file after existing imports:

```ts
function sampleAnalysisRun(overrides = {}) {
  return {
    run_id: "run-a",
    owner: "company",
    repo: "payment-web",
    number: 582,
    head_sha: "head",
    diff_hash: "diff",
    context_hash: "context",
    mode: "fast",
    model_id: "gpt-5.5",
    reasoning_effort: "medium",
    review_language: "en",
    custom_prompt: null,
    prompt_version: "reviewdesk-run-centric-v1",
    selected_files: ["src/lib.rs"],
    excluded_files: [],
    private_diff_consent_snapshot: true,
    status: "draft_ready",
    blocked_reason: null,
    draft_seed_body: "AI draft",
    findings: [],
    created_at: "2026-05-17T00:00:00Z",
    completed_at: "2026-05-17T00:00:01Z",
    ...overrides,
  };
}

function sampleReviewDraft(overrides = {}) {
  return {
    draft_id: "draft-a",
    owner: "company",
    repo: "payment-web",
    number: 582,
    source_run_ids: ["run-a"],
    base_head_sha: "head",
    base_diff_hash: "diff",
    verdict: "COMMENT",
    body: "User edited draft",
    inline_comments: [],
    user_edited: true,
    stale: false,
    created_at: "2026-05-17T00:00:00Z",
    updated_at: "2026-05-17T00:00:02Z",
    ...overrides,
  };
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- ui/src/App.test.tsx`

Expected: FAIL with missing mocked IPC functions or missing UI text.

- [ ] **Step 3: Implement `workspace-state.ts`**

Define reducer actions:

- `select_pr`
- `load_runs_success`
- `load_draft_success`
- `start_run`
- `run_completed`
- `edit_draft_body`
- `replace_draft_from_run`
- `save_draft_success`
- `validate_inline_success`
- `prepare_publish_success`

Rule: `run_completed` never changes `activeDraft` when `activeDraft.user_edited` is true.

- [ ] **Step 4: Create presentational components**

Create the component files listed in **File Structure**. Keep each component focused:

- `AssignedPrQueue` receives queue sections and selection callbacks.
- `PrWorkspace` receives context, selected file, runs, and diff callbacks.
- `RunShelf` receives quick run callbacks and custom run callback.
- `DraftPublishPanel` receives draft, inline comments, safety state, and publish callbacks.
- `PublishConfirmation` receives the exact payload object returned by `buildPublishPayload`.

- [ ] **Step 5: Refactor `App.tsx` to orchestrate only**

Keep in `App.tsx`:

- initial status loading
- auth/setup screen decision
- selected screen
- data fetching calls
- passing state to `AppShell`

Move rendering details to components and state transitions to `workspace-state.ts`.

- [ ] **Step 6: Add responsive layout**

In `ui/src/styles.css`, ensure the draft/publish panel stacks below the workspace when the viewport is narrow. Do not hide the panel at widths below 1024px.

- [ ] **Step 7: Run frontend tests**

Run: `npm test -- ui/src/App.test.tsx ui/src/lib/workspace-view-models.test.ts ui/src/lib/publish-safety.test.ts ui/src/lib/inline-comments.test.ts`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add ui/src/App.tsx ui/src/App.test.tsx ui/src/styles.css ui/src/lib/workspace-state.ts ui/src/components/AppShell.tsx ui/src/components/AssignedPrQueue.tsx ui/src/components/PrWorkspace.tsx ui/src/components/PrHeader.tsx ui/src/components/ChangedFileList.tsx ui/src/components/DiffReader.tsx ui/src/components/RunShelf.tsx ui/src/components/RunHistory.tsx ui/src/components/DraftPublishPanel.tsx ui/src/components/DraftBodyTab.tsx ui/src/components/InlineCommentsTab.tsx ui/src/components/SafetyTab.tsx ui/src/components/PublishConfirmation.tsx
git commit -m "feat: split run-centric workspace UI"
```

## Task 8: Tauri Permission Surface And End-To-End Verification

**Files:**
- Modify: `src-tauri/build.rs`
- Modify: `src-tauri/capabilities/main.json`
- Generated: `src-tauri/permissions/autogenerated/*.toml`
- Test: `tests/tauri_shell_tests.rs`
- Docs: `docs/verification/reviewdesk-run-centric-workspace.md`

- [ ] **Step 1: Extend Tauri shell tests**

Add to `tests/tauri_shell_tests.rs`:

```rust
#[test]
fn tauri_allowlist_contains_run_draft_inline_publish_commands() {
    let build_rs = fs::read_to_string("src-tauri/build.rs").expect("build.rs exists");
    for command in [
        "list_analysis_runs",
        "start_analysis_run",
        "read_analysis_run",
        "cancel_analysis_run",
        "archive_analysis_run",
        "create_draft_from_run",
        "save_review_draft",
        "read_review_draft",
        "list_review_drafts",
        "mark_active_draft",
        "validate_inline_comments",
    ] {
        assert!(build_rs.contains(command), "missing command {command}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails if commands are missing**

Run: `cargo test tauri_allowlist_contains_run_draft_inline_publish_commands --test tauri_shell_tests`

Expected: FAIL until every command is listed.

- [ ] **Step 3: Regenerate permissions**

Run: `npm run desktop:build`

Expected: Tauri build succeeds or reaches platform packaging after generating permissions. If packaging fails for environment reasons after Rust/TS compilation, capture the last successful generation step and run `cargo test --test tauri_shell_tests`.

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo fmt --check
cargo test
npm test
npm run build
```

Expected: all pass.

- [ ] **Step 5: Record verification**

Create `docs/verification/reviewdesk-run-centric-workspace.md`:

```markdown
# ReviewDesk Run-Centric Workspace Verification

Date: 2026-05-17

## Commands

- `cargo fmt --check`: pass
- `cargo test`: pass
- `npm test`: pass
- `npm run build`: pass

## Verified Behaviors

- Repeated analysis runs are unique and durable.
- Drafts persist per PR and retain source run ids.
- User-edited drafts are not overwritten by late run completion.
- Inline comments validate against current diff lines.
- Publish preflight blocks stale head, stale diff, invalid mappings, closed or merged PRs, auth/scope/SSO blockers, and empty payload.
- Publish preview and confirm submit use the same payload shape.
- GitHub review submission includes selected inline comments.
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/build.rs src-tauri/capabilities/main.json src-tauri/permissions/autogenerated tests/tauri_shell_tests.rs docs/verification/reviewdesk-run-centric-workspace.md
git commit -m "test: verify run-centric workspace release surface"
```

## Development Start Criteria

Development can start when this plan is accepted and an execution path is chosen. The first implementation task must begin with Task 1 tests. No production code should be changed before the failing tests for that task are written and observed.

## Execution Options

1. **Subagent-Driven (recommended)**: dispatch one worker per task or small task pair, review between tasks, and keep file ownership narrow.
2. **Inline Execution**: execute this plan in the current session with checkpoints after each task.
