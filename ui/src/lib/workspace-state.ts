import type { SubmitPreflightView } from "./ipc";
import type { PullRequestQueueItem } from "./view-models";
import type { AnalysisRunView, InlineCommentDraftView, ReviewDraftView } from "./workspace-view-models";

export interface WorkspaceState {
  selectedPr: PullRequestQueueItem | null;
  runs: AnalysisRunView[];
  activeDraft: ReviewDraftView | null;
  pendingRunIds: string[];
  validatedInlineComments: InlineCommentDraftView[];
  preparedPublish: SubmitPreflightView | null;
}

export type WorkspaceAction =
  | { type: "select_pr"; pr: PullRequestQueueItem }
  | { type: "load_runs_success"; runs: AnalysisRunView[] }
  | { type: "load_draft_success"; draft: ReviewDraftView | null }
  | { type: "start_run"; run: AnalysisRunView }
  | { type: "run_completed"; run: AnalysisRunView; draft?: ReviewDraftView | null }
  | { type: "edit_draft_body"; body: string; updated_at?: string }
  | { type: "replace_draft_from_run"; draft: ReviewDraftView }
  | { type: "save_draft_success"; draft: ReviewDraftView }
  | { type: "validate_inline_success"; comments: InlineCommentDraftView[] }
  | { type: "prepare_publish_success"; preflight: SubmitPreflightView };

export const initialWorkspaceState: WorkspaceState = {
  selectedPr: null,
  runs: [],
  activeDraft: null,
  pendingRunIds: [],
  validatedInlineComments: [],
  preparedPublish: null,
};

export function workspaceReducer(state: WorkspaceState, action: WorkspaceAction): WorkspaceState {
  switch (action.type) {
    case "select_pr":
      return {
        ...initialWorkspaceState,
        selectedPr: action.pr,
      };
    case "load_runs_success":
      return {
        ...state,
        runs: sortRuns(action.runs),
        pendingRunIds: action.runs.filter((run) => run.status === "queued" || run.status === "running").map((run) => run.run_id),
      };
    case "load_draft_success":
      return {
        ...state,
        activeDraft: action.draft,
        validatedInlineComments: action.draft?.inline_comments ?? [],
        preparedPublish: null,
      };
    case "start_run":
      return {
        ...state,
        runs: sortRuns(upsertRun(state.runs, action.run)),
        pendingRunIds: Array.from(new Set([...state.pendingRunIds, action.run.run_id])),
      };
    case "run_completed": {
      if (!state.selectedPr || !runMatchesSelectedPr(action.run, state.selectedPr)) return state;
      const nextRuns = sortRuns(upsertRun(state.runs, action.run));
      const activeDraft = state.activeDraft?.user_edited ? state.activeDraft : action.draft ?? state.activeDraft;
      return {
        ...state,
        runs: nextRuns,
        activeDraft,
        validatedInlineComments: activeDraft?.inline_comments ?? state.validatedInlineComments,
        pendingRunIds: state.pendingRunIds.filter((runId) => runId !== action.run.run_id),
      };
    }
    case "edit_draft_body":
      if (!state.activeDraft) return state;
      return {
        ...state,
        activeDraft: {
          ...state.activeDraft,
          body: action.body,
          user_edited: true,
          updated_at: action.updated_at ?? new Date().toISOString(),
        },
        preparedPublish: null,
      };
    case "replace_draft_from_run":
      return {
        ...state,
        activeDraft: action.draft,
        validatedInlineComments: action.draft.inline_comments,
        preparedPublish: null,
      };
    case "save_draft_success":
      return {
        ...state,
        activeDraft: action.draft,
        validatedInlineComments: action.draft.inline_comments,
      };
    case "validate_inline_success":
      return {
        ...state,
        validatedInlineComments: action.comments,
        activeDraft: state.activeDraft ? { ...state.activeDraft, inline_comments: action.comments } : state.activeDraft,
        preparedPublish: null,
      };
    case "prepare_publish_success":
      return {
        ...state,
        preparedPublish: action.preflight,
      };
  }
}

export function sortRuns(runs: AnalysisRunView[]): AnalysisRunView[] {
  return (Array.isArray(runs) ? [...runs] : []).sort((left, right) => right.created_at.localeCompare(left.created_at));
}

function upsertRun(runs: AnalysisRunView[], nextRun: AnalysisRunView): AnalysisRunView[] {
  const found = runs.some((run) => run.run_id === nextRun.run_id);
  if (!found) return [...runs, nextRun];
  return runs.map((run) => (run.run_id === nextRun.run_id ? nextRun : run));
}

function runMatchesSelectedPr(run: AnalysisRunView, selectedPr: PullRequestQueueItem): boolean {
  return run.owner === selectedPr.owner && run.repo === selectedPr.repo && run.number === selectedPr.number;
}
