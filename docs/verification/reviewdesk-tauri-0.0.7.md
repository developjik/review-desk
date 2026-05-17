# ReviewDesk Tauri 0.0.7 Verification

## 1. 검증 대상

검증 대상은 `docs/prd/prd-0.0.7.md` 기반 구현이다.

0.0.7 구현 범위:

- GitHub OAuth 기본 경로를 Device Flow에서 browser OAuth + loopback callback으로 변경
- GitHub Authorization Code + PKCE helper, authorize URL 생성, token exchange helper 추가
- loopback redirect URI를 `127.0.0.1` 또는 `::1` + explicit port로 제한
- Device Flow를 GitHub fallback mode로 유지
- Tauri command에 `start_github_oauth(mode)`, `poll_github_oauth`, `cancel_github_oauth`, `logout_github`, `refresh_github_auth_status` 연결
- Tauri capability/build manifest/IPC allowlist에 새 command 반영
- OAuth code, verifier, client secret, bearer token, GitHub token-like payload redaction 보강
- React startup auth gate를 browser sign-in CTA 중심으로 변경
- GitHub device fallback, ChatGPT device fallback, `Continue without AI` CTA 분리
- `GPT` 단독 표기를 UI에서 제거하고 `Codex ChatGPT`로 정리
- EN/KO i18n copy와 version metadata를 `0.0.7`로 갱신

## 2. 검증 결과

판정: PASS

실행한 명령:

```bash
cargo fmt --check
npm test -- --run
npm run build
cargo test
cargo test --test app_core_tests ipc_allowlist_contains_prd_commands_and_marks_submit_as_high_risk
cargo test --test github_tests github_browser_oauth
cargo test --test security_tests masks_github_oauth_browser_flow_secrets
cargo test --test tauri_shell_tests
cargo check --manifest-path src-tauri/Cargo.toml
```

결과 요약:

- `npm test -- --run`: 1개 test file, 9개 test 통과
- `npm run build`: TypeScript check 및 Vite production build 통과
- `cargo fmt --check`: 통과
- `cargo test`: 전체 Rust test suite 통과
- `github_browser_oauth` targeted tests: 5개 통과
- `masks_github_oauth_browser_flow_secrets`: 통과
- `tauri_shell_tests`: 4개 통과
- `cargo check --manifest-path src-tauri/Cargo.toml`: 통과

## 3. 보안 확인

확인된 보안 경계:

- React는 GitHub access token, OAuth code, state 원문, PKCE verifier, Authorization header를 받지 않는다.
- GitHub browser OAuth token exchange는 Rust GitHub client 경계에서 처리한다.
- loopback callback redirect URI는 loopback host와 explicit port만 허용한다.
- `client_secret`은 필수가 아니며 public-client PKCE 경로를 지원한다.
- Tauri capability는 ReviewDesk command만 허용하고 broad `fs`, `shell`, `http`, `process` 권한을 열지 않는다.
- OAuth 관련 secret-like 문자열은 security masker 테스트로 검증했다.

## 4. 남은 제약

- 실제 GitHub OAuth end-to-end 인증은 `REVIEWDESK_GITHUB_CLIENT_ID`와 사용자의 GitHub 승인 과정이 필요하므로 자동 테스트에서는 mock exchange/helper 단위로 검증했다.
- Codex ChatGPT는 0.0.6의 managed-auth boundary를 유지한다. 공식 Codex App Server transport가 없는 환경에서는 fake success를 만들지 않고 unsupported/blocked 상태를 표시한다.
- packaged app signing/notarization 검증은 이번 범위가 아니다.
- GitHub inline comment submit, auto-submit, OAuth broker/GitHub App 전환은 0.0.7 범위가 아니다.

## 5. 구현 판정

0.0.7은 PRD의 핵심 요구인 browser OAuth first UX, GitHub loopback callback foundation, Device Flow fallback, token 미노출, Tauri command/capability 정합성, EN/KO auth copy 갱신을 구현했다.

따라서 현재 판정은 “0.0.7 제품 계약 구현 완료, 실제 GitHub 계정 OAuth E2E는 client id 설정 후 수동 검증 필요”다.
