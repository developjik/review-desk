# ReviewDesk UI 0.0.3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/prd/prd-0.0.3.md` by modernizing the Rust `egui` UI system, preserving review safety, and updating verification docs.

**Architecture:** Keep the existing `eframe` shell and `AppViewState` model. Add view-state tests first, then upgrade `theme.rs`, `components.rs`, and pane renderers to use reusable styled components and stable PR queue rows.

**Tech Stack:** Rust 2024, `eframe`/`egui` 0.34.2, `egui_extras`, cargo tests, markdown verification docs.

---

### Task 1: View-State Safety Tests

**Files:**
- Modify: `tests/ui_state_tests.rs`
- Modify: `src/ui/state.rs`

- [x] Add failing tests for 0.0.3 snapshot names, explicit `APPROVE`/`REQUEST_CHANGES` selection, private diff consent blocking for AI-generated drafts, SSO submit blocking, and submit failure draft preservation.
- [x] Run `cargo test --test ui_state_tests` and verify the new tests fail.
- [x] Implement the minimal state changes in `src/ui/state.rs`.
- [x] Re-run `cargo test --test ui_state_tests` and verify it passes.

### Task 2: UI System Components

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/ui/theme.rs`
- Modify: `src/ui/components.rs`

- [x] Add `egui_extras` dependency for stable queue/table support.
- [x] Expand `theme.rs` with design tokens for colors, spacing, typography, surfaces, strokes, and button variants.
- [x] Replace generic badge/button helpers with reusable status badges, banners, button variants, queue/finding/file/checklist rows, empty state, skeleton row, and sticky action bar helpers.

### Task 3: Pane Modernization

**Files:**
- Modify: `src/ui/top_bar.rs`
- Modify: `src/ui/queue_pane.rs`
- Modify: `src/ui/pr_workspace.rs`
- Modify: `src/ui/draft_pane.rs`
- Modify: `src/ui/dialogs.rs`
- Modify: `src/ui/bottom_status.rs`

- [x] Rework Top Bar priority and button hierarchy.
- [x] Rework Queue into stable rows/table and demote Direct PR fallback to secondary UI.
- [x] Rework Workspace header, summary strip, banners, findings, files, and checklist rows.
- [x] Rework Draft Pane with explicit verdict selection, structured safety checklist, and sticky action bar.
- [x] Rework Submit dialog into a structured final confirmation surface.

### Task 4: Documentation and Verification

**Files:**
- Modify: `docs/prd/prd-0.0.3.md`
- Create: `docs/verification/reviewdesk-ui-0.0.3.md`

- [x] Add implementation status to the PRD.
- [x] Document automated verification, visual QA cases, known limits, and exact commands.
- [x] Run `cargo fmt --check`, `cargo check`, and `cargo test`.
