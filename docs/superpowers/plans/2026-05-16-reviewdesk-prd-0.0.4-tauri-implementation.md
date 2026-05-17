# ReviewDesk PRD 0.0.4 Tauri Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the PRD 0.0.4 Tauri + React shell while preserving the existing Rust core security boundary.

**Architecture:** The existing Rust crate remains the domain/core crate. A new `app_core` module exposes frontend-safe view models, command allowlist metadata, and submit preflight logic. A separate `src-tauri` crate owns Tauri IPC commands and calls the Rust crate without exposing tokens. The React frontend renders the reference-driven Review Inbox, command bar, diff workspace, agent run panel, draft composer, and submit safety dock.

**Tech Stack:** Rust, Tauri v2, React, TypeScript, Vite, Tailwind CSS, shadcn-style local components, Vitest.

---

### Task 1: Rust AppCore Boundary

**Files:**
- Create: `src/app_core.rs`
- Modify: `src/lib.rs`
- Test: `tests/app_core_tests.rs`

- [x] **Step 1: Write failing tests**

Run: `cargo test --test app_core_tests`

Expected: FAIL because `reviewdesk::app_core` does not exist.

- [x] **Step 2: Implement frontend-safe app status**

Create `AuthConnectionState`, `AppStatusView`, `frontend_safe_app_status`.

- [x] **Step 3: Implement IPC allowlist metadata**

Create `CommandRisk`, `IpcCommandSpec`, `allowed_ipc_commands`.

- [x] **Step 4: Implement submit preflight**

Create `SubmitPreflightInput`, `SubmitPreflightStatus`, `SubmitPreflightView`, `prepare_submit_review`.

- [x] **Step 5: Verify Rust AppCore**

Run: `cargo test --test app_core_tests`

Expected: PASS.

### Task 2: Tauri Shell and Command Adapter

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/main.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/commands.rs`

- [x] **Step 1: Define Tauri command allowlist**

Mirror PRD command names in `build.rs` and expose only frontend-safe payloads.

- [x] **Step 2: Implement commands**

Implement `get_app_status`, OAuth start/poll placeholders, GitHub repo/queue/context commands, AI blocked draft command, draft save, preflight, confirm submit, and allowlisted URL open.

- [x] **Step 3: Verify Tauri compile**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 3: React Product Shell

**Files:**
- Create: `package.json`
- Create: `index.html`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `ui/src/main.tsx`
- Create: `ui/src/App.tsx`
- Create: `ui/src/lib/ipc.ts`
- Create: `ui/src/lib/view-models.ts`
- Create: `ui/src/lib/view-models.test.ts`
- Create: `ui/src/styles.css`

- [x] **Step 1: Write frontend behavior tests**

Test command metadata, startup gate labels, and no token fields in app status.

- [x] **Step 2: Implement UI shell**

Render command bar, review inbox, diff workspace, agent run panel, draft composer, and submit safety dock.

- [x] **Step 3: Verify frontend**

Run: `npm install`, `npm test`, `npm run build`.

Expected: PASS.

### Task 4: Documentation and Final Verification

**Files:**
- Modify: `docs/prd/prd-0.0.4.md`
- Create: `docs/verification/reviewdesk-tauri-0.0.4.md`

- [x] **Step 1: Update PRD status**

Mark implementation artifacts and known ChatGPT OAuth limitation.

- [x] **Step 2: Write verification document**

Record Rust, frontend, Tauri, and security checks.

- [ ] **Step 3: Full verification**

Run:

```bash
cargo fmt --check
cargo check
cargo test
npm test
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS, or document any environment blocker with exact output.
