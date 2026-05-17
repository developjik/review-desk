type GithubStartLike = {
  flow_id: string | null;
  status: string;
};

type CodexStartLike = {
  login_id: string;
  status: string;
};

type PollLike = {
  status: string;
  disabled_reason?: string | null;
};

export function shouldPollGithubOAuth(start: GithubStartLike): boolean {
  return Boolean(
    start.flow_id &&
      ["callback_waiting", "pending", "slow_down"].includes(start.status),
  );
}

export function shouldPollCodexLogin(start: CodexStartLike): boolean {
  return Boolean(start.login_id && start.status === "pending");
}

export function authPollTerminalMessage(provider: string, poll: PollLike): string | null {
  if (poll.status === "authorized" || poll.status === "connected") {
    return `${provider} connected.`;
  }
  if (["denied", "failed", "timeout", "cancelled", "blocked"].includes(poll.status)) {
    return `${provider} ${poll.status}: ${poll.disabled_reason ?? "authorization did not complete"}`;
  }
  return null;
}
