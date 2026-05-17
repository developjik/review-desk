# ReviewDesk PRD 0.0.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the ReviewDesk 0.0.1 Rust native desktop MVP described in `docs/prd/reviewdesk-prd-0.0.1.md`.

**Architecture:** Implement a Rust crate with a testable core library and a thin `egui`/`eframe` desktop shell. The Core MVP must work without AI by showing `AI_BLOCKED_UNSUPPORTED_AUTH`, while Conditional AI is represented by an official provider adapter boundary that can use Codex app-server when available.

**Tech Stack:** Rust 1.95, `eframe`/`egui`, `tokio`, `reqwest`, `serde`, `keyring`, `regex`, `globset`, `chrono`, `thiserror`, `url`, `sha2`, `webbrowser`, `tempfile`, `httpmock`.

---

## File Map

- `Cargo.toml`: Rust package metadata and dependencies.
- `src/main.rs`: Native desktop entry point.
- `src/lib.rs`: Library module exports.
- `src/app.rs`: `egui` app state, panels, and UI event handlers.
- `src/domain.rs`: Shared domain structs/enums for repositories, PRs, review runs, findings, drafts, AI state, and errors.
- `src/security.rs`: Secret masking, private diff consent model, high-risk detection.
- `src/storage.rs`: `.reviewdesk/` local storage, owner-only permissions, accidental commit protection, JSON persistence.
- `src/github.rs`: GitHub OAuth Device Flow, REST API client, query builders, stale check, rate-limit/scope/SSO error classification.
- `src/ai.rs`: AI provider trait, blocked provider, mock provider for tests, Codex app-server provider boundary.
- `src/review.rs`: Review input construction, diff filtering, report generation, state transitions.
- `tests/security_tests.rs`: Secret masking and storage safety behavior.
- `tests/github_tests.rs`: GitHub query/API payload/error behavior with mocked HTTP.
- `tests/review_tests.rs`: Review state machine, stale check, AI blocked/manual draft behavior.
- `docs/implementation/reviewdesk-0.0.1.md`: Implementation notes and verification status.

## Tasks

### Task 1: Scaffold Rust App

- [x] Create `Cargo.toml`, `src/main.rs`, and `src/lib.rs`.
- [x] Add dependencies for GUI, async HTTP, serialization, keychain, masking, local storage, and tests.
- [x] Run `cargo test` and verify the empty baseline compiles.

### Task 2: Domain Model

- [x] Add `src/domain.rs` with PRD-backed structs and enums.
- [x] Add tests for review run status serialization and AI blocked status.
- [x] Run domain coverage through `cargo test`.

### Task 3: Security And Storage

- [x] Add failing tests for masking GitHub tokens, JWTs, private keys, env secrets, webhook URLs, and generic API keys.
- [x] Implement `SecretMasker`.
- [x] Add failing tests for `.reviewdesk/` owner-only creation and `.reviewdesk/.gitignore`.
- [x] Implement `LocalStore`.
- [x] Run `cargo test security_tests`.

### Task 4: GitHub Client

- [x] Add tests for review-requested and assigned search queries.
- [x] Add tests for top-level review payload and stale check behavior.
- [x] Add mocked HTTP tests for Device Flow `slow_down`, Issues Comments API, pagination link handling, and rate-limit classification.
- [x] Implement `GitHubClient`.
- [x] Run `cargo test github_tests`.

### Task 5: AI Provider Boundary

- [x] Add tests for `BlockedAiProvider`, model/reasoning option behavior, and mock provider output.
- [x] Implement `AiProvider`, `BlockedAiProvider`, `MockAiProvider`, and `CodexAppServerProvider` boundary.
- [x] Run AI provider coverage through `cargo test`.

### Task 6: Review Pipeline

- [x] Add tests for diff filtering, review input creation, manual draft path, AI blocked path, Markdown report generation, and state transitions.
- [x] Implement `ReviewPipeline`.
- [x] Run `cargo test review_tests`.

### Task 7: Desktop UI

- [x] Implement `ReviewDeskApp` with four visible areas: auth/provider status, repository/PR selection, analysis/result, draft/submit.
- [x] Ensure long-running actions are represented as task states and do not block the UI thread.
- [x] Expose AI blocked state, manual draft, model/depth selectors, private diff consent, stale submit blocking, and error messages.
- [x] Run `cargo check`.

### Task 8: Docs And Final Verification

- [x] Update `docs/implementation/reviewdesk-0.0.1.md` with implemented scope, AI provider caveat, commands, and verification output.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo test`.
- [x] Run `cargo check`.
- [x] Re-scan PRD and implementation docs for stale contradictions.
