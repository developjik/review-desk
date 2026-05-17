use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use reviewdesk::domain::ReviewEvent;
use reviewdesk::github::{
    CiRollupState, DevicePoll, GitHubClient, ReviewSubmitRequest, StaleCheckInput, StaleStatus,
    assigned_query, build_github_authorize_url, parse_pr_reference, pkce_challenge_s256,
    review_requested_query,
};

#[test]
fn builds_review_queue_search_queries() {
    assert_eq!(
        review_requested_query("developjik", Some("company/payment-web")),
        "repo:company/payment-web is:pr is:open archived:false user-review-requested:@me"
    );
    assert_eq!(
        assigned_query("developjik", None),
        "is:pr is:open archived:false assignee:@me"
    );
}

#[test]
fn parses_pr_reference_fallback_inputs() {
    assert_eq!(
        parse_pr_reference("https://github.com/company/payment-web/pull/582")
            .unwrap()
            .to_string(),
        "company/payment-web#582"
    );
    assert_eq!(
        parse_pr_reference("company/payment-web#582")
            .unwrap()
            .to_string(),
        "company/payment-web#582"
    );
    assert_eq!(
        parse_pr_reference("company/payment-web/pull/582")
            .unwrap()
            .to_string(),
        "company/payment-web#582"
    );
    assert!(parse_pr_reference("https://gitlab.com/company/payment-web/pull/582").is_err());
}

#[tokio::test]
async fn starts_device_flow_with_client_id_and_scopes() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST)
            .path("/login/device/code")
            .body_includes("client_id=client-id")
            .body_includes("scope=public_repo");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "device_code": "device",
                "user_code": "ABCD-1234",
                "verification_uri": "https://github.com/login/device",
                "expires_in": 900,
                "interval": 5
            }));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");
    let flow = client
        .start_device_flow("client-id", &["public_repo"])
        .await
        .expect("device flow");

    assert_eq!(flow.device_code, "device");
    assert_eq!(flow.interval, 5);
}

#[test]
fn github_browser_oauth_authorize_url_uses_https_broker_pkce_and_state() {
    let authorize_url = build_github_authorize_url(
        "client-id",
        "https://broker.reviewdesk.dev/auth/github/callback",
        &["read:user", "repo"],
        "state-value",
        "challenge-value",
    )
    .expect("authorize url");

    assert_eq!(authorize_url.host_str(), Some("github.com"));
    assert_eq!(authorize_url.path(), "/login/oauth/authorize");

    let query = authorize_url
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        query.get("client_id").map(|value| value.as_ref()),
        Some("client-id")
    );
    assert_eq!(
        query.get("redirect_uri").map(|value| value.as_ref()),
        Some("https://broker.reviewdesk.dev/auth/github/callback")
    );
    assert_eq!(
        query.get("scope").map(|value| value.as_ref()),
        Some("read:user repo")
    );
    assert_eq!(
        query.get("state").map(|value| value.as_ref()),
        Some("state-value")
    );
    assert_eq!(
        query.get("code_challenge").map(|value| value.as_ref()),
        Some("challenge-value")
    );
    assert_eq!(
        query
            .get("code_challenge_method")
            .map(|value| value.as_ref()),
        Some("S256")
    );
}

#[test]
fn github_browser_oauth_rejects_non_https_broker_redirect_uri() {
    let result = build_github_authorize_url(
        "client-id",
        "http://broker.reviewdesk.dev/auth/github/callback",
        &["read:user"],
        "state",
        "challenge",
    );

    assert!(result.is_err());
}

#[test]
fn github_browser_oauth_pkce_challenge_is_url_safe_sha256() {
    let challenge = pkce_challenge_s256("abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJK");

    assert_eq!(challenge.len(), 43);
    assert!(!challenge.contains('='));
    assert!(
        challenge
            .chars()
            .all(|character| character.is_ascii_alphanumeric()
                || character == '-'
                || character == '_')
    );
}

#[tokio::test]
async fn github_broker_redeems_one_time_code_without_client_secret() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST)
            .path("/oauth/github/redeem")
            .body_includes("broker_code=broker-code")
            .body_includes("code_verifier=verifier-value")
            .body_excludes("client_secret");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "access_token": "gho_abcdefghijklmnopqrstuvwxyz123456",
                "token_type": "bearer",
                "scope": "read:user,repo"
            }));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "");
    let token = client
        .redeem_broker_oauth_code(&server.url(""), "broker-code", "verifier-value")
        .await
        .expect("broker token");

    assert_eq!(token.access_token, "gho_abcdefghijklmnopqrstuvwxyz123456");
    assert_eq!(token.scope.as_deref(), Some("read:user,repo"));
}

