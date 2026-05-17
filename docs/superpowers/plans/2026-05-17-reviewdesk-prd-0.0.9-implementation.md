# ReviewDesk PRD 0.0.9 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/prd/prd-0.0.9.md` so ReviewDesk has release-ready GitHub/Codex connection handling, Review Inbox/context contracts, submit safety, i18n coverage, packaging gates, and updated verification documentation.

**Architecture:** Keep the existing Rust core + Tauri command + React UI split. Add focused contracts to `src/github.rs` and `src/app_core.rs`, keep Tauri commands thin, and expose richer view models to the frontend without storing raw credentials. Use tests to lock each contract before implementation.

**Tech Stack:** Rust, Tauri v2, reqwest, serde, chrono, React, TypeScript, Vitest, Cargo tests, GitHub REST API, Codex managed auth bridge.

---

### Task 1: GitHub OAuth Broker And Auth State Contract

**Files:**
- Modify: `src/github.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `tests/github_tests.rs`
- Modify: `tests/app_core_tests.rs`

- [ ] **Step 1: Write failing tests**
  - Add tests proving browser OAuth start requires a broker URL, creates state/PKCE-bound session metadata, and does not accept desktop `client_secret`.
  - Add tests proving GitHub auth state uses normalized `github_scope_insufficient`, `github_sso_required`, `github_rate_limited`, `github_network_error`.

- [ ] **Step 2: Run tests to verify RED**
  - Run: `cargo test github_oauth_broker --test github_tests`
  - Expected: FAIL because broker helpers and normalized state contract do not exist yet.

- [ ] **Step 3: Implement minimal broker contract**
  - Add broker URL validation and broker authorize/session view.
  - Remove direct desktop browser token exchange from default path.
  - Keep GitHub device flow as fallback.

- [ ] **Step 4: Run tests to verify GREEN**
  - Run: `cargo test github_oauth_broker --test github_tests`
  - Expected: PASS.

### Task 2: GitHub Review Inbox And Context Contract

**Files:**
- Modify: `src/domain.rs`
- Modify: `src/github.rs`
- Modify: `tests/github_tests.rs`

- [ ] **Step 1: Write failing tests**
  - Add tests for `user-review-requested:@me` / `assignee:@me` search queries with `archived:false`.
  - Add tests for independent pagination of review-requested and assigned search queries.
  - Add tests for duplicate PR reason `both`.
  - Add tests for typed PR context view: patch coverage, conversation summary, CI rollup, `diff_hash`, `context_hash`.

- [ ] **Step 2: Run tests to verify RED**
  - Run: `cargo test --test github_tests`
  - Expected: FAIL on new queue/context contract tests.

- [ ] **Step 3: Implement minimal contract**
  - Update query builders.
  - Add queue pagination and partial result metadata where the existing API can support it.
  - Add typed context view structs and conversion from raw GitHub responses.
  - Add deterministic CI rollup mapping.

- [ ] **Step 4: Run tests to verify GREEN**
  - Run: `cargo test --test github_tests`
  - Expected: PASS.

### Task 3: Stale Detection And Submit Safety

**Files:**
- Modify: `src/domain.rs`
- Modify: `src/github.rs`
- Modify: `src/app_core.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `tests/app_core_tests.rs`
- Modify: `tests/github_tests.rs`

- [ ] **Step 1: Write failing tests**
  - Add tests for stale enum values: `fresh`, `unknown`, `head_changed`, `diff_changed`, `context_changed`, `pr_closed`, `pr_merged`.
  - Add tests for confirmation ID binding to body hash, event, head SHA, account hash, and TTL.
  - Add tests that `confirm_submit_review` consumes confirmation and uses `commit_id=headSha`.

- [ ] **Step 2: Run tests to verify RED**
  - Run: `cargo test --test app_core_tests --test github_tests`
  - Expected: FAIL on stale/submit additions.

- [ ] **Step 3: Implement submit safety contract**
  - Add stable confirmation data model.
  - Normalize submit blocked reasons.
  - Ensure GitHub submit request includes `commit_id`.

- [ ] **Step 4: Run tests to verify GREEN**
  - Run: `cargo test --test app_core_tests --test github_tests`
  - Expected: PASS.

### Task 4: Frontend View Models And i18n

**Files:**
- Modify: `ui/src/lib/view-models.ts`
- Modify: `ui/src/lib/i18n.ts`
- Modify: `ui/src/lib/view-models.test.ts`
- Modify: `ui/src/lib/auth-flow.ts`
- Modify: `ui/src/lib/auth-flow.test.ts`
- Modify: `ui/src/App.tsx`

- [ ] **Step 1: Write failing tests**
  - Add i18n inventory test for all PRD 0.0.9 GitHub/Codex states, inbox errors, context warnings, blocked reasons, submit dock warnings, diagnostics keys, and consent messages.
  - Add view-model tests for broker-backed browser OAuth labels, device fallback, PR detail warnings, and submit dock disabled reasons.

- [ ] **Step 2: Run tests to verify RED**
  - Run: `npm test -- --run`
  - Expected: FAIL on missing i18n keys/view model helpers.

- [ ] **Step 3: Implement frontend contract**
  - Add keys and helpers.
  - Update visible labels from direct browser OAuth to broker-backed browser OAuth.
  - Add PR detail and submit dock state helpers that match Rust normalized reasons.

- [ ] **Step 4: Run tests to verify GREEN**
  - Run: `npm test -- --run`
  - Expected: PASS.

### Task 5: Packaging And Documentation Gates

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `docs/setup/reviewdesk-desktop-runbook.md`
- Create: `docs/setup/github-oauth.md`
- Create: `docs/setup/codex-managed-login.md`
- Create: `docs/security/oauth-secret-policy.md`
- Modify: `docs/verification/reviewdesk-tauri-0.0.9.md`
- Modify: `docs/prd/prd-0.0.9.md`

- [ ] **Step 1: Write failing checks**
  - Add or update tests that assert Tauri version is `0.0.9`, bundle is active, and broad capabilities are still absent.

- [ ] **Step 2: Run tests to verify RED if current config is stale**
  - Run: `cargo test --test tauri_shell_tests`
  - Expected: FAIL while version/bundle metadata is still old.

- [ ] **Step 3: Update config/docs**
  - Set Tauri version to `0.0.9`.
  - Enable bundle metadata with the existing icon.
  - Add setup/security docs required by the PRD.
  - Update verification evidence after running commands.

- [ ] **Step 4: Full verification**
  - Run:
    - `cargo fmt --check`
    - `cargo test`
    - `npm test -- --run`
    - `npm run build`
    - `cargo check --manifest-path src-tauri/Cargo.toml`
    - `npm run build && cargo tauri build`
  - Expected: all automated commands either pass or any limitation is explicitly recorded in `docs/verification/reviewdesk-tauri-0.0.9.md`.

### Self-Review

- Spec coverage: PRD 0.0.9 sections map to Tasks 1-5.
- Placeholder scan: no placeholder tasks remain.
- Type consistency: Rust/TypeScript names use normalized `github_*` and `codex_*` reason names from the PRD.
