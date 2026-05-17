# ReviewDesk Tauri 0.0.5 Verification

## 범위

검증 대상은 `docs/prd/prd-0.0.5.md` 기반 구현이다.

구현 범위:

- GitHub-first Startup Auth Gate
- GitHub missing 상태에서 실제 Review Inbox/PR Workspace 차단
- GitHub Connect 버튼이 Tauri IPC `start_github_oauth` 후 `verification_uri`를 `open_external_url`로 열도록 연결
- GitHub-only 상태에서 manual review 가능, GPT/ChatGPT missing 상태에서 Agent Run만 차단하는 capability matrix
- Strict Auth Mode capability
- `Setup`, `Inbox`, `PR Workspace`, `Agent Runs`, `Drafts`, `Settings` screen state 기반 분리
- 영어/한국어 UI language와 review language 분리
- i18n catalog parity test
- review locale metadata를 위한 frontend/Rust language model
- HTTPS-only external URL allowlist
- private diff consent submit preflight block
- version 0.0.5 metadata 반영

비범위:

- 실제 ChatGPT OAuth generation adapter
- GitHub inline comment submit
- local checkout/test runner
- third language support
- GitHub App team mode
- app signing/updater/installer packaging
- URL router/deep link

## 구현 파일

Rust core:

- `src/app_core.rs`

Tauri shell:

- `src-tauri/src/commands.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.lock`

Frontend:

- `package.json`
- `package-lock.json`
- `ui/src/App.tsx`
- `ui/src/lib/ipc.ts`
- `ui/src/lib/i18n.ts`
- `ui/src/lib/view-models.ts`
- `ui/src/lib/view-models.test.ts`

Planning/docs:

- `docs/prd/prd-0.0.5.md`
- `docs/superpowers/plans/2026-05-16-reviewdesk-prd-0.0.5-implementation.md`
- `docs/verification/reviewdesk-tauri-0.0.5.md`

## 제품 검증 포인트

- 앱 첫 실행에서 GitHub token이 없으면 Startup Auth Gate만 표시한다.
- GitHub 미연결 상태에서는 sample queue, sample PR, sample diff가 production fallback으로 렌더링되지 않는다.
- GitHub connected + GPT missing 상태는 Rust capability에서 app shell, review queue, PR context, manual draft, submit을 허용하고 AI review만 차단한다.
- Strict Auth Mode에서는 GitHub connected + GPT missing 상태도 app shell을 차단한다.
- 화면은 단일 대시보드가 아니라 `Inbox`, `Workspace`, `Agent Runs`, `Drafts`, `Settings`로 이동한다.
- Agent Run은 model, reasoning depth, review language, private diff consent를 표시한다.
- Settings와 Setup에서 UI language와 review language를 분리해 선택한다.

## 보안 검증 포인트

- `AppStatusView`는 token, secret, authorization 문자열을 직렬화하지 않는다.
- GitHub/GPT token은 React로 전달하지 않는다.
- React의 sample fallback은 `get_app_status` 또는 명시적 `VITE_REVIEWDESK_DEMO_MODE=1`에서만 허용된다.
- `open_external_url`은 `https` scheme과 GitHub/ChatGPT allowlisted host만 허용한다.
- submit preflight는 private diff consent가 필요한데 동의가 없으면 `private_diff_consent_required`로 차단한다.
- review language는 `en`/`ko` enum으로만 다룬다.

## UI 검증 포인트

Chrome에서 `http://127.0.0.1:1421/`을 열어 확인했다.

확인 내용:

- GitHub missing 상태에서 Setup/Auth Gate만 표시
- 한국어 OS/browser locale에서 한국어 UI 기본 표시
- GitHub 연결 전 Review Inbox/PR Workspace/Diff 미노출
- UI language와 review language selector 표시
- 화면 중앙 auth card 레이아웃에서 텍스트 겹침 없음
- 실제 Tauri 런타임에서는 GitHub device OAuth `verification_uri`가 system browser로 열린다.
- ChatGPT/GPT는 공식 standalone OAuth adapter 미구성 상태이므로 가짜 창을 띄우지 않고 blocked reason을 표시한다.

## 검증 명령

최종 검증 결과:

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo test` | PASS: Rust tests 42 passed, 0 failed |
| `npm test` | PASS: Vitest 7 passed, 0 failed |
| `npm run build` | PASS: TypeScript + Vite production build completed |
| `cargo check --manifest-path src-tauri/Cargo.toml` | PASS |

추가 재검증:

| Command | Result |
| --- | --- |
| `npm run build` after OAuth browser-open wiring | PASS |
| `npm test` after OAuth browser-open wiring | PASS: Vitest 7 passed, 0 failed |
| `cargo test --test app_core_tests external_url_validation_requires_https_and_allowlisted_hosts` | PASS |

추가 확인:

| Check | Result |
| --- | --- |
| `npm run desktop` script | PASS: script exists and maps to `tauri dev` |
| package version | PASS: `0.0.5` |
| Tauri package/config version | PASS: `0.0.5` |

## 남은 리스크

- 실제 GitHub OAuth는 `REVIEWDESK_GITHUB_CLIENT_ID` 환경 변수가 필요하다.
- GitHub organization SSO와 fine-grained scope는 실제 계정으로 추가 수동 QA가 필요하다.
- ChatGPT/GPT OAuth generation adapter는 공식 독립 앱용 연동 경로가 연결될 때까지 blocked 상태다.
- 현재 multi-screen navigation은 PRD대로 internal screen state 기반이며, URL deep link는 0.0.5 비범위다.
- browser demo에서 app shell sample data를 보려면 `VITE_REVIEWDESK_DEMO_MODE=1` 환경 변수를 사용한다.
