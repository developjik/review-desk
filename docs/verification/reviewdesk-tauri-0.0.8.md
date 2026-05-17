# ReviewDesk Tauri 0.0.8 Verification

## 1. 검증 대상

검증 대상은 `docs/prd/prd-0.0.8.md` 기반 구현이다.

0.0.8 구현 범위:

- `CodexBridge` 모듈 추가
- Codex CLI version probe와 binary resolution
- Codex app-server stdio one-shot JSON-RPC bridge
- `account/read`, `account/login/start`, `account/rateLimits/read`, `model/list`, `account/logout` 연결
- `authMode=chatgpt`만 Agent Run 가능 후보로 인정
- `apiKey`, `chatgptAuthTokens`, unknown auth mode 차단
- hardcoded model list 제거 및 `model/list` 결과를 picker source of truth로 사용
- `minimal`, `low`, `medium`, `high`, `xhigh` reasoning effort 지원
- `codex exec --json` 기반 Agent Run generation fallback 연결
- Agent Run run metadata, draft body, report path, cancel/read command 추가
- private diff consent 전송 차단 유지
- Codex process env allowlist 및 token/auth cache boundary 테스트 추가
- React UI 문구를 `Connect ChatGPT OAuth`가 아니라 `Connect Codex` 중심으로 정리
- GitHub/Codex OAuth 시작 후 UI가 poll command로 인증 완료 상태를 갱신하도록 수정
- Settings에 Codex bridge transport/version 상태 표시
- 버전 metadata를 `0.0.8`로 갱신

## 2. 구현 메모

현재 구현은 account/model/rate-limit/login은 Codex app-server stdio JSON-RPC를 사용한다.

Agent Run draft generation은 `codex exec --json --ephemeral --sandbox read-only` 경로를 사용한다. 이는 PRD의 fallback 경로를 제품에서 바로 실행 가능한 경로로 승격한 것이다. 장기적으로는 long-lived app-server의 `thread/start` + `turn/start` streaming lifecycle을 별도 버전에서 교체할 수 있도록 bridge module 경계를 분리했다.

ReviewDesk는 다음을 하지 않는다.

- ChatGPT OAuth token 직접 수신
- ChatGPT refresh token 저장
- `~/.codex/auth.json` 원문 읽기/수정/삭제
- `chatgptAuthTokens` mode 사용
- broad Tauri `fs`, `shell`, `process`, `http`, `websocket` permission 추가

## 3. 검증 결과

판정: PASS

실행한 명령:

```bash
cargo fmt --check
cargo test
npm test -- --run
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
codex --version
codex login status
codex app-server --listen stdio://
npm run dev
npm run desktop
```

결과 요약:

- `cargo fmt --check`: 통과
- `cargo test`: 전체 Rust test suite 통과
- `npm test -- --run`: 2개 test file, 13개 test 통과
- `npm run build`: TypeScript check 및 Vite production build 통과
- `cargo check --manifest-path src-tauri/Cargo.toml`: 통과
- `codex --version`: `codex-cli 0.130.0`
- `codex login status`: `Logged in using ChatGPT`
- `codex app-server --listen stdio://`: `account/read`, `model/list`, `account/rateLimits/read` 응답 확인
- `npm run dev`: `http://127.0.0.1:1420/` 기동 확인 후 종료
- `npm run desktop`: Tauri dev window 기동 확인 후 종료

## 4. 추가 테스트

신규 Rust 테스트:

- `tests/codex_bridge_tests.rs`

검증 항목:

- Codex CLI version parsing
- ChatGPT account metadata mapping
- API key / `chatgptAuthTokens` mode 차단
- app-server `model/list` mapping
- hidden/unavailable model handling
- `minimal` reasoning effort handling
- JSON-RPC response/notification/server-request classification
- `codex exec --json` JSONL draft extraction
- token-like field redaction/rejection
- process env allowlist
- app-server-first transport contract

기존 테스트 보강:

