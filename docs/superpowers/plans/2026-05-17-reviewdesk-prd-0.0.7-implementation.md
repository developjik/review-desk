# ReviewDesk PRD 0.0.7 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement `docs/prd/prd-0.0.7.md` by making browser OAuth the default GitHub/Codex authentication UX, preserving secure token boundaries, and updating verification docs.

**Architecture:** Add GitHub browser OAuth primitives to the Rust core, then expose them through narrow Tauri commands that own loopback callback handling and keychain writes. React keeps only auth metadata and renders browser pending, fallback, and blocked states without ever seeing tokens, OAuth codes, PKCE verifiers, or client secrets.

**Tech Stack:** Rust 2024, Tauri v2, Tokio, Reqwest, URL, SHA-256 PKCE, React 19, TypeScript, Vitest, Cargo tests.

---

## File Map

- `src/github.rs`: GitHub OAuth URL construction, PKCE helpers, authorization-code token exchange, and OAuth error parsing.
- `src/security.rs`: secret masking patterns for GitHub OAuth token/code/client secret strings.
- `src/app_core.rs`: auth status/blocked reason extensions and IPC command allowlist additions.
- `src-tauri/src/commands.rs`: browser-loopback GitHub OAuth coordinator, cancel/logout/refresh commands, Codex browser-login UX status.
- `src-tauri/src/main.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/main.json`: Tauri command registration and permissions.
- `ui/src/lib/ipc.ts`, `ui/src/lib/view-models.ts`, `ui/src/lib/view-models.test.ts`: browser OAuth response types, pending/fallback states, view-model behavior tests.
- `ui/src/App.tsx`, `ui/src/lib/i18n.ts`: browser sign-in primary CTA, fallback Device Flow action, status copy, and GitHub/Codex naming cleanup.
- `tests/github_tests.rs`, `tests/security_tests.rs`, `tests/app_core_tests.rs`, `tests/tauri_shell_tests.rs`: RED/GREEN coverage for OAuth URL, token exchange, redaction, command surface.
- `docs/verification/reviewdesk-tauri-0.0.7.md`: final implementation/verification record.

## Tasks

### Task 1: GitHub Browser OAuth Core

- [x] Add failing tests in `tests/github_tests.rs` for authorize URL generation, PKCE challenge, token exchange request body, and OAuth error mapping.
- [x] Run `cargo test --test github_tests github_browser_oauth` and confirm the new tests fail.
- [x] Implement minimal helpers in `src/github.rs`.
- [x] Run `cargo test --test github_tests` and confirm pass.

### Task 2: Secret Redaction and IPC Contract

- [x] Add failing tests in `tests/security_tests.rs` and `tests/app_core_tests.rs` for GitHub OAuth secrets and new IPC commands.
- [x] Run targeted tests and confirm failure.
- [x] Extend secret masking and allowed IPC commands.
- [x] Run targeted tests and confirm pass.

### Task 3: Tauri Browser OAuth Flow

- [x] Add failing Tauri command/capability tests for `cancel_github_oauth`, `logout_github`, and `refresh_github_auth_status`.
- [x] Run `cargo test --test tauri_shell_tests` and confirm failure.
- [x] Implement browser-loopback GitHub OAuth state, local listener, token exchange, cancel/logout/refresh commands, and command registration.
- [x] Run `cargo test --test tauri_shell_tests` and `cargo check --manifest-path src-tauri/Cargo.toml`.

### Task 4: Frontend Browser Auth UX

- [x] Add failing Vitest coverage for browser OAuth metadata, fallback Device Flow visibility, `Continue without AI`, and `Codex ChatGPT` naming.
- [x] Run `npm test -- --run` and confirm failure.
- [x] Update IPC types, App UI, i18n copy, and view models.
- [x] Run `npm test -- --run` and `npm run build`.

### Task 5: Verification and Docs

- [x] Create `docs/verification/reviewdesk-tauri-0.0.7.md`.
- [x] Run full verification: `cargo fmt --check`, `cargo test`, `npm test -- --run`, `npm run build`, `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] Update verification doc with PASS/remaining risks.
