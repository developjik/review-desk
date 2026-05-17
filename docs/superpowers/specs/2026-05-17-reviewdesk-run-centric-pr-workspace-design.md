# ReviewDesk Run-Centric PR Workspace Design

Date: 2026-05-17
Status: Approved design
Scope: Product redesign for assigned PR review workflow

## 1. Summary

ReviewDesk should be redesigned around a run-centric PR workspace.

The user wants to handle PRs assigned to them faster through analysis, human review, AI review, editable review drafts, inline comments, and explicit GitHub review publishing. The product should automate analysis and draft preparation where useful, but publishing to GitHub must always remain a deliberate user action.

The chosen direction is:

Assigned PR list -> Selected PR workspace -> Multiple analysis runs -> Editable review draft -> Inline comment verification -> Explicit GitHub publish

This keeps the PR as the main work object while allowing the same PR to be analyzed multiple times with different modes, models, settings, and custom prompts.

## 2. Goals

- Make repeated review of assigned GitHub PRs faster and easier.
- Support automatic and manual analysis.
- Allow the same PR to have multiple analysis runs.
- Support different run modes, model/settings choices, and custom prompts.
- Treat AI output as draft material, not final review content.
- Let users edit top-level review bodies and inline comments before publishing.
- Support publishing both top-level review body and inline GitHub review comments.
- Block unsafe publishing when PR state, diff state, permissions, or inline line mappings are invalid.
- Preserve enough provenance to understand which run produced which draft material.

## 3. Non-Goals

- No automatic GitHub publishing.
- No generic AI agent orchestration product.
- No branch code editing workflow in the first implementation pass.
- No auto-merge or auto-approve flow.
- No draft-first product structure where AI-generated drafts replace PR context as the primary navigation object.

## 4. Direction Considered

Three product directions were compared.

### 4.1 Linear Pipeline

Queue -> Analyze -> Review -> Publish.

This is easy to understand and useful as a happy path, but it is too rigid as the core architecture. PR review is iterative: users may rerun analysis, change prompts, compare model outputs, discard noisy findings, and publish only part of a draft.

### 4.2 Run-Centric PR Workspace

The assigned PR is the workspace. Each PR owns multiple analysis runs, draft material, inline comment candidates, stale state, and publish attempts.

This direction best fits the requirements because it models both core objects:

- PR: the user's work item.
- Analysis run: the automation unit that can repeat under different settings.

### 4.3 Draft Inbox First

Generated review drafts become the main inbox.

This is fast for approving prepared text, but it over-centers AI output and risks users reviewing AI prose instead of the PR and evidence. A "drafts needing attention" view is deferred to a post-v1 secondary surface and should not be the primary structure.

## 5. Product Structure

The main app has three top-level areas:

- Assigned PR Queue
- PR Workspace
- Settings

Drafts, AI runs, and publish safety are not separate top-level destinations. They live inside the selected PR workspace so review context does not break.

### 5.1 Object Hierarchy

#### AssignedPR

Represents one PR assigned to the user.

Owns:

- GitHub owner, repo, and PR number
- title, author, URL, review reason
- current head SHA
- current diff hash
- queue state
- latest run id
- active draft id
- publish/review status

#### AnalysisRun

Represents one execution of analysis for one PR.

Every execution gets a unique run id. A run id must not be derived only from PR and diff hash because the same PR and same diff can have multiple valid runs with different modes, models, prompts, or settings.

Records:

- run id
- PR reference
- head SHA
- diff hash
- context hash
- run mode
- model and reasoning settings
- review language
- private diff consent snapshot
- custom prompt or prompt summary
- included files
- excluded files
- findings
- generated draft seed
- status
- timestamps

#### ReviewDraft

Represents the user-editable review artifact.

The draft may be seeded by one or more analysis runs, but the saved draft is user-owned editable work.

Contains:

- draft id
- source run ids
- base head SHA
- base diff hash
- verdict
- top-level review body
- inline comment drafts
- edit state
- stale state

#### InlineCommentDraft

Represents one candidate inline GitHub review comment.

Contains:

- path
- side
- line or range
- body
- severity
- confidence
- source finding id, if available
- source run id, if available
- selected for publish state
- dismissed state
- generated/user-edited marker
- current diff mapping status
- stale state

#### PublishAttempt

Represents one attempt to submit a review to GitHub.

Records:

- attempt id
- draft id
- GitHub account
- preflight id
- submitted head SHA
- submitted verdict
- submitted body
- submitted inline comment payload
- result
- GitHub review id on success
- error and retry state on failure

## 6. Core Workflow

The default workflow is:

1. Sync assigned PRs from GitHub.
2. Run automatic lightweight triage for eligible PRs.
3. User selects a PR.
4. Workspace loads PR context, diff, run history, and draft state.
5. User starts or reviews analysis runs.
6. User assembles or edits a review draft.
7. User verifies inline comments against the diff.
8. User opens publish preview.
9. App runs preflight.
10. User explicitly confirms publish.
11. App submits review to GitHub and records the publish attempt.

### 6.1 Automatic Analysis

On app open, eligible assigned PRs may receive a lightweight background triage run. This run prepares:

- PR summary
- risk level
- likely review effort
- changed-file guide
- obvious blockers

Automatic analysis can prepare draft material, but it must not publish anything.

### 6.2 Manual Analysis

Inside a PR workspace, the user can start additional runs.

Required run types:

- Fast
- Deep
- Security
- Tests
- Custom prompt

Manual runs can vary:

- mode
- model
- reasoning depth
- prompt
- language
- file scope
- private diff consent snapshot

### 6.3 Draft Creation

Analysis results are draft material, not final output.

The user can:

- use one run as a draft source
- select findings from multiple runs
- edit the review body
- choose verdict
- edit, delete, add, or deselect inline comments
- save a draft without publishing

### 6.4 Publishing

Publishing is always explicit.

Before submission, the confirmation view must show:

- GitHub account
- repo and PR number
- target head SHA
- verdict
- exact top-level review body
- exact inline comments that will be sent
- inline comment count
- blockers and warnings

The confirmation view must use the same payload shape that submission uses.

## 7. UX Design

The main screen uses three zones.

### 7.1 Assigned PR Queue

Purpose: choose the next PR and understand review state quickly.

Shows:

- repo and PR number
- title and author
- review reason
- CI state
- latest run state
- draft state
- stale/blocked state
- inline comment count

Queue sections:

- Needs review
- Draft ready
- Running
- Blocked or stale
- Published or recently reviewed

### 7.2 PR Evidence Workspace

Purpose: build trust in the review by keeping PR evidence visible.

Shows:

- selected PR header
- current head SHA and freshness
- CI/status summary
- changed file list
- risk indicators
- run shelf and run history
- findings grouped by severity, file, or theme
- diff reader with line numbers
- inline draft anchors

Run shelf supports:

- quick run buttons
- custom run drawer
- run history
- compare/select as draft source

### 7.3 Draft & Publish Panel

Purpose: edit exactly what will be published.

Contains:

- Body tab
- Inline tab
- Safety tab

Body tab:

- editable top-level body
- verdict selector
- source run marker
- stale state

Inline tab:

- selected inline comments
- edit/delete/dismiss controls
- jump-to-diff action
- mapping status
- selected-for-publish state

Safety tab:

- current preflight status
- blockers
- warnings
- recovery actions
- publish preview entry point

## 8. Architecture

The current architecture should evolve rather than be replaced.

Existing boundaries to preserve:

- GitHub auth
- assigned/requested PR queue loading
- PR context collection
- AI/Codex bridge
- submit preflight
- GitHub top-level review submit

New boundaries to add:

- durable analysis run management
- durable draft management
- inline comment mapping validation
- inline GitHub review comment payload support
- publish attempt persistence

### 8.1 Frontend Modules

The frontend should be split into focused modules:

- Assigned PR queue
- PR workspace
- Run shelf/history
- Diff reader
- Draft editor
- Inline verifier
- Publish confirmation
- Safety state view model

Run, draft, and publish state should not remain as one global draft string inside a single large screen component.

### 8.2 Tauri Command Surface

Run management:

- list_analysis_runs
- start_analysis_run
- read_analysis_run
- cancel_analysis_run
- archive_analysis_run

Draft management:

- create_draft_from_run
- save_review_draft
- read_review_draft
- list_review_drafts
- mark_active_draft

Inline validation:

- validate_inline_comments

Publish:

- prepare_submit_review with body and selected inline comments
- confirm_submit_review with the exact confirmed payload

### 8.3 Persistence

Local storage should group artifacts per PR workspace:

- snapshots
- analysis runs
- drafts
- publish attempts

Runs and publish attempts are append-first. Drafts are mutable, but preserve source run ids, base head SHA, base diff hash, edit metadata, and stale state.

Tokens remain in keychain. Local artifacts must remain path-safe, owner-only, and secret-safe.

## 9. Safety and Error Handling

### 9.1 Publish Blockers

Publishing must be blocked when:

- current PR head SHA differs from the draft/run base
- current diff hash differs from the draft/run base
- selected inline comments have unresolved path, side, line, or range mapping
- GitHub auth is missing
- required repo scope is missing
- SSO or organization approval is required
- GitHub account does not match the prepared publish context
- PR is closed or merged
- review body is empty and no inline comments are selected

### 9.2 Recoverable Errors

Queue load failure:

- show retry
- show auth refresh action
- preserve access to local drafts when possible

Analysis run failure:

- persist failed run
- show failure reason
- allow retry with same settings
- allow retry with adjusted settings

Draft conflict:

- never overwrite user edits with late AI output
- offer merge or create another draft

Submit failure:

- record publish attempt
- preserve draft
- show GitHub error reason
- allow retry after fresh preflight

### 9.3 Trust Rules

- Findings and inline comments should link back to source file, line, and run when available.
- Manual edits win over generated output.
- Generated output remains visibly draft material.
- The publish preview must match the actual submission payload.
- Runs and publish attempts remain inspectable after success or failure.

## 10. Test Strategy

Domain tests:

- unique run ids for repeated analysis
- run snapshot/provenance recording
- draft source run links
- stale state rules
- inline mapping status
- publish blocker evaluation

GitHub tests:

- review body plus inline comments payload
- stale head rejection
- permission and scope error classification
- closed/merged PR handling
- submit failure preservation

Tauri command tests:

- list/read/start analysis runs
- save/read review drafts
- validate inline comments
- prepare submit blockers
- confirm submit exact payload
- publish attempt persistence

Frontend tests:

- PR switching preserves drafts
- multiple runs display without overwriting
- draft editing does not get overwritten by late generation
- stale warning is visible
- inline comment edit/delete/select works
- invalid inline mapping blocks publish
- publish confirmation displays exact payload

## 11. First Implementation Slice

The safest implementation order is:

1. Add durable run identity and run persistence.
2. Add per-PR draft persistence and source run links.
3. Refactor frontend state around selected PR, runs, and draft models.
4. Add inline comment draft model and UI editing.
5. Add inline mapping validation.
6. Extend submit preflight and confirm submit for inline comments.
7. Add exact publish preview.
8. Add polish for queue state, run history, and safety states.

The first slice should prove that repeated analysis of the same PR no longer overwrites previous results and that a selected draft survives PR switching.
