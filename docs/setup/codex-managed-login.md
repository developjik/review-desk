# Codex Managed Login

## 0.0.9 경계

ReviewDesk는 ChatGPT OAuth를 직접 구현하지 않는다. AI 리뷰는 Codex CLI/App Server의 managed ChatGPT login 상태를 bridge로 읽고, 모델/추론 강도 선택값을 Agent Run snapshot에 저장한다.

ReviewDesk가 하지 않는 것:

- ChatGPT OAuth URL 생성
- ChatGPT access token 수신
- ChatGPT refresh token 저장
- `~/.codex/auth.json` 직접 파싱
- ChatGPT cookie/session scraping
- API key 입력을 기본 UX로 제공

ReviewDesk가 하는 것:

- Codex CLI/App Server discovery
- Codex login 시작 및 polling
- Codex account/model/rate-limit metadata 조회
- model/reasoning effort 선택
- private diff consent snapshot 저장
- Codex 결과를 ReviewDesk draft로 변환

## 사용자 흐름

1. 앱 시작 후 GitHub를 먼저 연결한다.
2. Codex 연결이 없으면 Review Inbox와 manual draft는 계속 사용할 수 있다.
3. AI Agent Run 버튼은 Codex managed login, 모델 목록, rate limit, private diff consent가 통과해야 활성화된다.
4. 모델과 추론 강도는 run record에 저장된다.

## 실패 상태

대표 blocked reason:

- `codex_chatgpt_auth_required`
- `codex_chatgpt_login_pending`
- `codex_chatgpt_expired`
- `codex_chatgpt_refresh_failed`
- `codex_auth_mode_unsupported`
- `model_list_unavailable`
- `model_unavailable`
- `reasoning_effort_unavailable`
- `codex_chatgpt_rate_limited`
- `codex_chatgpt_credits_depleted`

이 값은 UI에서 사용자 행동으로 번역되어야 하며 token-like diagnostic을 그대로 보여주면 안 된다.
