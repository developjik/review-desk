# ReviewDesk PRD 0.0.8 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/prd/prd-0.0.8.md` by replacing Codex env stubs with a token-safe Codex bridge, real command surface, Agent Run state, tests, and verification docs.

**Architecture:** Keep Tauri commands thin and move Codex process/RPC/JSONL parsing into `src/codex_bridge.rs`. Use testable bridge helpers and fake app-server/exec payloads for CI; do not read Codex auth cache or expose tokens. React receives only sanitized bridge/account/model/run views.

**Tech Stack:** Rust 2024, Tauri v2, Tokio process/io, Serde JSON, React 19, TypeScript, Vitest, Cargo tests.

---

## File Map

- Create `src/codex_bridge.rs`: Codex bridge types, process diagnostics, JSON-RPC/JSONL parsing, model/account/rate/run mapping, redaction-safe helpers.
- Modify `src/lib.rs`: export `codex_bridge`.
- Modify `src/app_core.rs`: add `minimal` reasoning effort and Codex blocked reasons/IPC commands.
- Modify `src/security.rs`: add Codex auth cache and app-server redaction patterns.
- Modify `src-tauri/src/commands.rs`: wire Codex bridge helpers into existing and new Tauri commands.
- Modify `src-tauri/src/main.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/main.json`: register new commands.
- Modify `ui/src/lib/ipc.ts`, `ui/src/lib/view-models.ts`, `ui/src/lib/i18n.ts`, `ui/src/App.tsx`: expose Codex bridge status, Agent Run commands, Codex-first copy.
- Add tests: `tests/codex_bridge_tests.rs`, extend `tests/app_core_tests.rs`, `tests/security_tests.rs`, `tests/tauri_shell_tests.rs`, `ui/src/lib/view-models.test.ts`.
- Add `docs/verification/reviewdesk-tauri-0.0.8.md` and update `docs/prd/prd-0.0.8.md`.

## Tasks

### Task 1: Codex Bridge Core

- [ ] Add failing tests for Codex binary/version parsing, JSON-RPC message classification, authMode mapping, hidden model filtering, minimal reasoning effort, JSONL draft extraction, and token-like field rejection.
- [ ] Implement `src/codex_bridge.rs` with sanitized view types and pure parsers.
- [ ] Run `cargo test --test codex_bridge_tests`.

### Task 2: App Core and Security Contract

- [ ] Add failing tests for new blocked reasons, `minimal` reasoning effort, new IPC commands, and Codex token/auth-cache redaction.
- [ ] Extend `src/app_core.rs` and `src/security.rs`.
- [ ] Run targeted app core/security tests.

### Task 3: Tauri Command Surface

- [ ] Add `get_codex_bridge_status`, `poll_codex_chatgpt_login`, `start_agent_run`, `read_agent_run`, and `cancel_agent_run`.
- [ ] Register commands in Tauri handler, build manifest, capability file, and app core allowlist.
- [ ] Keep `generate_review_draft` as compatibility wrapper over the same bridge/run path.
- [ ] Run `cargo test --test tauri_shell_tests` and `cargo check --manifest-path src-tauri/Cargo.toml`.

### Task 4: Frontend Codex UX

- [ ] Add Vitest coverage for Codex bridge statuses, ChatGPT OAuth wording removal, `Continue without AI`, `minimal` reasoning, and token-like rendered payload detection.
- [ ] Update IPC/view-model/i18n/App surfaces.
- [ ] Run `npm test -- --run` and `npm run build`.

### Task 5: Verification and Docs

- [ ] Run full verification: `cargo fmt --check`, `cargo test`, `npm test -- --run`, `npm run build`, `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] Create `docs/verification/reviewdesk-tauri-0.0.8.md`.
- [ ] Update `docs/prd/prd-0.0.8.md` status and implementation result.
