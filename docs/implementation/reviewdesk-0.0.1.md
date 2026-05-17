# ReviewDesk 0.0.1 구현 노트

작성일: 2026-05-16

## 구현 범위

ReviewDesk 0.0.1은 Rust native desktop 앱과 테스트 가능한 core library로 구현했다. 제품 방향은 웹/Tauri가 아니라 `egui`/`eframe` 기반 단일 창 데스크톱 앱이다.

구현된 주요 경계:

- Desktop shell: `src/main.rs`, `src/app.rs`
- Domain model: `src/domain.rs`
- GitHub OAuth/token 경계: `src/auth.rs`, `src/github.rs`
- GitHub REST client: repository 목록, review requested/assigned PR 검색, PR snapshot/context 수집, changed files pagination, comments/reviews, check runs, commit status, top-level review submit
- AI provider adapter: `BlockedAiProvider`, `MockAiProvider`, `CodexAppServerProvider` 경계
- Review pipeline: diff filtering, deterministic input, AI blocked fallback, markdown report 생성
- Local storage: `.reviewdesk/` owner-only 생성, `.gitignore` 보호, JSON/text 저장
- Security baseline: secret masking, private diff consent, path traversal 방지

## PRD 매핑

| PRD 항목 | 구현 상태 | 코드/검증 |
| --- | --- | --- |
| Rust desktop app | 구현 | `src/main.rs`, `src/app.rs`, `cargo check` |
| GitHub OAuth Device Flow | 구현 | `GitHubClient::start_device_flow`, `poll_device_token`, `tests/github_tests.rs` |
| OS keychain token 저장 경계 | 구현 | `KeyringTokenStore`, `MemoryTokenStore`, `tests/auth_tests.rs` |
| Repository 선택 기반 흐름 | Core API 구현, UI shell 제공 | `list_repositories`, `review_queue`, `ReviewDeskApp` |
| review requested/assigned PR 목록 | 구현 | `review_queue`, query tests |
| PR URL/reference fallback | 구현 | `parse_pr_reference` |
| PR context 수집 | 구현 | `collect_pull_request_context` |
| changed files pagination | 구현 | `list_pull_files`, pagination test |
| CI/check status 요약 수집 | 구현 | `list_check_runs`, `commit_status` |
| 기존 대화/리뷰 수집 | 구현 | `list_issue_comments`, `list_pull_reviews`, `list_pull_review_comments` |
| secret masking | 구현 | `SecretMasker`, `tests/security_tests.rs` |
| private diff consent | 구현 | `PrivateDiffConsent` |
| AI OAuth-only 원칙 | 구현 | API key provider 없음, unsupported 공식 provider는 blocked 처리 |
| GPT 모델/추론 깊이 선택 | UI 및 provider 경계 구현 | `AiProvider`, `ReasoningDepth`, `ReviewDeskApp` |
| AI unavailable fallback | 구현 | `AI_BLOCKED_UNSUPPORTED_AUTH`, blocked pipeline tests |
| Draft/report 로컬 저장 | 구현 | `LocalStore::save_json`, `save_text` |
| Submit 전 stale check | 구현 | `StaleCheckInput::evaluate` |
| top-level review 제출 | 구현 | `GitHubClient::submit_review` |
| inline comment 제출 | 제외 | PRD 0.0.1 제외 범위 |
| 주기적 polling/watch | 제외 | PRD 0.0.1 제외 범위 |

## AI Provider 전제

0.0.1은 사용자의 요구대로 OpenAI API key 입력을 제공하지 않는다. 공식 ChatGPT OAuth 기반 model invocation 경로가 준비되지 않은 환경에서는 `AI_BLOCKED_UNSUPPORTED_AUTH`가 정상 동작이다.

현재 `CodexAppServerProvider`는 공식 provider와 연결하기 위한 adapter boundary다. 실제 ChatGPT OAuth review 생성은 공식 provider endpoint와 모델 목록 API가 제공되는 환경에서만 활성화한다. 비공식 cookie/session 추출, 브라우저 자동화, API key fallback은 구현하지 않았다.

## 검증

최종 검증 결과:

```bash
$ cargo fmt --check
# pass

$ cargo check
# pass

$ cargo test
# pass: auth 1, github 11, review 4, security 4
```

검증 대상 테스트:

- `tests/auth_tests.rs`: OAuth token store 경계
- `tests/github_tests.rs`: Device Flow, PR reference parsing, repository/PR queue/context 수집, pagination, stale check, submit payload
- `tests/review_tests.rs`: AI blocked fallback, mock AI report, review pipeline
- `tests/security_tests.rs`: secret masking, private diff consent, local storage protection

## 운영 전제

실제 GitHub OAuth 사용에는 별도 GitHub OAuth App Client ID가 필요하다. Device Flow가 활성화된 OAuth App을 등록하고 앱 설정에 Client ID를 주입해야 한다.

실제 GitHub private repository review 제출에는 조직 SSO 승인, OAuth App 조직 승인, 적절한 `repo` 또는 `public_repo` scope가 필요할 수 있다. 이 경우 GitHub API 오류는 `GitHubErrorKind`로 분류해 UI에서 조치 가능한 메시지로 연결한다.
