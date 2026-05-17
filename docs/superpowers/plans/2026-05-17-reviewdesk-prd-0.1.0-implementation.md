# ReviewDesk PRD 0.1.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the PRD 0.1.0 PR-centric Review Shell so ReviewDesk opens to a prioritized Inbox and keeps PR overview, diff, AI, draft, and submit safety in one workspace.

**Architecture:** Keep Rust/Tauri IPC contracts unchanged. Add pure frontend workflow helpers for queue sections, risk badges, diff parsing, review target state, and submit safety derivation, then rebuild the React shell around `Inbox Rail + PR Workspace + Review Rail`. Leave P1/P2 backend-affecting items documented as future because PRD 0.1.0 marks full inline comments/suggestions and auth/submit protocol changes as non-goals.

**Tech Stack:** React 19, TypeScript, Vite/Vitest, Tauri IPC wrappers, Tailwind CSS, existing Rust core tests.

---

## File Structure

- Modify `ui/src/lib/view-models.ts`: add PRD 0.1.0 view-model helpers and exported types.
- Modify `ui/src/lib/view-models.test.ts`: add RED/GREEN coverage for queue sections, risk badges, diff parser, draft/preflight invalidation, and command copy.
- Add `ui/src/App.test.tsx`: render-level tests for startup gate and connected app shell.
- Modify `ui/src/App.tsx`: replace split top-level Agent/Drafts workflow with PR-centric shell and review rail.
- Modify `ui/src/lib/i18n.ts`: add English/Korean keys for new visible UI.
- Modify `ui/src/styles.css`: remove hard 1024px body minimum and add stable app overflow defaults.
- Modify `docs/prd/prd-0.1.0.md`: update status and implementation notes after completion.
- Add `docs/verification/reviewdesk-tauri-0.1.0.md`: record verification commands and manual checks.

## Task 1: View-Model Foundation

**Files:**
- Modify: `ui/src/lib/view-models.ts`
- Modify: `ui/src/lib/view-models.test.ts`

- [ ] **Step 1: Write failing tests for queue sections and risk badges**

Add tests to `ui/src/lib/view-models.test.ts`:

```ts
import {
  groupQueueBySection,
  prRiskBadges,
  reviewSearchPlaceholder,
  type PullRequestQueueItem,
} from "./view-models";

it("groups queue items into PRD 0.1.0 inbox sections", () => {
  const items: PullRequestQueueItem[] = [
    queueItem({ number: 1, review_requested: true, assigned: false, ci_status: "pass", stale_status: "fresh" }),
    queueItem({ number: 2, review_requested: true, assigned: true, ci_status: "fail", stale_status: "fresh" }),
    queueItem({ number: 3, review_requested: false, assigned: true, ci_status: "pass", stale_status: "head_changed" }),
  ];

  const sections = groupQueueBySection(items, {
    draftRefs: new Set(["company/payment-web#1"]),
    submittedRefs: new Set(["company/payment-web#3"]),
  });

  expect(sections.map((section) => [section.id, section.items.map((item) => item.number)])).toEqual([
    ["needs_review", [1, 2, 3]],
    ["blocked", [2, 3]],
    ["drafts", [1]],
    ["recently_reviewed", [3]],
  ]);
});

it("derives review risk badges without backend changes", () => {
  expect(
    prRiskBadges(queueItem({ ci_status: "fail", changed_files_count: 26, stale_status: "head_changed" })),
  ).toEqual([
    { id: "ci_failed", label: "CI failed", tone: "red" },
    { id: "stale", label: "Head changed", tone: "amber" },
    { id: "large_pr", label: "26 files", tone: "amber" },
  ]);
});

it("uses honest search copy until a true command palette exists", () => {
  expect(reviewSearchPlaceholder("en")).toBe("Search PRs, repos, titles, or files");
  expect(reviewSearchPlaceholder("ko")).toBe("PR, repo, 제목, 파일 검색");
});

function queueItem(overrides: Partial<PullRequestQueueItem>): PullRequestQueueItem {
  return {
    owner: "company",
    repo: "payment-web",
    number: 582,
    title: "Payment refactor",
    author: "alice",
    url: "https://github.com/company/payment-web/pull/582",
    draft: false,
    review_requested: true,
    assigned: false,
    ci_status: "pass",
    changed_files_count: 4,
    updated_at: null,
    review_reason: null,
    stale_status: "fresh",
    ...overrides,
  };
}
```

