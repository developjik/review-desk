# OAuth Secret Policy

## 원칙

ReviewDesk desktop app은 OAuth secret owner가 아니다.

- GitHub `client_id`: desktop env/config에 둘 수 있다.
- GitHub `client_secret`: desktop app, repo, UI, log, diagnostics, packaged bundle에 넣지 않는다.
- GitHub access token: OS keychain에만 저장한다.
- ChatGPT/Codex token: ReviewDesk가 직접 소유하지 않는다.

## GitHub Browser OAuth

Browser OAuth는 ReviewDesk OAuth Broker가 client secret을 소유한다. desktop app은 broker URL과 client id만 알고, broker가 발급한 one-time code를 redeem한다.

금지:

- `REVIEWDESK_GITHUB_CLIENT_SECRET`
- client secret 입력창
- client secret 문서화 예시값
- token exchange를 desktop binary에서 직접 수행

## Redaction 대상

다음 패턴은 UI, log, report, diagnostics, draft storage에서 마스킹되어야 한다.

- `client_secret`
- `access_token`
- `refresh_token`
- `Authorization`
- `Bearer `
- `gho_`
- `ghp_`
- `github_pat_`
- `sk-`
- `chatgptAuthTokens`
- `auth.json`
- `code_verifier`
- OAuth `code`

## Secret scan

릴리즈 전에는 다음 형태의 scan을 수행한다.

```bash
rg -n "client_secret|Authorization|Bearer |gho_|ghp_|github_pat_|sk-|access_token|refresh_token|chatgptAuthTokens|auth\\.json" \
  docs src src-tauri ui tests dist src-tauri/target/release/bundle .reviewdesk
```

문서의 정책 설명에서 패턴 이름이 잡히는 것은 allowlist로 기록하고, 실제 secret-like 값이 있으면 릴리즈 차단이다.
