# ReviewDesk PRD 0.0.5 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement PRD 0.0.5 auth-first navigation, multi-workspace IA, English/Korean i18n, and verification documentation.

**Architecture:** Rust AppCore remains the source of truth for auth, capabilities, submit preflight, command risk, and safe language preferences. React owns screen state, command/filter UI, workspace layout, draft editing, and localized copy. Tauri IPC exposes narrow view models and never returns token material.

**Tech Stack:** Rust, Tauri v2, React 19, TypeScript, Vite, Tailwind CSS, Vitest, cargo test.

---

### Task 1: Core Capability And Language Model

**Files:**
- Modify: `src/app_core.rs`
- Modify: `tests/app_core_tests.rs`

- [ ] Write failing Rust tests for strict auth mode, GitHub-only partial access, language defaults, review language priority, and missing token serialization.
- [ ] Run `cargo test app_core_tests` and confirm new tests fail because the capability and i18n models do not exist.
- [ ] Add `Locale`, `LanguagePreferences`, `AppCapability`, `AppStatusInput`, and `frontend_safe_app_status_from_input`.
- [ ] Keep existing `frontend_safe_app_status(github_connected, chatgpt_connected)` as a compatibility wrapper.
- [ ] Run `cargo test app_core_tests` and confirm the new tests pass.

### Task 2: Safe URL And Submit Preflight Tightening

**Files:**
- Modify: `src/app_core.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `tests/app_core_tests.rs`

- [ ] Write failing tests for HTTPS-only external URL allowlist and private diff consent blocking.
- [ ] Run targeted tests and confirm failure.
- [ ] Add reusable `validate_external_url` in Rust core.
- [ ] Use `validate_external_url` from Tauri command before opening external URLs.
- [ ] Run targeted tests and confirm pass.

### Task 3: Frontend View Models And i18n

**Files:**
- Modify: `ui/src/lib/view-models.ts`
- Create: `ui/src/lib/i18n.ts`
- Modify: `ui/src/lib/view-models.test.ts`

- [ ] Write failing Vitest tests for startup gate behavior, strict auth mode, locale detection, review language priority, catalog parity, and production sample fallback policy.
- [ ] Run `npm test -- --run ui/src/lib/view-models.test.ts` and confirm failure.
- [ ] Add `AppCapabilityView`, `LanguagePreferences`, locale helpers, and i18n catalog.
- [ ] Update sample status to GitHub-missing by default so production UI does not appear before auth.
- [ ] Run targeted Vitest test and confirm pass.

### Task 4: React Multi-Workspace UI

**Files:**
- Modify: `ui/src/App.tsx`
- Modify: `ui/src/styles.css`

- [ ] Add `activeScreen` state for `setup`, `inbox`, `workspace`, `agent`, `drafts`, and `settings`.
- [ ] Render `StartupAuthGate` when GitHub is not connected, and block Review Inbox/Workspace/Drafts.
- [ ] Render App Shell with command bar, navigation, and focused screens when GitHub is connected.
- [ ] Keep Agent Run visible but disabled when GPT/ChatGPT is missing.
- [ ] Add UI and review language selectors in Setup and Settings.
- [ ] Ensure PR row selection updates workspace, agent target, draft target, and submit target.
- [ ] Run `npm run build` and fix type errors.

### Task 5: Verification And Documentation

**Files:**
- Create: `docs/verification/reviewdesk-tauri-0.0.5.md`
- Modify: `package.json`

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo test`.
- [ ] Run `npm test`.
- [ ] Run `npm run build`.
- [ ] Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] Update verification documentation with implemented files, PASS/FAIL results, and remaining risks.
- [ ] Ensure `npm run desktop` still runs the Tauri desktop app with one command.