- [ ] **Step 2: Run tests and verify RED**

Run: `npm test -- ui/src/lib/view-models.test.ts`

Expected: FAIL with missing exports `groupQueueBySection`, `prRiskBadges`, or `reviewSearchPlaceholder`.

- [ ] **Step 3: Implement queue section and risk helpers**

Add to `ui/src/lib/view-models.ts`:

```ts
export type InboxSectionId = "needs_review" | "blocked" | "drafts" | "recently_reviewed";
export type BadgeTone = "neutral" | "green" | "amber" | "red" | "blue";

export interface InboxSection {
  id: InboxSectionId;
  label: string;
  description: string;
  items: PullRequestQueueItem[];
}

export interface QueueGroupingState {
  draftRefs: Set<string>;
  submittedRefs: Set<string>;
}

export interface RiskBadge {
  id: string;
  label: string;
  tone: BadgeTone;
}

export function pullRequestRef(item: Pick<PullRequestQueueItem, "owner" | "repo" | "number">): string {
  return `${item.owner}/${item.repo}#${item.number}`;
}

export function groupQueueBySection(
  queue: PullRequestQueueItem[],
  state: QueueGroupingState = { draftRefs: new Set(), submittedRefs: new Set() },
): InboxSection[] {
  const sections: InboxSection[] = [
    {
      id: "needs_review",
      label: "Needs review",
      description: "Open pull requests requesting your attention.",
      items: queue.filter((item) => item.review_requested || item.assigned),
    },
    {
      id: "blocked",
      label: "Blocked",
      description: "Reviews with failing CI, stale context, or missing patches.",
      items: queue.filter((item) => prRiskBadges(item).some((badge) => badge.tone === "red" || badge.id === "stale")),
    },
    {
      id: "drafts",
      label: "Drafts",
      description: "Pull requests with local or AI review draft work.",
      items: queue.filter((item) => state.draftRefs.has(pullRequestRef(item))),
    },
    {
      id: "recently_reviewed",
      label: "Recently reviewed",
      description: "Reviews submitted in this session.",
      items: queue.filter((item) => state.submittedRefs.has(pullRequestRef(item))),
    },
  ];
  return sections;
}

export function prRiskBadges(item: PullRequestQueueItem): RiskBadge[] {
  const badges: RiskBadge[] = [];
  if (item.ci_status === "fail") badges.push({ id: "ci_failed", label: "CI failed", tone: "red" });
  if (item.stale_status && item.stale_status !== "fresh") {
    badges.push({ id: "stale", label: staleStatusLabel(item.stale_status), tone: "amber" });
  }
  if ((item.changed_files_count ?? 0) >= 20) {
    badges.push({ id: "large_pr", label: `${item.changed_files_count} files`, tone: "amber" });
  }
  if (item.draft) badges.push({ id: "draft_pr", label: "Draft PR", tone: "blue" });
  return badges;
}

