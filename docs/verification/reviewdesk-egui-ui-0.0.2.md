# ReviewDesk egui UI 0.0.2 Verification

검증일: 2026-05-16

## 범위

이 문서는 `docs/prd/prd-0.0.2.md`의 egui UI 리디자인 구현 검증 기록이다.

검증 대상:

- Top Bar
- Left Queue Pane
- Center PR Workspace
- Right Draft Pane
- Bottom Status Strip
- Submit confirmation dialog
- UI view state helper
- PRD snapshot/test id 목록

제외 대상:

- 실제 GitHub OAuth runtime wiring
- 실제 ChatGPT OAuth provider invocation
- inline comment 제출
- background polling
- web/Tauri UI

위 항목은 PRD 0.0.2의 제외 범위다.

## 구현 파일

- `src/app.rs`: app shell, wide/narrow layout routing
- `src/ui/state.rs`: view state, disabled reason, dialog, viewport helper
- `src/ui/top_bar.rs`: GitHub/AI/model/depth/status controls
- `src/ui/queue_pane.rs`: repository selector, filters, queue rows, direct PR fallback
- `src/ui/pr_workspace.rs`: PR header, summary strip, tabs, findings/files/checklist
- `src/ui/draft_pane.rs`: verdict selector, draft editor, safety checklist, submit actions
- `src/ui/dialogs.rs`: submit confirm/submitting/error windows
- `src/ui/bottom_status.rs`: task/error/status strip
- `src/ui/components.rs`: badge/banner/button helpers
- `src/ui/theme.rs`: layout constants and status colors

## Automated Verification

Commands:

```bash
$ cargo fmt --check
# pass

$ cargo check
# pass

$ cargo test
# pass: auth 1, github 11, review 4, security 4, ui_state 8
```

UI-specific tests:

- `startup_state_disables_github_dependent_actions`
- `ai_blocked_state_keeps_manual_draft_available_but_disables_analyze`
- `queue_filtering_supports_review_requested_and_assigned`
- `direct_pr_input_validation_reports_invalid_host_or_pattern`
- `submit_disabled_reason_covers_empty_stale_missing_github_and_missing_pr`
- `submit_dialog_tracks_open_cancel_and_confirm_state`
- `snapshot_names_cover_prd_required_states`
- `viewport_width_selects_wide_or_narrow_layout`

## Snapshot/Test ID Coverage

| PRD name | Coverage |
| --- | --- |
| `startup_auth_required` | `required_snapshot_names`, startup view state test |
| `github_connected_ai_blocked` | sample connected/AI blocked state test |
| `queue_empty` | required snapshot name, queue empty UI path |
| `queue_loaded_with_selection` | sample queue state and queue filter tests |
| `direct_pr_invalid` | direct PR validation test |
| `context_collecting` | required snapshot name, PR selection sets context collecting |
| `consent_required_private_repo` | required snapshot name, center consent banner path |
| `draft_ready` | required snapshot name, draft pane state path |
| `stale_submit_blocked` | submit disabled reason test |
| `submit_confirm_dialog` | dialog state test |
| `submit_failed_draft_preserved` | required snapshot name and draft error state path |
| `submitted_review_id` | required snapshot name and submitted state path |

## Viewport Verification

| Viewport | PRD expectation | Verification |
| --- | --- | --- |
| Wide `1280x760` | Top Bar + Left Queue + Center Workspace + Right Draft + Bottom Status | `viewport_mode_for_width(..., 1280.0)` returns `Wide`; app renders three panes |
| Medium `1100x760` | three-pane layout with constrained pane widths | `viewport_mode_for_width(..., 1100.0)` returns `Wide`; side panels use width ranges |
| Narrow `800x760` | tabbed `Queue / Review / Draft` layout | `viewport_mode_for_width(..., 800.0)` returns `Narrow(Queue)` and preserves selected narrow tab |

## State Verification

| State | Verification |
| --- | --- |
| GitHub auth required | queue/analyze/submit disabled via `startup_state_disables_github_dependent_actions` |
| AI blocked | analyze disabled and manual draft enabled via `ai_blocked_state_keeps_manual_draft_available_but_disables_analyze` |
| Private diff consent required | center workspace renders consent banner when required and not accepted |
| Empty queue | queue pane renders empty message when filtered list is empty |
| Stale | submit disabled reason returns `DraftStale` |
| Submit failed | draft state preserves body and stores last error path |
| Submitted | `apply_sample_submitted` sets review id and submitted status |

## Acceptance Mapping

| PRD acceptance | Implementation/verification |
| --- | --- |
| Top Bar, Left Queue, Center Workspace, Right Draft Pane | `src/app.rs` + `src/ui/*` pane modules |
| Wide layout | `render_wide`, `viewport_mode_for_width` |
| Narrow tabbed layout | `render_narrow`, `WorkspaceSection` |
| GitHub 미연결 disabled | `submit_disabled_reason`, `can_refresh_queue`, `can_select_pr` |
| AI blocked but manual draft possible | `can_edit_manual_draft`, `analyze_disabled_reason` |
| Direct PR fallback validation | `validate_direct_pr_input` |
| Submit confirmation dialog | `DialogState::SubmitConfirm`, `open_submit_dialog` |
| Stale submit blocked | `SubmitDisabledReason::DraftStale` |
| Findings structured display | `FindingView`, `pr_workspace::render` |
| Read-only result vs editable draft | workspace uses labels; draft pane uses `TextEdit::multiline` |
| Snapshot names present | `required_snapshot_names` test |

## Notes

The UI uses local fixture state for demonstrable interactions because PRD 0.0.2 explicitly excludes real GitHub OAuth runtime wiring and real ChatGPT OAuth invocation. Core GitHub/API behavior remains covered by existing non-UI tests.
