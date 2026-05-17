# ReviewDesk Tauri 0.0.4 Verification

## 범위

검증 대상은 `docs/prd/prd-0.0.4.md` 기반 구현이다.

구현 범위:

- Rust AppCore boundary
- Tauri v2 shell
- Tauri IPC command adapter
- React/TypeScript/Tailwind UI shell
- Review Inbox, Command Bar, Changed Files/Diff Workspace, Agent Run Panel, Draft Composer, Submit Safety Dock
- frontend-safe status and submit preflight
- 문서 최신화

비범위:

- 실제 ChatGPT OAuth generation adapter
- API key 입력 UX
- ChatGPT cookie/session scraping
- inline review comments
- local checkout/test runner
- Tauri signing/updater/release packaging

## 구현 파일

Rust core:

- `src/app_core.rs`
- `src/security.rs`
- `src/lib.rs`

Tauri shell:

- `src-tauri/Cargo.toml`
- `src-tauri/build.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/main.json`
- `src-tauri/src/main.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/icons/icon.png`

Frontend:

- `package.json`
- `package-lock.json`
- `index.html`
- `vite.config.ts`
- `tsconfig.json`
- `ui/src/main.tsx`
- `ui/src/App.tsx`
- `ui/src/styles.css`
- `ui/src/components/ui.tsx`
- `ui/src/lib/ipc.ts`
- `ui/src/lib/utils.ts`
- `ui/src/lib/view-models.ts`
- `ui/src/lib/view-models.test.ts`

Tests:

- `tests/app_core_tests.rs`
- `tests/tauri_shell_tests.rs`

## 보안 검증 포인트

- `AppStatusView`는 GitHub/ChatGPT 연결 상태만 반환하고 token, secret, authorization 문자열을 직렬화하지 않는다.
- Tauri command allowlist는 `allowed_ipc_commands()`와 `src-tauri/build.rs`에 같은 명령명을 가진다.
- Tauri capability는 broad `fs:`, `shell:`, `http:`, `process:`, `upload:`, `websocket:` 권한을 사용하지 않는다.
- frontend는 GitHub API 또는 AI provider에 직접 bearer token을 붙이지 않는다.
- submit은 `prepare_submit_review`에서 stale, empty body, explicit verdict, private diff consent 조건을 확인한다.
- `confirm_submit_review`는 제출 직전 Rust에서 PR snapshot을 다시 조회하고 confirmation id를 재검증한다.

## UI 검증 포인트

- 1280x720 in-app browser에서 Review Inbox, changed files/diff, Agent Run Panel, Draft Composer, Submit Safety Dock이 렌더링됨을 확인했다.
- 1024x768 viewport override에서 Review Inbox, Agent Run Panel, Submit Safety Dock, verdict controls가 보이고 텍스트가 겹치지 않도록 조정했다.
- 1024px 폭에 맞추기 위해 main grid를 `280px / minmax(360px, 1fr) / 320px`로 조정했다.
- `REQUEST_CHANGES` 긴 라벨은 segmented control 안에서 겹치지 않도록 `Changes`로 표시한다.

## ChatGPT OAuth 상태

PRD 요구사항에 따라 API key 입력 UX와 비공식 ChatGPT 세션 재사용은 구현하지 않았다.

현재 구현은 다음과 같다.

- `start_chatgpt_oauth`
- `poll_chatgpt_oauth`
- ChatGPT missing status
- model selector
- reasoning depth selector
- AI run blocked reason

실제 generation adapter는 공식 독립 앱용 ChatGPT OAuth 경로가 연결될 때까지 blocked 상태로 둔다.

## 검증 명령

최종 검증 결과:

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo check` | PASS |
| `cargo test` | PASS: Rust tests 37 passed, 0 failed |
| `npm test` | PASS: Vitest 4 passed, 0 failed |
| `npm run build` | PASS: Vite production build completed |
| `cargo check --manifest-path src-tauri/Cargo.toml` | PASS |

브라우저 확인:

```text
http://127.0.0.1:1420/
```

확인 내용:

- Review Inbox visible
- Agent Run visible
- Submit Safety Dock visible
- 1024x768 viewport에서 주요 영역 visible
- verdict segmented control text no overlap

## 남은 리스크

- 실제 GitHub OAuth는 `REVIEWDESK_GITHUB_CLIENT_ID` 환경 변수가 필요하다.
- GitHub organization SSO와 fine-grained scope는 실제 계정으로 추가 수동 QA가 필요하다.
- ChatGPT OAuth generation adapter는 공식 지원 경로가 확정되어야 연결 가능하다.
- Tauri app signing, updater, installer packaging은 0.0.4 범위 밖이다.