#[tokio::test]
async fn github_browser_oauth_maps_provider_error_without_token() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/login/oauth/access_token");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "error": "bad_verification_code",
                "error_description": "The code passed is incorrect or expired."
            }));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "");
    let error = client
        .exchange_browser_oauth_code(
            "client-id",
            None,
            "bad-code",
            "http://127.0.0.1:49152/auth/github/callback",
            "verifier-value",
        )
        .await
        .expect_err("provider error");

    assert!(error.to_string().contains("bad_verification_code"));
}

#[tokio::test]
async fn fetches_current_user_and_accessible_repositories() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/user");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({"login": "developjik"}));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/user/repos")
            .query_param("visibility", "all")
            .query_param("affiliation", "owner,collaborator,organization_member")
            .query_param("sort", "updated")
            .query_param("per_page", "100")
            .query_param_missing("page");
        then.status(200)
            .header("content-type", "application/json")
            .header(
                "link",
                &format!(
                    "<{}/user/repos?visibility=all&affiliation=owner%2Ccollaborator%2Corganization_member&sort=updated&per_page=100&page=2>; rel=\"next\"",
                    server.url("")
                ),
            )
            .json_body_obj(&serde_json::json!([
                {"name": "payment-web", "full_name": "company/payment-web", "private": true}
            ]));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/user/repos")
            .query_param("visibility", "all")
            .query_param("affiliation", "owner,collaborator,organization_member")
            .query_param("sort", "updated")
            .query_param("per_page", "100")
            .query_param("page", "2");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!([
                {"name": "design-system", "full_name": "company/design-system", "private": false}
            ]));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");

    assert_eq!(client.current_user().await.unwrap().login, "developjik");

    let repositories = client.list_repositories().await.unwrap();
    assert_eq!(repositories.len(), 2);
    assert_eq!(repositories[0].full_name, "company/payment-web");
    assert!(repositories[0].private);
}

#[tokio::test]
async fn searches_review_requested_and_assigned_pr_queue() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/search/issues")
            .query_param(
                "q",
                "repo:company/payment-web is:pr is:open archived:false user-review-requested:@me",
            )
            .query_param("sort", "updated")
            .query_param("order", "desc")
            .query_param("per_page", "100")
            .query_param("page", "1");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "items": [
                    {
                        "number": 582,
                        "title": "Payment refactor",
                        "html_url": "https://github.com/company/payment-web/pull/582",
                        "repository_url": "https://api.github.com/repos/company/payment-web",
                        "user": {"login": "alice"},
                        "updated_at": "2026-05-16T01:02:03Z"
                    }
                ],
                "incomplete_results": false
            }));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/search/issues")
            .query_param(
                "q",
                "repo:company/payment-web is:pr is:open archived:false assignee:@me",
            )
            .query_param("sort", "updated")
            .query_param("order", "desc")
            .query_param("per_page", "100")
            .query_param("page", "1");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "items": [
                    {
                        "number": 590,
                        "title": "Button color",
                        "html_url": "https://github.com/company/payment-web/pull/590",
                        "repository_url": "https://api.github.com/repos/company/payment-web",
                        "user": {"login": "bob"},
                        "updated_at": "2026-05-16T01:03:03Z"
                    }
                ],
                "incomplete_results": false
            }));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");
    let queue = client
        .review_queue("developjik", Some("company/payment-web"))
        .await
        .unwrap();

    assert_eq!(queue.len(), 2);
    assert!(queue.iter().any(|item| item.review_requested));
    assert!(queue.iter().any(|item| item.assigned));
}

#[tokio::test]
async fn paginates_review_queue_queries_independently_and_merges_reason_both() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/search/issues")
            .query_param(
                "q",
                "is:pr is:open archived:false user-review-requested:@me",
            )
            .query_param("page", "1");
        then.status(200)
            .header("content-type", "application/json")
            .header(
                "link",
                &format!("<{}/search/issues?page=2>; rel=\"next\"", server.url("")),
            )
            .json_body_obj(&serde_json::json!({
                "incomplete_results": false,
                "items": [{
                    "number": 7,
                    "title": "Queue item",
                    "html_url": "https://github.com/company/repo/pull/7",
                    "repository_url": "https://api.github.com/repos/company/repo",
                    "user": {"login": "alice"},
                    "updated_at": "2026-05-16T01:02:03Z"
                }]
            }));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/search/issues")
            .query_param(
                "q",
                "is:pr is:open archived:false user-review-requested:@me",
            )
            .query_param("page", "2");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "incomplete_results": false,
                "items": []
            }));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/search/issues")
            .query_param("q", "is:pr is:open archived:false assignee:@me")
            .query_param("page", "1");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "incomplete_results": false,
                "items": [{
                    "number": 7,
                    "title": "Queue item",
                    "html_url": "https://github.com/company/repo/pull/7",
                    "repository_url": "https://api.github.com/repos/company/repo",
                    "user": {"login": "alice"},
                    "updated_at": "2026-05-16T01:02:03Z"
                }]
            }));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");
    let queue = client.review_queue("developjik", None).await.unwrap();

    assert_eq!(queue.len(), 1);
    assert!(queue[0].review_requested);
    assert!(queue[0].assigned);
    assert_eq!(queue[0].review_reason.as_deref(), Some("both"));
}

