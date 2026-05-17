# ReviewDesk egui UI PRD 0.0.2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `docs/prd/prd-0.0.2.md` egui UI redesign, verify behavior with tests and documented manual viewport checks, and update project documentation.

**Architecture:** Split the current `src/app.rs` shell into a small app host plus focused `src/ui/*` modules. Put testable UI behavior in pure view-state helpers, and keep egui rendering in pane modules for Top Bar, Queue, PR Workspace, Draft, Dialogs, and Bottom Status Strip.

**Tech Stack:** Rust 2024, `egui`/`eframe` 0.34, existing ReviewDesk domain models, `cargo test`, `cargo check`, `cargo fmt`.

---

## File Map

- Modify `src/lib.rs`: export `ui`.
- Replace `src/app.rs`: hold `ReviewDeskApp`, route egui panels, own `AppViewState`.
- Create `src/ui/mod.rs`: module exports.
- Create `src/ui/state.rs`: `AppViewState`, queue filters, tabs, dialog state, helper functions, sample fixtures for UI.
- Create `src/ui/theme.rs`: spacing/color helpers.
- Create `src/ui/components.rs`: badge, banner, section title, disabled helper, row helpers.
- Create `src/ui/top_bar.rs`: global auth/provider/model/depth/status controls.
- Create `src/ui/queue_pane.rs`: repository selector, filters, PR row list, direct input fallback.
- Create `src/ui/pr_workspace.rs`: PR header, summary strip, tabs, findings/files/checklist.
- Create `src/ui/draft_pane.rs`: verdict selector, editor, safety checklist, save/copy/submit actions.
- Create `src/ui/dialogs.rs`: submit confirmation and error guidance windows.
- Create `src/ui/bottom_status.rs`: last task/error strip.
- Create `tests/ui_state_tests.rs`: PRD-required view state, disabled reason, queue filter, dialog, snapshot-name tests.
- Create `docs/verification/reviewdesk-egui-ui-0.0.2.md`: manual viewport/state verification record.
- Update `docs/prd/prd-0.0.2.md`: implementation status and verification summary.

## Tasks

### Task 1: View State Tests

- [x] Add `tests/ui_state_tests.rs` covering:
  - startup state has GitHub-required controls disabled
  - AI blocked keeps manual draft possible but disables analyze
  - queue filtering finds review-requested and assigned PRs
  - direct PR input reports invalid host/pattern
  - submit disabled reason covers empty body, stale, missing GitHub, missing PR
  - submit dialog open/cancel/confirm state
  - required snapshot names are all present
- [x] Run `cargo test --test ui_state_tests` and verify RED because `reviewdesk::ui::state` does not exist.

### Task 2: UI State Model

- [x] Create `src/ui/state.rs` with PRD state structs and pure helpers.
- [x] Export `pub mod ui;` from `src/lib.rs`.
- [x] Run `cargo test --test ui_state_tests` and verify GREEN.

### Task 3: egui Modules

- [x] Create the UI render modules listed in the file map.
- [x] Implement Top Bar, Left Queue Pane, Center Workspace, Right Draft Pane, Submit Dialog, Bottom Status Strip.
- [x] Use `egui::Panel`, `egui::CentralPanel`, `egui::ScrollArea`, and `egui::Window`.
- [x] Keep findings/checklist/summary read-only by rendering labels, while draft stays editable.
- [x] Implement wide 3-pane and narrow tabbed layout switch based on available width.
- [x] Run `cargo check` and fix compile errors.

### Task 4: App Integration

- [x] Replace the 2-column shell in `src/app.rs` with `AppViewState` and the new render modules.
- [x] Add sample data so the UI demonstrates repository/PR queue, selected PR, AI blocked, findings, files, checklist, and draft flow without real runtime wiring.
- [x] Wire button interactions to local view state only: connect GitHub, refresh queue, select PR, validate direct PR, edit/save/copy draft, open/cancel/confirm submit dialog.
- [x] Run `cargo test` and `cargo check`.

### Task 5: Documentation

- [x] Create `docs/verification/reviewdesk-egui-ui-0.0.2.md` with wide/medium/narrow viewport verification and state fixture coverage.
- [x] Update `docs/prd/prd-0.0.2.md` to mark implementation and verification complete.
- [x] Run `rg` checks for stale references and placeholders.

### Task 6: Final Verification

- [x] Run `cargo fmt --check`.
- [x] Run `cargo check`.
- [x] Run `cargo test`.
- [x] Re-read `docs/prd/prd-0.0.2.md` acceptance criteria and map them to code/tests/docs.