export function staleStatusLabel(status: string): string {
  return status
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function reviewSearchPlaceholder(locale: Locale): string {
  return locale === "ko" ? "PR, repo, 제목, 파일 검색" : "Search PRs, repos, titles, or files";
}
```

- [ ] **Step 4: Run tests and verify GREEN**

Run: `npm test -- ui/src/lib/view-models.test.ts`

Expected: PASS.

## Task 2: Diff, Draft, and Safety Helpers

**Files:**
- Modify: `ui/src/lib/view-models.ts`
- Modify: `ui/src/lib/view-models.test.ts`

- [ ] **Step 1: Write failing tests for diff parser and safety derivation**

Add tests:

```ts
import {
  deriveSubmitSafetyState,
  parseUnifiedPatch,
  type SubmitSafetyInput,
} from "./view-models";

it("parses unified patches into hunks with line numbers and line types", () => {
  const hunks = parseUnifiedPatch("@@ -70,2 +70,3 @@ export async function request(input) {\n-  return fetch(input)\n+  await refreshSession()\n+  return fetch(input)\n }");

  expect(hunks).toEqual([
    {
      header: "@@ -70,2 +70,3 @@ export async function request(input) {",
      oldStart: 70,
      newStart: 70,
      lines: [
        { type: "delete", oldLine: 70, newLine: null, content: "  return fetch(input)" },
        { type: "add", oldLine: null, newLine: 70, content: "  await refreshSession()" },
        { type: "add", oldLine: null, newLine: 71, content: "  return fetch(input)" },
        { type: "context", oldLine: 71, newLine: 72, content: "}" },
      ],
    },
  ]);
});

it("derives submit safety state from draft and preflight", () => {
  const base: SubmitSafetyInput = {
    hasTarget: true,
    draftBody: "Looks good.",
    draftDirtySincePreflight: true,
    preflight: null,
    submitting: false,
    submittedReviewId: null,
    submitMessage: null,
  };

  expect(deriveSubmitSafetyState(base).status).toBe("stale");
  expect(deriveSubmitSafetyState({ ...base, draftBody: "" }).status).toBe("not_ready");
  expect(
    deriveSubmitSafetyState({
      ...base,
      draftDirtySincePreflight: false,
      preflight: { status: "ready", blocked_reasons: [], confirmation_id: "abc" },
    }).status,
  ).toBe("ready");
});
```

- [ ] **Step 2: Run tests and verify RED**

Run: `npm test -- ui/src/lib/view-models.test.ts`

Expected: FAIL with missing exports `parseUnifiedPatch` and `deriveSubmitSafetyState`.

- [ ] **Step 3: Implement diff parser and submit safety helpers**

Add to `ui/src/lib/view-models.ts`:

```ts
export type DiffLineType = "context" | "add" | "delete";

export interface DiffLine {
  type: DiffLineType;
  oldLine: number | null;
  newLine: number | null;
  content: string;
}

export interface DiffHunk {
  header: string;
  oldStart: number;
  newStart: number;
  lines: DiffLine[];
}

export function parseUnifiedPatch(patch: string | null): DiffHunk[] {
  if (!patch) return [];
  const hunks: DiffHunk[] = [];
  let current: DiffHunk | null = null;
  let oldLine = 0;
  let newLine = 0;

  for (const rawLine of patch.split("\n")) {
    const header = rawLine.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
    if (header) {
      oldLine = Number(header[1]);
      newLine = Number(header[2]);
      current = { header: rawLine, oldStart: oldLine, newStart: newLine, lines: [] };
      hunks.push(current);
      continue;
    }
    if (!current) continue;
    if (rawLine.startsWith("+")) {
      current.lines.push({ type: "add", oldLine: null, newLine, content: rawLine.slice(1) });
      newLine += 1;
    } else if (rawLine.startsWith("-")) {
      current.lines.push({ type: "delete", oldLine, newLine: null, content: rawLine.slice(1) });
      oldLine += 1;
    } else {
      current.lines.push({ type: "context", oldLine, newLine, content: rawLine.startsWith(" ") ? rawLine.slice(1) : rawLine });
      oldLine += 1;
      newLine += 1;
    }
  }

  return hunks;
}

export interface SubmitSafetyInput {
  hasTarget: boolean;
  draftBody: string;
  draftDirtySincePreflight: boolean;
  preflight: { status: "ready" | "blocked"; blocked_reasons: string[]; confirmation_id: string | null } | null;
  submitting: boolean;
  submittedReviewId: number | null;
  submitMessage: string | null;
}

export type SubmitSafetyStatus = "not_ready" | "stale" | "checking" | "blocked" | "ready" | "submitting" | "submitted" | "failed";

export interface SubmitSafetyState {
  status: SubmitSafetyStatus;
  label: string;
  blockers: string[];
  canPrepare: boolean;
  canConfirm: boolean;
}

export function deriveSubmitSafetyState(input: SubmitSafetyInput): SubmitSafetyState {
  if (input.submittedReviewId) {
    return { status: "submitted", label: `Submitted review #${input.submittedReviewId}`, blockers: [], canPrepare: false, canConfirm: false };
  }
  if (input.submitting) {
    return { status: "submitting", label: "Submitting review", blockers: [], canPrepare: false, canConfirm: false };
  }
  if (!input.hasTarget || !input.draftBody.trim()) {
    return { status: "not_ready", label: "Select a PR and write a draft", blockers: ["body_empty"], canPrepare: input.hasTarget, canConfirm: false };
  }
  if (input.draftDirtySincePreflight || !input.preflight) {
    return { status: "stale", label: "Prepare submit to refresh safety checks", blockers: [], canPrepare: true, canConfirm: false };
  }
  if (input.preflight.status === "blocked") {
    return { status: "blocked", label: "Submit blocked", blockers: input.preflight.blocked_reasons, canPrepare: true, canConfirm: false };
  }
  return { status: "ready", label: "Ready to submit", blockers: [], canPrepare: true, canConfirm: Boolean(input.preflight.confirmation_id) };
}
```

- [ ] **Step 4: Run tests and verify GREEN**

Run: `npm test -- ui/src/lib/view-models.test.ts`

Expected: PASS.

## Task 3: Render-Level Startup and Shell Tests

**Files:**
- Add: `ui/src/App.test.tsx`
- Modify: `ui/src/App.tsx`

- [ ] **Step 1: Write failing render tests**

Create `ui/src/App.test.tsx`:

```tsx
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { sampleConnectedStatus, sampleContext, sampleQueue, sampleRepositories, sampleStatus } from "./lib/view-models";

vi.mock("./lib/ipc", async () => {
  const models = await import("./lib/view-models");
  return {
    getAppStatus: vi.fn(),
    getCodexBridgeStatus: vi.fn().mockResolvedValue(null),
    listAiModels: vi.fn().mockResolvedValue({ status: "blocked", models: [], disabled_reason: "model_list_unavailable" }),
    listRepositories: vi.fn(),
    loadReviewQueue: vi.fn(),
    collectPrContext: vi.fn(),
    openExternalUrl: vi.fn(),
    pollCodexChatGptLogin: vi.fn(),
    pollGithubOAuth: vi.fn(),
    prepareSubmitReview: vi.fn().mockResolvedValue({ status: "blocked", blocked_reasons: ["body_empty"], confirmation_id: null }),
    confirmSubmitReview: vi.fn(),
    refreshAiAccountStatus: vi.fn(),
    refreshGithubAuthStatus: vi.fn(),
    startAgentRun: vi.fn(),
    startCodexChatGptLogin: vi.fn(),
    startGithubOAuth: vi.fn(),
    sampleContext: models.sampleContext,
  };
});

const ipc = await import("./lib/ipc");

describe("ReviewDesk app shell", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows setup gate when GitHub is missing", async () => {
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleStatus());

    render(<App />);

    expect(await screen.findByText("Connect your review workspace")).toBeInTheDocument();
    expect(screen.getByText("Connect GitHub in Browser")).toBeInTheDocument();
  });

  it("shows PR-centric review shell when GitHub is connected", async () => {
    const queue = sampleQueue();
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));

    render(<App />);

    expect(await screen.findByText("Needs review")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("Payment refactor with retry-safe session handling")).toBeInTheDocument());
    expect(await screen.findByText("Review rail")).toBeInTheDocument();
    expect(screen.queryByText("Agent Runs")).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run tests and verify RED**

