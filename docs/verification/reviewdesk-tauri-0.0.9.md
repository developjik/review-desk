# ReviewDesk Tauri 0.0.9 Verification Gate

## 1. 검증 대상

- PRD: `docs/prd/prd-0.0.9.md`
- 제품명: ReviewDesk
- 버전: 0.0.9
- 검증일: 2026-05-17
- 상태: Local implementation gate passed
- 목적: GitHub/Codex 연결 신뢰성, Review Inbox, PR context freshness, Submit Safety Dock, packaging, secret policy를 0.0.9 기준으로 검증한다.

## 2. 자동 검증 명령

| Command | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --check` | Pass | no formatting diff |
| `cargo test` | Pass | 65 Rust integration tests passed, doc tests passed |
| `npm test -- --run` | Pass | 2 Vitest files, 13 tests passed |
| `npm run build` | Pass | `tsc --noEmit` + Vite build, `dist/assets/index-CofRTCag.js` 생성 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | Pass | `reviewdesk-tauri v0.0.9` check finished |
| `npm run desktop:build` | Pass | `.app` bundle 생성: `src-tauri/target/release/bundle/macos/ReviewDesk.app` |

참고: 이 환경에는 `cargo tauri` subcommand가 없어 `cargo tauri build`는 사용할 수 없었다. 프로젝트에 포함된 Tauri CLI를 쓰도록 `npm run desktop:build`를 추가했고, 이 명령으로 동일한 release bundle 검증을 수행했다.

## 3. 구현 검증 요약

| Area | Result | Evidence |
| --- | --- | --- |
| GitHub broker OAuth contract | Pass | `github_browser_oauth_authorize_url_uses_https_broker_pkce_and_state`, `github_broker_redeems_one_time_code_without_client_secret` |
| Device flow fallback contract | Pass | `starts_device_flow_with_client_id_and_scopes`, `handles_device_flow_slow_down` |
| GitHub token validation state | Pass | `get_app_status` path validates `/user`; connected is not based on keychain existence only |
| Repo list contract | Pass | `/user/repos?visibility=all&affiliation=owner,collaborator,organization_member&sort=updated&per_page=100` test coverage |
| Review Inbox query contract | Pass | `user-review-requested:@me`, `assignee:@me`, repo scoped variants, independent pagination, reason merge |
| PR context typed payload | Pass | `PullRequestContextView` includes `pr`, `files`, `conversation`, `ci`, `freshness`, `diff_hash`, `context_hash`, `ai_input` |
| Stale detection | Pass | `fresh`, `head_changed`, `pr_closed`, `pr_merged` contract tested |
| Submit Safety Dock | Pass | normalized reason codes, preflight confirmation id, latest PR snapshot re-fetch in `confirm_submit_review` |
| React context compatibility | Pass | TypeScript build and Vitest pass with new context shape |
| i18n key inventory | Pass | English/Korean catalog parity and required key tests |
| Tauri permission safety | Pass | command allowlist and no broad fs/shell/http/process permissions |
| Packaging metadata | Pass | `version=0.0.9`, bundle active, target `app`, icon metadata present |

## 4. Manual E2E Evidence

Live GitHub/Codex E2E는 이 로컬 검증 환경에 `REVIEWDESK_GITHUB_CLIENT_ID`, `REVIEWDESK_OAUTH_BROKER_URL`, live GitHub account/repo fixture, Codex account fixture가 제공되지 않아 실행하지 않았다. 대신 외부 의존이 필요한 흐름은 mocked GitHub/Codex contract tests로 검증했다.

| Scenario | Result | Evidence |
| --- | --- | --- |
| broker-backed GitHub browser OAuth success | Contract pass | broker URL HTTPS requirement, state/PKCE, broker redeem test |
| GitHub device flow success | Contract pass | device flow start/poll tests |
| device flow expired/denied/slow_down | Contract pass | `slow_down` interval test; denied/expired states handled by poll mapper |
| revoked token in keychain | Contract pass | `/user` validation path maps GitHub auth errors to disconnected states |
| scope insufficient / SSO required | Contract pass | GitHub error classifier and submit reason normalization |
| Codex `authMode=chatgpt` | Contract pass | Codex bridge account/model tests |
| Codex missing manual draft flow | Contract pass | app status keeps Review Inbox/manual draft available |
| Codex unsupported auth mode | Contract pass | `apiKey`/`chatgptAuthTokens` blocked tests |
| public/private PR happy path | Contract pass | PR files/comments/reviews/checks/status mocked context tests |
| Submit `COMMENT` | Contract pass | top-level review payload test |
| Submit `APPROVE` / `REQUEST_CHANGES` | Contract pass | explicit verdict preflight test |
| stale head SHA submit block | Contract pass | `head_changed` stale/preflight tests |
| confirmation TTL/body/account changed | Partial | body/head confirmation id tested; TTL/account-specific confirmation is a future hardening item |

## 5. Packaged App Evidence

| Check | Result | Evidence |
| --- | --- | --- |
| `src-tauri/tauri.conf.json` version is `0.0.9` | Pass | `tauri_manifest_is_release_ready_for_0_0_9` |
| bundle active and icon metadata configured | Pass | `icons/icon.icns`, `icons/icon.ico`, `icons/icon.png` |
| artifact generated under `src-tauri/target/release/bundle/` | Pass | `src-tauri/target/release/bundle/macos/ReviewDesk.app` |
| app bundle Info.plist | Pass | `CFBundleIdentifier=dev.reviewdesk.desktop`, version `0.0.9` |
| release binary generated | Pass | `src-tauri/target/release/reviewdesk-tauri` |
| DMG artifact | Not in 0.0.9 local gate | DMG generation depends on Finder automation and was excluded by `--bundles app` |

## 6. Secret Scan Evidence

Broad policy scan:

```bash
rg -n "client_secret|Authorization|Bearer |gho_|ghp_|github_pat_|sk-|access_token|refresh_token|chatgptAuthTokens|auth\\.json" \
  docs src src-tauri ui tests dist src-tauri/target/release/bundle .reviewdesk