#[tokio::test]
async fn collects_pull_snapshot_and_related_context() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/pulls/582");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "number": 582,
                "title": "Payment refactor",
                "html_url": "https://github.com/company/payment-web/pull/582",
                "state": "open",
                "merged": false,
                "draft": false,
                "user": {"login": "alice"},
                "base": {"ref": "main"},
                "head": {"ref": "feature/payment", "sha": "head-sha"},
                "additions": 12,
                "deletions": 4,
                "changed_files": 1
            }));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/pulls/582/files")
            .query_param_missing("page");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!([
                {"filename": "src/payment.rs", "patch": "@@ patch", "status": "modified", "additions": 12, "deletions": 4}
            ]));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/issues/582/comments");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!([]));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/pulls/582/reviews");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!([]));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/pulls/582/comments");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!([]));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/commits/head-sha/check-runs");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "check_runs": [
                    {"name": "test", "status": "completed", "conclusion": "success"}
                ]
            }));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/commits/head-sha/status");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "state": "success",
                "statuses": [
                    {"context": "legacy-ci", "state": "success", "description": "ok"}
                ]
            }));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");
    let context = client
        .collect_pull_request_context_view("company", "payment-web", 582)
        .await
        .unwrap();

    assert_eq!(context.pr.head_sha, "head-sha");
    assert_eq!(context.files[0].path, "src/payment.rs");
    assert_eq!(context.files[0].patch_coverage, "available");
    assert_eq!(context.ci.state, CiRollupState::Success);
    assert_eq!(context.patch_coverage, "complete");
    assert!(!context.diff_hash.is_empty());
    assert!(!context.context_hash.is_empty());
}

#[test]
fn detects_stale_or_closed_pr_before_submit() {
    let stale = StaleCheckInput {
        draft_head_sha: "old".to_string(),
        current_head_sha: "new".to_string(),
        state: "open".to_string(),
        merged: false,
    }
    .evaluate();

    assert!(stale.is_stale);
    assert_eq!(stale.status, StaleStatus::HeadChanged);

    let closed = StaleCheckInput {
        draft_head_sha: "same".to_string(),
        current_head_sha: "same".to_string(),
        state: "closed".to_string(),
        merged: false,
    }
    .evaluate();

    assert!(closed.is_stale);
    assert_eq!(closed.status, StaleStatus::PrClosed);
}

#[tokio::test]
async fn handles_device_flow_slow_down() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/login/oauth/access_token");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({
                "error": "slow_down",
                "error_description": "slow down"
            }));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");
    let result = client
        .poll_device_token("client-id", "device-code")
        .await
        .expect("poll result");

    assert_eq!(result, DevicePoll::SlowDown);
}

#[tokio::test]
async fn fetches_issue_comments_for_pr_conversation() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/issues/582/comments");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!([
                {"id": 1, "body": "top-level discussion", "user": {"login": "alice"}}
            ]));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");
    let comments = client
        .list_issue_comments("company", "payment-web", 582)
        .await
        .expect("issue comments");

    assert_eq!(comments[0].body, "top-level discussion");
}

#[tokio::test]
async fn follows_pagination_for_pr_files() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/pulls/582/files")
            .query_param_missing("page");
        then.status(200)
            .header("content-type", "application/json")
            .header(
                "link",
                &format!(
                    "<{}/repos/company/payment-web/pulls/582/files?page=2>; rel=\"next\"",
                    server.url("")
                ),
            )
            .json_body_obj(&serde_json::json!([
                {"filename": "src/a.rs", "patch": "@@ a", "status": "modified", "additions": 1, "deletions": 0}
            ]));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/repos/company/payment-web/pulls/582/files")
            .query_param("page", "2");
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!([
                {"filename": "src/b.rs", "patch": "@@ b", "status": "added", "additions": 2, "deletions": 0}
            ]));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");
    let files = client
        .list_pull_files("company", "payment-web", 582)
        .await
        .expect("files");

    assert_eq!(files.len(), 2);
    assert_eq!(files[1].filename, "src/b.rs");
}

#[tokio::test]
async fn submits_top_level_review_payload() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST)
            .path("/repos/company/payment-web/pulls/582/reviews")
            .json_body_obj(&serde_json::json!({
                "commit_id": "sha",
                "event": "COMMENT",
                "body": "review body"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body_obj(&serde_json::json!({"id": 99}));
    });

    let client = GitHubClient::for_test(server.url(""), server.url(""), "token");
    let review = client
        .submit_review(
            "company",
            "payment-web",
            582,
            &ReviewSubmitRequest {
                commit_id: "sha".to_string(),
                event: ReviewEvent::Comment,
                body: "review body".to_string(),
            },
        )
        .await
        .expect("review submit");

    assert_eq!(review.id, 99);
}
