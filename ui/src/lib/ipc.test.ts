import { describe, expect, it } from "vitest";
import {
  buildConfirmSubmitReviewRequest,
  buildPrepareSubmitReviewRequest,
  isReviewDeskDemoSearch,
  shouldUseDemoFallback,
  type ReviewPublishPayload,
} from "./ipc";

describe("ipc runtime selection", () => {
  it("lets demo mode override host Tauri globals in the in-app browser", () => {
    expect(shouldUseDemoFallback("load_review_queue", true, true)).toBe(true);
    expect(shouldUseDemoFallback("confirm_submit_review", true, true)).toBe(false);
    expect(shouldUseDemoFallback("get_app_status", false, false)).toBe(true);
    expect(shouldUseDemoFallback("load_review_queue", false, true)).toBe(false);
  });

  it("supports an explicit browser query flag for local visual verification", () => {
    expect(isReviewDeskDemoSearch("?reviewdesk_demo=1")).toBe(true);
    expect(isReviewDeskDemoSearch("?reviewdesk_demo=false")).toBe(false);
    expect(isReviewDeskDemoSearch("")).toBe(false);
  });

  it("wraps submit preflight and confirmation around the publish payload contract", () => {
    const payload: ReviewPublishPayload = {
      owner: "company",
      repo: "payment-web",
      number: 582,
      expected_head_sha: "head-a",
      expected_diff_hash: "diff-a",
      body: "review body",
      event: "COMMENT",
      inline_comments: [],
      explicit_verdict_confirmed: true,
      private_diff_consent_required: true,
      private_diff_consent_accepted: true,
    };

    expect(
      buildPrepareSubmitReviewRequest({
        payload,
        github_connected: true,
        write_scope_valid: true,
        sso_required: false,
        pr_open: true,
        pr_merged: false,
        current_head_sha: "head-a",
        current_diff_hash: "diff-a",
      }),
    ).toEqual({
      payload,
      github_connected: true,
      write_scope_valid: true,
      sso_required: false,
      pr_open: true,
      pr_merged: false,
      current_head_sha: "head-a",
      current_diff_hash: "diff-a",
    });
    expect(buildConfirmSubmitReviewRequest({ payload, confirmation_id: "confirm-a" })).toEqual({
      payload,
      confirmation_id: "confirm-a",
    });
  });
});