Run: `npm test -- ui/src/App.test.tsx`

Expected: FAIL because the app still renders old shell/nav and matcher setup may need `@testing-library/jest-dom`.

- [ ] **Step 3: Adjust test matchers if needed**

If `toBeInTheDocument` is unavailable, replace assertions with `expect(element).toBeTruthy()` instead of adding a new dependency.

- [ ] **Step 4: Implement shell changes in later tasks until tests pass**

Expected after Task 5: PASS.

## Task 4: i18n and CSS Foundation

**Files:**
- Modify: `ui/src/lib/i18n.ts`
- Modify: `ui/src/styles.css`

- [ ] **Step 1: Add i18n keys**

Add English/Korean keys for:

```ts
"shell.searchPlaceholder": "Search PRs, repos, titles, or files"
"shell.reviewRail": "Review rail"
"shell.noTarget": "Select a PR from the inbox."
"inbox.section.needs_review": "Needs review"
"inbox.section.blocked": "Blocked"
"inbox.section.drafts": "Drafts"
"inbox.section.recently_reviewed": "Recently reviewed"
"workspace.overview": "Overview"
"workspace.fileViewed": "Viewed"
"workspace.markViewed": "Mark viewed"
"rail.summary": "Summary"
"rail.ai": "AI"
"rail.draft": "Draft"
"rail.safety": "Safety"
"rail.activity": "Activity"
"safety.stale": "Prepare submit to refresh safety checks"
```

Korean equivalents:

