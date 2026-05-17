# ReviewDesk Desktop Runbook

## 기준 버전

- 대상: ReviewDesk 0.1.0
- 앱 형태: Rust core + Tauri desktop shell + React UI
- 기본 실행 명령: `npm run desktop`

## 개발 실행

```bash
npm install
export REVIEWDESK_GITHUB_CLIENT_ID="<github-oauth-client-id>"
export REVIEWDESK_OAUTH_BROKER_URL="https://<reviewdesk-oauth-broker>"
npm run desktop
```

`npm run desktop`은 Tauri dev server와 Vite UI를 한 번에 실행한다. 별도 터미널에서 `npm run dev`를 직접 실행할 필요는 없다.

## Browser OAuth 조건

GitHub browser OAuth는 desktop app에 client secret을 넣지 않기 위해 ReviewDesk OAuth Broker가 필요하다.

- `REVIEWDESK_GITHUB_CLIENT_ID`: GitHub OAuth App client id
- `REVIEWDESK_OAUTH_BROKER_URL`: HTTPS broker origin
- desktop app은 loopback listener를 열고 broker가 발급한 one-time `broker_code`만 받는다.
- GitHub access token은 `/user` 검증 후 OS keychain에 저장한다.

broker가 없거나 개발 환경에서 즉시 테스트해야 하면 앱의 GitHub device code flow를 사용한다.

## 패키징

```bash
npm run build
npm run desktop:build
```

Tauri manifest는 0.1.0 기준으로 bundle이 활성화되어 있고 macOS `.app` bundle을 생성한다. DMG 생성은 Finder automation 의존성이 있어 0.1.0 자동 검증 경로에서는 제외한다.

## 자주 막히는 경우

- GitHub 연결 후 UI가 그대로이면 `refresh_github_auth_status`가 `/user` 검증에서 실패한 상태다.
- `github_broker_missing`이면 `REVIEWDESK_OAUTH_BROKER_URL`이 없다.
- browser OAuth가 열리지 않으면 broker URL이 HTTPS인지 확인한다.
- `github_scope_insufficient`이면 review submit에 필요한 repo/write 권한이 부족하다.
- `github_sso_required`이면 GitHub organization SSO 승인이 필요하다.
- Codex가 missing이어도 기본 모드에서는 Review Inbox와 manual draft는 사용할 수 있다.
