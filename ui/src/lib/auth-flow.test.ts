import { describe, expect, it } from "vitest";
import {
  authPollTerminalMessage,
  shouldPollCodexLogin,
  shouldPollGithubOAuth,
} from "./auth-flow";

describe("auth flow helpers", () => {
  it("polls GitHub browser and device flows after the browser opens", () => {
    expect(shouldPollGithubOAuth({ flow_id: "github-1", status: "callback_waiting" })).toBe(true);
    expect(shouldPollGithubOAuth({ flow_id: "github-1", status: "pending" })).toBe(true);
    expect(shouldPollGithubOAuth({ flow_id: "github-1", status: "authorized" })).toBe(false);
    expect(shouldPollGithubOAuth({ flow_id: null, status: "callback_waiting" })).toBe(false);
  });

  it("polls Codex login only while login is pending", () => {
    expect(shouldPollCodexLogin({ login_id: "codex-1", status: "pending" })).toBe(true);
    expect(shouldPollCodexLogin({ login_id: "codex-1", status: "connected" })).toBe(false);
    expect(shouldPollCodexLogin({ login_id: "", status: "pending" })).toBe(false);
  });

  it("formats terminal auth states without leaking raw payloads", () => {
    expect(authPollTerminalMessage("GitHub", { status: "authorized", disabled_reason: null })).toBe(
      "GitHub connected.",
    );
    expect(authPollTerminalMessage("Codex", { status: "denied", disabled_reason: "access_denied" })).toBe(
      "Codex denied: access_denied",
    );
  });
});
