import { describe, expect, it } from "vitest";
import {
  buildPublishPayload,
  derivePublishBlockers,
  type BuildPublishPayloadInput,
} from "./publish-safety";

function payloadInput(overrides: Partial<BuildPublishPayloadInput> = {}): BuildPublishPayloadInput {
  return {
    owner: "company",
    repo: "payment-web",
    number: 582,
    expected_head_sha: "head",
    expected_diff_hash: "diff",
    event: "COMMENT",
    body: "Body",
    inline_comments: [],
    explicit_verdict_confirmed: false,
    private_diff_consent_required: false,
    private_diff_consent_accepted: false,
    ...overrides,
  };
}

describe("publish safety", () => {
  it("allows empty body when one valid inline comment is selected", () => {
    const payload = buildPublishPayload(
      payloadInput({
        body: "",
        inline_comments: [
          {
            id: "c1",
            selected_for_publish: true,
            dismissed: false,
            mapping_status: "valid",
            body: "Fix this",
          },
        ],
      }),
    );

    expect(derivePublishBlockers(payload, { current_head_sha: "head", current_diff_hash: "diff" })).toEqual([]);
    expect(payload.inline_comments.map((comment) => comment.id)).toEqual(["c1"]);
  });

  it("blocks unresolved inline mappings", () => {
    const payload = buildPublishPayload(
      payloadInput({
        inline_comments: [
          {
            id: "c1",
            selected_for_publish: true,
            dismissed: false,
            mapping_status: "invalid_line",
            body: "Fix this",
          },
        ],
      }),
    );

    expect(derivePublishBlockers(payload, { current_head_sha: "head", current_diff_hash: "diff" })).toContain(
      "inline_mapping_invalid",
    );
  });

  it("ignores unselected dismissed and empty invalid inline mappings", () => {
    const payload = buildPublishPayload(
      payloadInput({
        inline_comments: [
          {
            id: "unselected",
            selected_for_publish: false,
            dismissed: false,
            mapping_status: "invalid_line",
            body: "Fix this",
          },
          {
            id: "dismissed",
            selected_for_publish: true,
            dismissed: true,
            mapping_status: "missing_path",
            body: "Fix this",
          },
          {
            id: "empty",
            selected_for_publish: true,
            dismissed: false,
            mapping_status: "stale_diff",
            body: " ",
          },
        ],
      }),
    );

    expect(derivePublishBlockers(payload, { current_head_sha: "head", current_diff_hash: "diff" })).not.toContain(
      "inline_mapping_invalid",
    );
  });

  it("derives all publish blockers from the exact payload", () => {
    const payload = buildPublishPayload(
      payloadInput({
        expected_head_sha: "old-head",
        expected_diff_hash: "old-diff",
        event: "REQUEST_CHANGES",
        body: " ",
        explicit_verdict_confirmed: false,
        private_diff_consent_required: true,
        private_diff_consent_accepted: false,
      }),
    );

    expect(derivePublishBlockers(payload, { current_head_sha: "head", current_diff_hash: "diff" })).toEqual([
      "head_changed",
      "diff_changed",
      "payload_empty",
      "explicit_verdict_confirmation_required",
      "private_diff_consent_required",
    ]);
  });
});
