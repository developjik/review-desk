# GitHub OAuth Setup

## 0.0.9 결정

ReviewDesk desktop app은 GitHub OAuth client secret을 저장하지 않는다. Browser OAuth 기본 UX는 ReviewDesk OAuth Broker가 client secret을 소유하고, desktop app은 broker one-time code를 redeem하는 방식이다.

## GitHub OAuth App 설정

권장 설정:

- Application name: ReviewDesk
- Homepage URL: ReviewDesk 배포 또는 문서 URL
- Authorization callback URL: `https://<reviewdesk-oauth-broker>/auth/github/callback`
- Device Flow: 개발/복구 fallback을 위해 활성화

필요 권한:

- `read:user`
- `repo`

0.0.9 구현은 GitHub OAuth App scope 모델을 기준으로 한다. fine-grained token UX는 장기 후보이며, 현재 first-run 기본 흐름은 아니다.

## Browser OAuth 흐름

```text
Desktop -> Broker -> GitHub authorize
GitHub -> Broker callback
Broker -> GitHub token exchange
Broker -> Desktop loopback broker_code
Desktop -> Broker redeem
Desktop -> GitHub /user validate
Desktop -> OS keychain save
```

desktop app은 다음을 하지 않는다.

- client secret 저장
- client secret UI 입력
- GitHub authorization code 직접 token exchange
- token-like 값을 diagnostics나 draft에 출력

## Device Flow fallback

broker를 사용할 수 없는 개발 환경에서는 device flow를 사용한다.

필요 조건:

- `REVIEWDESK_GITHUB_CLIENT_ID` 설정
- GitHub OAuth App에서 Device Flow 활성화

앱은 `authorization_pending`, `slow_down`, `expired_token`, `access_denied` 상태를 polling 결과로 정규화한다.

## 연결 판정

0.0.9에서 keychain token 존재는 connected가 아니다. 앱은 token이 있으면 GitHub `/user`를 호출해 다음 상태로 정규화한다.

- `connected`
- `missing`
- `expired`
- `scope_missing`
- `sso_required`
- `rate_limited`
- `reauth_required`