```ts
"shell.searchPlaceholder": "PR, repo, 제목, 파일 검색"
"shell.reviewRail": "리뷰 레일"
"shell.noTarget": "인박스에서 PR을 선택하세요."
"inbox.section.needs_review": "리뷰 필요"
"inbox.section.blocked": "차단됨"
"inbox.section.drafts": "초안"
"inbox.section.recently_reviewed": "최근 리뷰"
"workspace.overview": "개요"
"workspace.fileViewed": "확인함"
"workspace.markViewed": "확인 처리"
"rail.summary": "요약"
"rail.ai": "AI"
"rail.draft": "초안"
"rail.safety": "안전"
"rail.activity": "활동"
"safety.stale": "제출 준비를 실행해 안전 검사를 갱신하세요"
```

- [ ] **Step 2: Update required key inventory**

Add the new P0 keys to `requiredI18nKeys()` so parity tests guard them.

- [ ] **Step 3: Remove hard minimum width**

Change `ui/src/styles.css`:

```css
body {
  margin: 0;
  min-width: 800px;
  background: #09090b;
}
```

- [ ] **Step 4: Run i18n tests**

Run: `npm test -- ui/src/lib/view-models.test.ts`

Expected: PASS.

## Task 5: PR-Centric App Shell

**Files:**
- Modify: `ui/src/App.tsx`

- [ ] **Step 1: Add shell state**

In `App.tsx`, replace `screens = ["inbox", "workspace", "agent", "drafts", "settings"]` with app-level screens:

```ts
const appScreens = ["inbox", "settings"] as const;
type Screen = "setup" | (typeof appScreens)[number];
type RailPanel = "summary" | "ai" | "draft" | "safety" | "activity";
type LoadState = "idle" | "loading" | "ready" | "error";
```

Add state:

```ts
const [queueState, setQueueState] = useState<LoadState>("idle");
const [queueError, setQueueError] = useState<string | null>(null);
const [contextState, setContextState] = useState<LoadState>("idle");
const [contextError, setContextError] = useState<string | null>(null);
const [activeRailPanel, setActiveRailPanel] = useState<RailPanel>("summary");
const [viewedFiles, setViewedFiles] = useState<Set<string>>(new Set());
const [submittedRefs, setSubmittedRefs] = useState<Set<string>>(new Set());
const [draftDirtySincePreflight, setDraftDirtySincePreflight] = useState(true);
const [submitting, setSubmitting] = useState(false);
```

- [ ] **Step 2: Update queue loading**

Wrap `loadReviewQueue` calls with `queueState` and `queueError`, preserving existing selection behavior.

- [ ] **Step 3: Update PR selection loading**

Wrap `collectPrContext` with `contextState` and `contextError`, preserve selected PR identity on failure, and avoid clearing the workspace into an indistinguishable empty state.

- [ ] **Step 4: Replace old top-level nav**

Render only `Inbox` and `Settings`. The `Inbox` screen contains the PR-centric shell: left inbox rail, central workspace, right review rail.

- [ ] **Step 5: Run render test**

Run: `npm test -- ui/src/App.test.tsx`

Expected: still may fail until Task 6/7 components are wired, but setup gate should pass.

## Task 6: Inbox Rail and PR Cards

**Files:**
- Modify: `ui/src/App.tsx`

- [ ] **Step 1: Replace `ReviewInboxScreen`**

Make `ReviewInboxScreen` render:

- repository picker
- refresh button with pending text/state
- filter controls
- section list from `groupQueueBySection`
- PR cards with `summarizeQueueItem` and `prRiskBadges`

- [ ] **Step 2: Add explicit queue states**

Render:

```tsx
if (queueState === "loading") return <StatusState title="Loading review inbox" />;
if (queueState === "error") return <StatusState title="Could not load review inbox" action={onRefresh} />;
if (queue.length === 0) return <StatusState title={t(locale, "inbox.empty")} />;
```

- [ ] **Step 3: Track draft refs**

Build `draftRefs` from selected PR when draft has content:

```ts
const draftRefs = useMemo(() => {
  const refs = new Set<string>();
  if (selectedPr && draft.trim()) refs.add(pullRequestRef(selectedPr));
  return refs;
}, [draft, selectedPr]);
```

- [ ] **Step 4: Run helper and render tests**

Run: `npm test -- ui/src/lib/view-models.test.ts ui/src/App.test.tsx`

Expected: view-model tests PASS; render tests closer to GREEN.

## Task 7: PR Workspace and Diff Viewer

**Files:**
- Modify: `ui/src/App.tsx`

- [ ] **Step 1: Add `DiffViewer` component**

Render parsed hunks from `parseUnifiedPatch(selectedFile.patch)`. Each line shows old line, new line, and content with classes for add/delete/context.