```

Result:

- broad scan intentionally reports policy docs, redaction tests, and false positives such as `reviewdesk-tauri` because the PRD pattern contains broad `sk-`.
- focused scan for the previously pasted client id/secret fragments returned `0`.
- focused high-risk token scan found only test fixtures in `tests/security_tests.rs` and `tests/github_tests.rs`.
- `.app` bundle scan excluding binary/icon files returned only a false positive from `reviewdesk-tauri` in `Info.plist` under the broad `sk-` rule.

| Target | Result | Findings | Allowlist / Remediation |
| --- | --- | --- | --- |
| source/docs/tests | Pass with allowlist | policy docs and test fixtures only | no real secret found |
| built frontend bundle | Pass with allowlist | credential-detector strings are embedded by UI safety code | no real token value found |
| Tauri release bundle | Pass with allowlist | broad `sk-` false positive in `reviewdesk-tauri` string | no real token value found |
| runtime logs | Not generated in local gate | no log export created | live E2E required |
| diagnostics export | Not generated in local gate | no diagnostics export created | live E2E required |
| draft/report storage | Not generated in local gate | no `.reviewdesk` output created | live Agent Run required |

## 7. Documentation Update Evidence

| Document | Required update | Result |
| --- | --- | --- |
| `docs/prd/prd-0.0.9.md` | final implemented status and deviations | Updated |
| `docs/verification/reviewdesk-tauri-0.0.9.md` | actual evidence | Updated |
| `docs/setup/reviewdesk-desktop-runbook.md` | dev/packaged execution | Added |
| `docs/setup/github-oauth.md` | broker/device flow/scopes | Added |
| `docs/setup/codex-managed-login.md` | Codex managed auth/model/reasoning | Added |
| `docs/security/oauth-secret-policy.md` | client id/client secret/broker policy | Added |

## 8. Release Decision

- Overall result: Pass for local implementation gate.
- Blocking failures: None in automated tests, TypeScript build, Tauri check, or `.app` packaging.
- Known limitations:
  - Live GitHub broker OAuth and live Codex account E2E were not executed because this environment has no broker/account fixture configured.
  - DMG generation is excluded from the default verification path because Finder automation can hang in local/headless runs; `.app` bundle is the 0.0.9 packaged artifact.
  - confirmation TTL/account-specific invalidation is not implemented yet; head/body/verdict confirmation id and latest PR snapshot re-fetch are implemented.
- Final reviewer: Codex
