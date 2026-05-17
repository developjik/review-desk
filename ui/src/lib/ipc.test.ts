import { describe, expect, it } from "vitest";
import { isReviewDeskDemoSearch, shouldUseDemoFallback } from "./ipc";

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
});