- [ ] **Step 2: Add file viewed toggle**

File rows include viewed state and button:

```tsx
onClick={() => setViewedFiles((current) => new Set(current).add(file.path))}
```

- [ ] **Step 3: Add hunk navigation**

Add small hunk list above diff:

```tsx
{hunks.map((hunk, index) => (
  <button key={hunk.header} onClick={() => document.getElementById(`hunk-${index}`)?.scrollIntoView({ block: "start" })}>
    Hunk {index + 1}
  </button>
))}
```

- [ ] **Step 4: Add overview header**

Display PR title/ref/author/head SHA/additions/deletions/freshness/patch coverage/AI included count.

- [ ] **Step 5: Handle no-patch states**

Show binary/no textual patch copy instead of empty diff for null patch.

- [ ] **Step 6: Run tests**

Run: `npm test -- ui/src/lib/view-models.test.ts ui/src/App.test.tsx`

Expected: PASS for diff helper and shell render.

## Task 8: Review Rail Panels

**Files:**
- Modify: `ui/src/App.tsx`

- [ ] **Step 1: Add rail tabs**

Render `Summary`, `AI`, `Draft`, `Safety`, `Activity` as segmented buttons. Keep `activeRailPanel` stable when selecting files.

- [ ] **Step 2: Move Agent UI into `AiRailPanel`**

Reuse existing model, reasoning, private consent, and `runAgent` state. Show blocked reason and included file count in the rail.

- [ ] **Step 3: Move Draft UI into `DraftRailPanel`**

Reuse existing verdict/draft/explicit confirmation state. Draft edits set `draftDirtySincePreflight` and clear preflight/submit message.

- [ ] **Step 4: Move Submit UI into `SafetyRailPanel`**

Use `deriveSubmitSafetyState` and existing `prepareSubmit`/`confirmSubmit`. Show blockers, ready state, prepare, confirm.

- [ ] **Step 5: Update submit success**

On `confirmSubmit` success, add selected PR ref to `submittedRefs` and set `submitting` false.

- [ ] **Step 6: Run tests**

Run: `npm test -- ui/src/App.test.tsx ui/src/lib/view-models.test.ts`

Expected: PASS.

## Task 9: Documentation and Verification

**Files:**
- Modify: `docs/prd/prd-0.1.0.md`
- Add: `docs/verification/reviewdesk-tauri-0.1.0.md`

- [ ] **Step 1: Run full frontend tests**

Run: `npm test`

Expected: PASS.

- [ ] **Step 2: Run frontend build/lint**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 3: Run Rust tests**

Run: `cargo test`

Expected: PASS.

- [ ] **Step 4: Update PRD status**

Change `docs/prd/prd-0.1.0.md` status from `Draft` to `Implemented and verified`, and add an implementation result section listing completed P0 items and deferred P1/P2 items.

- [ ] **Step 5: Write verification doc**

Create `docs/verification/reviewdesk-tauri-0.1.0.md` with:

```md
# ReviewDesk 0.1.0 Verification

## Scope

Verified PRD 0.1.0 PR-centric Review Shell implementation.

## Commands

- `npm test`
- `npm run build`
- `cargo test`

## Manual Checks

- GitHub missing shows Setup gate.
- GitHub connected demo mode shows Review Inbox.
- Selecting a PR keeps overview, diff, AI, draft, and safety in one workspace.
- Queue/context loading and error states are visually distinct from empty states.
- Diff viewer shows line numbers, hunk headers, and add/delete/context styling.
- Draft edits invalidate submit readiness.
- Safety rail blocks confirm until preflight is ready.

## Deferred

P1/P2 items remain documented future work: true command palette, persisted reviewed state, full inline comments, suggested changes, and AI-native guided walkthrough.
```

- [ ] **Step 6: Final verification scan**

Run:

```bash
rg -n "TBD|TODO|FIXME|placeholder|mock only" docs/prd/prd-0.1.0.md docs/verification/reviewdesk-tauri-0.1.0.md ui/src
git status --short
```

Expected: no unintended placeholders; status includes only intended files.

## Self-Review

- Spec coverage: P0 requirements map to Tasks 1-8. P1/P2 items remain deferred because PRD 0.1.0 and Non-goals exclude full inline comments/suggestions and backend protocol changes.
- Placeholder scan: no `TBD`, `TODO`, or "similar to" placeholders are used in executable steps.
- Type consistency: helper names are consistent across tests and implementation steps.

