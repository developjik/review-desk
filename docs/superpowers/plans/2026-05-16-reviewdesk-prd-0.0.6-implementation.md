# ReviewDesk PRD 0.0.6 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/prd/prd-0.0.6.md` by replacing the vague ChatGPT token gate with a Codex ChatGPT managed-auth capability contract, wiring Agent Run to the backend command, and updating verification documentation.

**Architecture:** Keep GitHub as the App Shell hard gate and model Codex ChatGPT as an AI-only capability gate. Rust remains the source of truth for account/model/rate-limit/generation state; React renders only redacted status, login metadata, model capability, and blocked reasons.

**Tech Stack:** Rust core crate, Tauri commands, React + TypeScript + Vite, Vitest, Cargo tests.

---

## File Map

- Modify `src/app_core.rs`: add Codex ChatGPT status, model capability, rate-limit snapshot, blocked reason, run metadata, URL allowlist, and IPC allowlist entries.
- Modify `src/domain.rs`: add `xhigh` reasoning support for provider-facing model settings.
- Modify `src-tauri/src/commands.rs`: add Codex ChatGPT commands, status/model commands, rate-limit/logout/refresh commands, and make `generate_review_draft` return honest run metadata.
- Modify `src-tauri/src/main.rs`: register new Tauri commands.
- Modify `src-tauri/build.rs`: autogenerate permissions for new commands.
- Modify `ui/src/lib/view-models.ts`: mirror Rust status/model/run types and helpers.
- Modify `ui/src/lib/ipc.ts`: expose new commands and typed draft generation.
- Modify `ui/src/App.tsx`: render Codex connection details, model capability picker, device-code fallback, and call `generate_review_draft`.
- Modify `ui/src/lib/i18n.ts`: update English/Korean copy for Codex ChatGPT managed auth.
- Modify `tests/app_core_tests.rs`: add Rust contract tests.
- Modify `ui/src/lib/view-models.test.ts`: add frontend view-model tests.
- Modify `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`: bump visible version to `0.0.6`.
- Create `docs/verification/reviewdesk-tauri-0.0.6.md`: record implementation scope, verification commands, and remaining risks.

## Tasks

### Task 1: Rust Contract Tests

- [ ] Add tests proving:
  - GitHub connected + Codex missing keeps Inbox/Diff/Manual Draft/Submit enabled and blocks only Agent Run.
  - Strict Auth Mode blocks App Shell until Codex is connected.
  - App status serializes no token-like fields.
  - URL allowlist accepts `https://auth.openai.com/...`.
  - IPC allowlist contains new Codex commands and exposes no credentials.
  - Model capability blocks unavailable models and unsupported reasoning effort.

Run: `cargo test --test app_core_tests`

Expected before implementation: compile/test failures for missing 0.0.6 types and commands.

### Task 2: Rust Core Implementation

- [ ] Add `AiConnectionStatusView`, `AiAccountView`, `AiModelView`, `AiRateLimitSnapshot`, `AiBlockedReason`, `ReasoningEffort`, and `AgentRunRecordView`.
- [ ] Add `frontend_safe_app_status_from_context` and keep existing wrappers compatible.
- [ ] Add model/reasoning validation helpers.
- [ ] Update external URL allowlist for Codex ChatGPT auth URLs.
- [ ] Update IPC allowlist with the new commands.

Run: `cargo test --test app_core_tests`

Expected after implementation: Rust app core tests pass.

### Task 3: Tauri Command Contract

- [ ] Add `get_ai_connection_status`, `start_codex_chatgpt_login`, `cancel_codex_chatgpt_login`, `read_codex_account`, `list_ai_models`, `select_ai_model`, `read_codex_rate_limits`, `logout_codex_chatgpt`, and `refresh_ai_account_status`.
- [ ] Keep legacy `start_chatgpt_oauth` and `poll_chatgpt_oauth` as compatibility wrappers that do not return tokens.
- [ ] Update `generate_review_draft` to validate Codex status, model, reasoning effort, and private diff consent before creating a run record.
- [ ] Register commands in Tauri main/build files.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected after implementation: Tauri shell compiles.

### Task 4: Frontend Tests

- [ ] Add tests proving:
  - Codex blocked reasons are rendered as command disabled reasons.
  - Available models exclude hidden entries and disable unavailable entries.
  - Reasoning effort options come from the selected model.
  - Token-like fields are still detected before rendering raw payloads.
  - English/Korean i18n catalogs remain in parity.

Run: `npm test -- --run`

Expected before frontend implementation: failing tests for missing helpers/types.

### Task 5: Frontend Implementation

- [ ] Update TypeScript types to mirror Rust.
- [ ] Expose new IPC functions.
- [ ] Replace GPT wording with Codex ChatGPT where the product is describing managed auth.
- [ ] Add browser/device connection actions.
- [ ] Render account, plan, model list sync, rate-limit status, and blocked reason in Settings.
- [ ] Make Agent Run call `generate_review_draft`; blocked adapter results must show `generation_adapter_unavailable`, not `queued`.
- [ ] Keep manual draft and submit usable when Codex is missing.

Run: `npm test -- --run && npm run build`

Expected after implementation: frontend tests and build pass.

### Task 6: Documentation And Final Verification

- [ ] Create `docs/verification/reviewdesk-tauri-0.0.6.md`.
- [ ] Update version metadata to `0.0.6`.
- [ ] Run the full verification set:
  - `cargo fmt --check`
  - `cargo test`
  - `npm test -- --run`
  - `npm run build`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] Search for token-like leaks in status fixtures and docs where relevant.

Expected after verification: all commands exit 0, and the verification document states remaining risks without claiming real Codex OAuth success when the App Server is not configured.
