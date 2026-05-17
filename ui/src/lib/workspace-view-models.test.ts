import { describe, expect, it } from "vitest";
import {
  groupRunCentricQueue,
  isDraftStale,
  type RunCentricQueueItem,
} from "./workspace-view-models";

function queueItem(overrides: Partial<RunCentricQueueItem> = {}): RunCentricQueueItem {
  return {
    owner: "company",
    repo: "payment-web",
    number: 1,
    title: "Payment refactor",
    author: "alice",
    url: "https://github.com/company/payment-web/pull/1",
    draft: false,
    review_requested: true,
    assigned: true,
    ci_status: null,
    changed_files_count: 1,
    updated_at: "2026-05-17T00:00:00Z",
    ...overrides,
  };
}

describe("workspace view models", () => {
  it("groups queue into run-centric sections", () => {
    const sections = groupRunCentricQueue([
      queueItem({ number: 1 }),
      queueItem({ number: 2, latest_run_status: "running" }),
      queueItem({ number: 3, active_draft_id: "draft-3" }),
      queueItem({ number: 4, stale_status: "head_changed" }),
      queueItem({ number: 5, published_review_id: 99 }),
    ]);

    expect(sections.map((section) => section.id)).toEqual([
      "needs_review",
      "draft_ready",
      "running",
      "blocked_stale",
      "published",
    ]);
    expect(sections.map((section) => section.items.map((item) => item.number))).toEqual([
      [1],
      [3],
      [2],
      [4],
      [5],
    ]);
  });

  it("includes empty queue sections in stable order", () => {
    const sections = groupRunCentricQueue([]);

    expect(sections).toHaveLength(5);
    expect(sections.map((section) => section.id)).toEqual([
      "needs_review",
      "draft_ready",
      "running",
      "blocked_stale",
      "published",
    ]);
    expect(sections.every((section) => section.items.length === 0)).toBe(true);
  });

  it("derives stale draft from head and diff mismatch", () => {
    expect(isDraftStale({ base_head_sha: "old", base_diff_hash: "diff" }, "new", "diff")).toBe(true);
    expect(isDraftStale({ base_head_sha: "head", base_diff_hash: "old" }, "head", "new")).toBe(true);
    expect(isDraftStale({ base_head_sha: "head", base_diff_hash: "diff" }, "head", "diff")).toBe(false);
  });
});
