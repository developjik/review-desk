# ReviewDesk Run-Centric Workspace Verification

Date: 2026-05-17

## Commands

- `cargo fmt --check`: pass
- `cargo test`: pass
- `cargo test --manifest-path src-tauri/Cargo.toml`: pass
- `npm test`: pass (46 tests)
- `npm run build`: pass
- `npm run desktop:build`: pass
- `git diff --check`: pass
- Browser smoke test at `http://127.0.0.1:1420/?reviewdesk_demo=1`: pass, with no console errors

## Verified Behaviors

- Repeated analysis runs are unique and durable.
- Analysis run IPC commands are registered in Tauri, allowed by app-core policy, enabled by capabilities, and backed by generated permissions.
- Analysis runs can be listed, started, read, cancelled, and archived through persisted per-PR workspace storage.
- Drafts persist per PR and retain source run ids.
- User-edited drafts are not overwritten by late run completion.
- Inline comments validate against current diff lines.
- Publish preflight blocks stale head, stale diff, invalid mappings, closed or merged PRs, auth/scope/SSO blockers, and empty payload.
- Publish preview and confirm submit use the same payload shape.
- GitHub review submission includes selected inline comments.
- Stale prepare, validation, run completion, and submit completion responses are ignored after switching PRs.
- Switching PRs while a publish confirmation is in flight clears submit progress and closes the old confirmation dialog.
