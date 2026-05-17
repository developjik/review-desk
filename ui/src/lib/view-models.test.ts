import { describe, expect, it } from "vitest";
import {
  commandDisabledReason,
  containsCredentialWords,
  defaultLanguagePreferences,
  groupQueueBySection,
  deriveSubmitSafetyState,
  parseUnifiedPatch,
  prRiskBadges,
  reasoningEffortsForModel,
  resolveReviewLocale,
  reviewSearchPlaceholder,
  selectableAiModels,
  shouldShowStartupGate,
  startupGateLabel,
  summarizeQueueItem,
  type AppStatusView,
  type LanguagePreferences,
  type PullRequestQueueItem,
  type SubmitSafetyInput,
} from "./view-models";
import { catalogHasParity, requiredI18nKeys, t } from "./i18n";

describe("frontend view-model helpers", () => {
  it("labels startup auth gates in GitHub-first order", () => {
    const status: AppStatusView = {
      github: "missing",
      chatgpt: "missing",
      startup_gate_required: true,
      review_queue_available: false,
      ai_review_available: false,
      selected_model: "gpt-5.4",
      selected_reasoning_depth: "medium",
      ai_blocked_reason: "codex_chatgpt_auth_required",
      commands: [],
    };

    expect(startupGateLabel(status)).toBe("Connect GitHub in Browser to load your review inbox");
  });

  it("keeps queue commands enabled when GitHub is connected but ChatGPT is missing", () => {
    const status: AppStatusView = {
      github: "connected",
      chatgpt: "missing",
      startup_gate_required: false,
      review_queue_available: true,
      ai_review_available: false,
      selected_model: "gpt-5.4",
      selected_reasoning_depth: "medium",
      ai_blocked_reason: "codex_chatgpt_auth_required",
      commands: [],
    };

    expect(commandDisabledReason(status, "load_review_queue")).toBeNull();
    expect(commandDisabledReason(status, "generate_review_draft")).toBe(
      "codex_chatgpt_auth_required",
    );
  });

  it("uses Codex blocked reasons for Agent Run capability", () => {
    const status: AppStatusView = {
      github: "connected",
      chatgpt: "missing",
      startup_gate_required: false,
      review_queue_available: true,
      ai_review_available: false,
      selected_model: "gpt-5.5",
      selected_reasoning_depth: "medium",
      selected_reasoning_effort: "medium",
      ai_blocked_reason: "codex_chatgpt_auth_required",
      ai_connection: {
        provider: "codex_chatgpt",
        status: "missing",
        authMode: null,
        blockedReason: "codex_chatgpt_auth_required",
      },
      ai_models: [],
      ai_rate_limit: {
        status: "unknown",
        checkedAt: null,
        blockedReason: null,
      },
      generation_available: false,
      generation_blocked_reason: "codex_chatgpt_auth_required",
      commands: [],
    };

    expect(commandDisabledReason(status, "load_review_queue")).toBeNull();
    expect(commandDisabledReason(status, "generate_review_draft")).toBe(
      "codex_chatgpt_auth_required",
    );
  });

  it("filters hidden models and exposes reasoning effort from selected model", () => {
    const models: AppStatusView["ai_models"] = [
      {
        id: "gpt-5.5",
        displayName: "GPT-5.5",
        isDefault: true,
        hidden: false,
        available: true,
        unavailableReason: null,
        supportedReasoningEfforts: ["low", "medium", "high"],
        defaultReasoningEffort: "medium",
        inputModalities: ["text"],
        upgrade: null,
      },
      {
        id: "gpt-5.3-codex-spark",
        displayName: "GPT-5.3-Codex-Spark",
        isDefault: false,
        hidden: false,
        available: false,
        unavailableReason: "plan_required",
        supportedReasoningEfforts: ["low"],
        defaultReasoningEffort: "low",
        inputModalities: ["text"],
        upgrade: { requiredPlan: "pro", message: "ChatGPT Pro required" },
      },
      {
        id: "internal-hidden",
        displayName: "Internal Hidden",
        isDefault: false,
        hidden: true,
        available: true,
        unavailableReason: null,
        supportedReasoningEfforts: ["medium"],
        defaultReasoningEffort: "medium",
        inputModalities: ["text"],
        upgrade: null,
      },
    ];

    expect(selectableAiModels(models).map((model) => model.id)).toEqual([
      "gpt-5.5",
      "gpt-5.3-codex-spark",
    ]);
    expect(reasoningEffortsForModel(models, "gpt-5.5")).toEqual(["low", "medium", "high"]);
    expect(reasoningEffortsForModel(models, "missing")).toEqual([]);
  });

  it("supports Codex minimal reasoning and bridge-specific blocked reasons", () => {
    const status: AppStatusView = {
      github: "connected",
      chatgpt: "unsupported",
      startup_gate_required: false,
      review_queue_available: true,
      ai_review_available: false,
      selected_model: "gpt-5.5",
      selected_reasoning_depth: "minimal",
      selected_reasoning_effort: "minimal",
      ai_blocked_reason: "codex_auth_mode_unsupported",
      ai_connection: {
        provider: "codex_chatgpt",
        status: "unsupported",
        authMode: "apikey",
        blockedReason: "codex_auth_mode_unsupported",
      },
      commands: [],
    };

    expect(commandDisabledReason(status, "generate_review_draft")).toBe(
      "codex_auth_mode_unsupported",
    );
    expect(startupGateLabel(status)).toBe("Connect Codex to run AI review drafts");
  });

  it("blocks app shell in strict auth mode until ChatGPT is connected", () => {
    const status: AppStatusView = {
      github: "connected",
      chatgpt: "missing",
      startup_gate_required: true,
      review_queue_available: false,
      ai_review_available: false,
      selected_model: "gpt-5.4",
      selected_reasoning_depth: "medium",
      ai_blocked_reason: "codex_chatgpt_auth_required",
      strict_auth_mode: true,
      capabilities: {
        app_shell_available: false,
        review_queue_available: false,
        pr_context_available: false,
        ai_review_available: false,
        manual_draft_available: false,
        submit_available: false,
        settings_available: true,
        blocked_reasons: ["codex_chatgpt_auth_required"],
      },
      language_preferences: {
        ui_locale: "en",
        review_locale: "en",
        repo_review_locale_overrides: {},
      },
      commands: [],
    };

    expect(shouldShowStartupGate(status)).toBe(true);
    expect(commandDisabledReason(status, "load_review_queue")).toBe("codex_chatgpt_auth_required");
  });

  it("resolves default and repo-specific review language", () => {
    expect(defaultLanguagePreferences("ko-KR")).toEqual({
      ui_locale: "ko",
      review_locale: "ko",
      repo_review_locale_overrides: {},
    });
    expect(defaultLanguagePreferences("en-US")).toEqual({
      ui_locale: "en",
      review_locale: "en",
      repo_review_locale_overrides: {},
    });

    const preferences: LanguagePreferences = {
      ui_locale: "ko",
      review_locale: "en",
      repo_review_locale_overrides: {
        "company/payment-web": "ko",
      },
    };

    expect(resolveReviewLocale(preferences, "company/payment-web", "en")).toBe("ko");
    expect(resolveReviewLocale(preferences, "company/admin", "ko")).toBe("ko");
    expect(resolveReviewLocale(preferences, "company/admin", null)).toBe("en");
  });

  it("keeps English and Korean catalogs in sync", () => {
    expect(catalogHasParity()).toBe(true);
    expect(t("en", "nav.inbox")).toBe("Inbox");
    expect(t("ko", "nav.inbox")).toBe("인박스");
    expect(t("en", "auth.connectGithub")).toBe("Connect GitHub in Browser");
    expect(t("ko", "auth.connectGithub")).toBe("브라우저에서 GitHub 연결");
    expect(t("en", "auth.connectChatGPT")).toBe("Connect Codex");
    expect(t("ko", "auth.connectChatGPT")).toBe("Codex 연결");
    expect(t("en", "auth.useGithubDeviceCode")).toBe("Use GitHub device code instead");
    expect(t("ko", "unknown.key")).toBe("unknown.key");
    for (const key of requiredI18nKeys()) {
      expect(t("en", key)).not.toBe(key);
      expect(t("ko", key)).not.toBe(key);
    }
  });

  it("detects credential-like fields before rendering raw status payloads", () => {
    expect(containsCredentialWords(JSON.stringify({ access_token: "secret" }))).toBe(true);
    expect(containsCredentialWords(JSON.stringify({ client_secret: "secret" }))).toBe(true);
    expect(containsCredentialWords(JSON.stringify({ code_verifier: "secret" }))).toBe(true);
    expect(containsCredentialWords(JSON.stringify({ code: "github-oauth-code" }))).toBe(true);
    expect(containsCredentialWords("Authorization: Bearer gho_secret")).toBe(true);
    expect(containsCredentialWords(JSON.stringify({ github: "connected" }))).toBe(false);
  });

  it("summarizes queue rows for dense inbox rendering", () => {
    const item: PullRequestQueueItem = {
      owner: "company",
      repo: "payment-web",
      number: 582,
      title: "Payment refactor",
      author: "alice",
      url: "https://github.com/company/payment-web/pull/582",
      draft: false,
      review_requested: true,
      assigned: true,
      ci_status: "pass",
      changed_files_count: 26,
      updated_at: null,
      review_reason: null,
      stale_status: "fresh",
    };

    expect(summarizeQueueItem(item)).toEqual({
      ref: "company/payment-web#582",
      reason: "review requested, assigned",
      ci: "pass",
      files: "26 files",
    });
  });

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

  it("parses unified patches into hunks with line numbers and line types", () => {
    const hunks = parseUnifiedPatch(
      "@@ -70,2 +70,3 @@ export async function request(input) {\n-  return fetch(input)\n+  await refreshSession()\n+  return fetch(input)\n }",
    );

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