- `tests/app_core_tests.rs`
  - `minimal` reasoning effort 검증
  - `get_codex_bridge_status`, `poll_codex_chatgpt_login`, `start_agent_run`, `read_agent_run`, `cancel_agent_run` allowlist 검증

- `tests/tauri_shell_tests.rs`
  - Tauri build manifest와 capability가 신규 command를 포함하는지 검증
  - broad permission 금지 유지 검증

- `ui/src/lib/view-models.test.ts`
  - Codex 중심 startup label
  - `codex_auth_mode_unsupported` blocked reason
  - EN/KO `Connect Codex` copy parity

- `ui/src/lib/auth-flow.test.ts`
  - GitHub browser/device OAuth poll 조건 검증
  - Codex login poll 조건 검증
  - terminal auth state message 검증

## 5. 인증 런타임 디버그 메모

`npm run desktop`에서 GitHub 인증 버튼이 동작하지 않는 가장 흔한 원인은 `REVIEWDESK_GITHUB_CLIENT_ID`가 설정되지 않은 상태다. 현재 `start_github_oauth` command는 GitHub OAuth App client id가 없으면 `github_client_id_missing`으로 차단한다.

GitHub browser OAuth는 loopback callback을 사용한다. 앱은 인증 URL을 열고, callback 수신 후 token을 OS keychain에 저장하며, UI는 `poll_github_oauth`와 `refresh_github_auth_status`로 연결 상태를 다시 가져온다.

Codex 연결은 ReviewDesk 자체 ChatGPT OAuth가 아니다. UI는 Codex app-server/CLI가 제공하는 login flow를 시작하고, `poll_codex_chatgpt_login`과 `refresh_ai_account_status`로 Codex managed auth 상태를 갱신한다.

`npm run desktop`이 시작되지 않으면 이전 dev process가 `127.0.0.1:1420`을 점유 중인지 먼저 확인한다.

```bash
lsof -nP -iTCP:1420 -sTCP:LISTEN
pkill -f "tauri dev"
pkill -f "vite --host 127.0.0.1 --port 1420"
```

## 6. 보안 확인

확인된 보안 경계:

- `CodexProcessEnvPolicy`는 `PATH`, `HOME`, `USER`, `LOGNAME`, `SHELL`, `TMPDIR`, `LANG`만 Codex child process로 전달한다.
- `GITHUB_TOKEN`, `OPENAI_API_KEY`, arbitrary token-like env는 전달하지 않는다.
- Codex JSON-RPC payload에 `accessToken`, `refreshToken`, `Authorization`, `chatgptAuthTokens`, `auth.json`이 있으면 forbidden payload로 간주한다.
- diagnostics는 token-like key와 secret pattern을 redaction한다.
- Tauri capability는 ReviewDesk command만 허용한다.
- Agent Run 결과는 draft로만 저장되며 GitHub submit은 별도 Submit Safety Dock preflight가 필요하다.

## 7. 남은 제약

- 실제 GitHub OAuth E2E는 `REVIEWDESK_GITHUB_CLIENT_ID`와 사용자 승인 과정이 필요하므로 자동 테스트에서는 helper/API 단위로 검증했다.
- Codex Agent Run은 현재 `codex exec --json` fallback 경로로 실행한다. long-lived app-server turn streaming은 다음 버전에서 교체 가능한 구조로 남겨뒀다.
- 실제 PR diff를 Codex로 보내는 수동 E2E는 private repo consent와 사용자의 실제 PR 선택이 필요하다.
- Browser 자동화 도구가 이번 세션에 노출되지 않아 UI 클릭/스크린샷 검증은 `npm run build`와 Vite dev server startup 확인으로 대체했다.
- packaged app signing/notarization 검증은 이번 범위가 아니다.

## 8. 구현 판정

0.0.8은 ReviewDesk가 ChatGPT OAuth/token을 직접 구현하지 않고 Codex managed auth를 bridge로 사용하는 제품 경계를 구현했다.

현재 판정은 “0.0.8 Codex managed auth bridge, account/model/rate-limit source of truth, Agent Run draft-first execution, token boundary, IPC/capability 정합성 구현 및 검증 완료”다.
