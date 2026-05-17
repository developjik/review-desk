import { describe, expect, it } from "vitest";
import {
  dismissInlineComment,
  editInlineCommentBody,
  invalidSelectedInlineComments,
  publishableInlineComments,
  restoreInlineComment,
  selectInlineComment,
  setInlineCommentSelection,
  type InlineCommentDraftView,
} from "./inline-comments";

function comment(overrides: Partial<InlineCommentDraftView> = {}): InlineCommentDraftView {
  return {
    id: "c1",
    path: "src/payment.ts",
    side: "RIGHT",
    line: 42,
    start_line: null,
    start_side: null,
    body: "Check this",
    severity: null,
    confidence: null,
    source_run_id: null,
    source_finding_id: null,
    selected_for_publish: false,
    dismissed: false,
    user_edited: false,
    mapping_status: "valid",
    ...overrides,
  };
}

describe("inline comment helpers", () => {
  it("selects and dismisses inline comments without changing body text", () => {
    const original = comment();

    expect(selectInlineComment(original, true).selected_for_publish).toBe(true);
    const dismissed = dismissInlineComment(original);

    expect(dismissed.dismissed).toBe(true);
    expect(dismissed.body).toBe("Check this");
    expect(original).toEqual(comment());
  });

  it("edits, restores, and updates selection immutably", () => {
    const comments = [comment({ id: "c1" }), comment({ id: "c2", body: "Second", dismissed: true })];
    const edited = editInlineCommentBody(comments, "c1", "Updated");
    const restored = restoreInlineComment(edited, "c2");
    const selected = setInlineCommentSelection(restored, "c2", true);

    expect(edited).not.toBe(comments);
    expect(edited[0]).not.toBe(comments[0]);
    expect(edited[0]).toMatchObject({ body: "Updated", user_edited: true });
    expect(restored[1]).toMatchObject({ dismissed: false });
    expect(selected[1]).toMatchObject({ selected_for_publish: true, body: "Second" });
    expect(comments[0]).toMatchObject({ body: "Check this", user_edited: false });
    expect(comments[1]).toMatchObject({ dismissed: true, selected_for_publish: false });
  });

  it("filters publishable and invalid selected inline comments", () => {
    const comments = [
      comment({ id: "valid", selected_for_publish: true }),
      comment({ id: "blank", selected_for_publish: true, body: "  " }),
      comment({ id: "dismissed", selected_for_publish: true, dismissed: true }),
      comment({ id: "invalid", selected_for_publish: true, mapping_status: "invalid_line" }),
      comment({ id: "unselected", selected_for_publish: false, mapping_status: "invalid_line" }),
    ];

    expect(publishableInlineComments(comments).map((item) => item.id)).toEqual(["valid"]);
    expect(invalidSelectedInlineComments(comments).map((item) => item.id)).toEqual(["invalid"]);
  });
});
