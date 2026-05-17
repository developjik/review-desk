use crate::domain::{
    ChangedFile, GitHubErrorKind, PullRequestQueueItem, PullRequestSnapshot, Repository, Result,
    ReviewDeskError, ReviewEvent,
};
use crate::review::stable_changed_files_hash;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, LINK, RETRY_AFTER};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;

pub fn review_requested_query(username: &str, repo: Option<&str>) -> String {
    let _ = username;
    scoped_query("user-review-requested:@me", repo)
}

pub fn assigned_query(username: &str, repo: Option<&str>) -> String {
    let _ = username;
    scoped_query("assignee:@me", repo)
}

fn scoped_query(qualifier: &str, repo: Option<&str>) -> String {
    match repo {
        Some(repo) => format!("repo:{repo} is:pr is:open archived:false {qualifier}"),
        None => format!("is:pr is:open archived:false {qualifier}"),
    }
}

#[derive(Debug, Clone)]
pub struct GitHubClient {
    api_base: String,
    login_base: String,
    token: Option<String>,
    client: reqwest::Client,
}

impl GitHubClient {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            api_base: "https://api.github.com".to_string(),
            login_base: "https://github.com".to_string(),
            token: Some(token.into()),
            client: reqwest::Client::new(),
        }
    }

    pub fn for_test(api_base: String, login_base: String, token: &str) -> Self {
        Self {
            api_base,
            login_base,
            token: Some(token.to_string()),
            client: reqwest::Client::new(),
        }
    }

    pub async fn current_user(&self) -> Result<GitHubUser> {
        self.get_json(self.api_url("/user")).await
    }

    pub async fn list_repositories(&self) -> Result<Vec<Repository>> {
        let repositories = self
            .get_paginated_json::<RepositoryResponse>(self.api_url(
                "/user/repos?visibility=all&affiliation=owner,collaborator,organization_member&sort=updated&per_page=100",
            ))
            .await?;
        Ok(repositories
            .into_iter()
            .map(|repository| Repository {
                owner: repository
                    .full_name
                    .split_once('/')
                    .map(|(owner, _)| owner.to_string())
                    .unwrap_or_default(),
                repo: repository.name,
                full_name: repository.full_name,
                private: repository.private,
                last_refreshed_at: Some(Utc::now()),
            })
            .collect())
    }

    pub async fn review_queue(
        &self,
        username: &str,
        repo: Option<&str>,
    ) -> Result<Vec<PullRequestQueueItem>> {
        let mut by_key: HashMap<(String, String, u64), PullRequestQueueItem> = HashMap::new();
        for (query, review_requested, assigned) in [
            (review_requested_query(username, repo), true, false),
            (assigned_query(username, repo), false, true),
        ] {
            for item in self
                .search_pull_requests(&query, review_requested, assigned)
                .await?
            {
                by_key
                    .entry((item.owner.clone(), item.repo.clone(), item.number))
                    .and_modify(|existing| {
                        existing.review_requested |= item.review_requested;
                        existing.assigned |= item.assigned;
                        existing.review_reason =
                            Some(review_reason(existing.review_requested, existing.assigned));
                    })
                    .or_insert(item);
            }
        }

        let mut items = by_key.into_values().collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.full_pr_ref().cmp(&right.full_pr_ref()))
        });
        Ok(items)
    }

    pub async fn list_repository_open_pulls(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequestQueueItem>> {
        let url = self.api_url(&format!("/repos/{owner}/{repo}/pulls?state=open"));
        let pulls = self.get_paginated_json::<PullRequestResponse>(url).await?;
        Ok(pulls
            .into_iter()
            .map(|pull| pull.into_queue_item(owner, repo, false, false))
            .collect())
    }

    pub async fn redeem_broker_oauth_code(
        &self,
        broker_base: &str,
        broker_code: &str,
        code_verifier: &str,
    ) -> Result<BrowserOAuthToken> {
        let url = format!("{}/oauth/github/redeem", broker_base.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .header("accept", "application/json")
            .form(&[
                ("broker_code", broker_code.to_string()),
                ("code_verifier", code_verifier.to_string()),
            ])
            .send()
            .await?;
        self.decode_response(response).await
    }

    pub async fn start_device_flow(
        &self,
        client_id: &str,
        scopes: &[&str],
    ) -> Result<DeviceFlowStart> {
        let url = format!(
            "{}/login/device/code",
            self.login_base.trim_end_matches('/')
        );
        let scope = scopes.join(" ");
        let response = self
            .client
            .post(url)
            .header("accept", "application/json")
            .form(&[("client_id", client_id.to_string()), ("scope", scope)])
            .send()
            .await?;
        self.decode_response(response).await
    }

    pub async fn poll_device_token(
        &self,
        client_id: &str,
        device_code: &str,
    ) -> Result<DevicePoll> {
        let url = format!(
            "{}/login/oauth/access_token",
            self.login_base.trim_end_matches('/')
        );
        let response = self
            .client
            .post(url)
            .header("accept", "application/json")
            .form(&serde_json::json!({
                "client_id": client_id,
                "device_code": device_code,
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
            }))
            .send()
            .await?;
        let value: serde_json::Value = response.json().await?;

        if let Some(error) = value.get("error").and_then(|value| value.as_str()) {
            return Ok(match error {
                "authorization_pending" => DevicePoll::Pending,
                "slow_down" => DevicePoll::SlowDown,
                other => DevicePoll::Denied(other.to_string()),
            });
        }

        if let Some(token) = value.get("access_token").and_then(|value| value.as_str()) {
            return Ok(DevicePoll::Authorized(token.to_string()));
        }

        Ok(DevicePoll::Denied("missing access_token".to_string()))
    }

    pub async fn exchange_browser_oauth_code(
        &self,
        client_id: &str,
        client_secret: Option<&str>,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<BrowserOAuthToken> {
        let url = format!(
            "{}/login/oauth/access_token",
            self.login_base.trim_end_matches('/')
        );
        let mut form = vec![
            ("client_id", client_id.to_string()),
            ("code", code.to_string()),
            ("redirect_uri", redirect_uri.to_string()),
            ("code_verifier", code_verifier.to_string()),
        ];
        if let Some(secret) = client_secret.filter(|secret| !secret.is_empty()) {
            form.push(("client_secret", secret.to_string()));
        }

        let response = self
            .client
            .post(url)
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await?;
        let status = response.status();
        let headers = response.headers().clone();
        let value: serde_json::Value = response.json().await?;

        if !status.is_success() {
            return Err(ReviewDeskError::GitHubApi {
                kind: classify_github_error(status.as_u16(), &headers, &value.to_string()),
                message: value.to_string(),
            });
        }

        if let Some(error) = value.get("error").and_then(|value| value.as_str()) {
            let description = value
                .get("error_description")
                .and_then(|value| value.as_str())
                .unwrap_or("GitHub OAuth failed");
            return Err(ReviewDeskError::GitHubApi {
                kind: GitHubErrorKind::AuthRequired,
                message: format!("{error}: {description}"),
            });
        }

        let access_token = value
            .get("access_token")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ReviewDeskError::GitHubApi {
                kind: GitHubErrorKind::AuthRequired,
                message: "missing access_token".to_string(),
            })?
            .to_string();

        Ok(BrowserOAuthToken {
            access_token,
            token_type: value
                .get("token_type")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned),
            scope: value
                .get("scope")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned),
        })
    }

    pub async fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Vec<IssueComment>> {
        let url = self.api_url(&format!(
            "/repos/{owner}/{repo}/issues/{issue_number}/comments"
        ));
        self.get_json(url).await
    }

    pub async fn pull_request_snapshot(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<PullRequestSnapshot> {
        let pull = self.pull_request(owner, repo, pull_number).await?;
        Ok(pull.into_snapshot(owner, repo))
    }

    pub async fn collect_pull_request_context(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<PullRequestContext> {
        let pull = self.pull_request(owner, repo, pull_number).await?;
        let head_sha = pull.head.sha.clone();
        let snapshot = pull.into_snapshot(owner, repo);
        let files = self
            .list_pull_files(owner, repo, pull_number)
            .await?
            .into_iter()
            .map(|file| ChangedFile {
                path: file.filename,
                patch: file.patch,
            })
            .collect();
        let issue_comments = self.list_issue_comments(owner, repo, pull_number).await?;
        let reviews = self.list_pull_reviews(owner, repo, pull_number).await?;
        let review_comments = self
            .list_pull_review_comments(owner, repo, pull_number)
            .await?;
        let check_runs = self.list_check_runs(owner, repo, &head_sha).await?;
        let commit_status = self.commit_status(owner, repo, &head_sha).await?;

        Ok(PullRequestContext {
            snapshot,
            files,
            issue_comments,
            reviews,
            review_comments,
            check_runs,
            commit_status,
        })
    }

    pub async fn collect_pull_request_context_view(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<PullRequestContextView> {
        let context = self
            .collect_pull_request_context(owner, repo, pull_number)
            .await?;
        Ok(PullRequestContextView::from_context(context))
    }

    pub async fn list_pull_files(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<Vec<PullFile>> {
        let url = self.api_url(&format!("/repos/{owner}/{repo}/pulls/{pull_number}/files"));
        self.get_paginated_json(url).await
    }

    pub async fn list_pull_reviews(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<Vec<PullReview>> {
        let url = self.api_url(&format!(
            "/repos/{owner}/{repo}/pulls/{pull_number}/reviews"
        ));
        self.get_paginated_json(url).await
    }

    pub async fn list_pull_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<Vec<ReviewComment>> {
        let url = self.api_url(&format!(
            "/repos/{owner}/{repo}/pulls/{pull_number}/comments"
        ));
        self.get_paginated_json(url).await
    }

    pub async fn list_check_runs(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
    ) -> Result<Vec<CheckRun>> {
        let url = self.api_url(&format!(
            "/repos/{owner}/{repo}/commits/{head_sha}/check-runs"
        ));
        let response: CheckRunsResponse = self.get_json(url).await?;
        Ok(response.check_runs)
    }

    pub async fn commit_status(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
    ) -> Result<CommitStatusSummary> {
        let url = self.api_url(&format!("/repos/{owner}/{repo}/commits/{head_sha}/status"));
        self.get_json(url).await
    }

    pub async fn submit_review(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
        request: &ReviewSubmitRequest,
    ) -> Result<SubmittedReviewResponse> {
        let url = self.api_url(&format!(
            "/repos/{owner}/{repo}/pulls/{pull_number}/reviews"
        ));
        let response = self
            .authorized(self.client.post(url))
            .json(request)
            .send()
            .await?;
        self.decode_response(response).await
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: String) -> Result<T> {
        let response = self.authorized(self.client.get(url)).send().await?;
        self.decode_response(response).await
    }

    async fn pull_request(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<PullRequestResponse> {
        let url = self.api_url(&format!("/repos/{owner}/{repo}/pulls/{pull_number}"));
        self.get_json(url).await
    }

    async fn search_pull_requests(
        &self,
        query: &str,
        review_requested: bool,
        assigned: bool,
    ) -> Result<Vec<PullRequestQueueItem>> {
        const MAX_SEARCH_PAGES: u32 = 5;
        let url = self.api_url("/search/issues");
        let mut page = 1;
        let mut items = Vec::new();

        loop {
            let page_string = page.to_string();
            let response = self
                .authorized(self.client.get(&url).query(&[
                    ("q", query),
                    ("sort", "updated"),
                    ("order", "desc"),
                    ("per_page", "100"),
                    ("page", page_string.as_str()),
                ]))
                .send()
                .await?;
            let headers = response.headers().clone();
            let result: SearchIssuesResponse = self.decode_response(response).await?;
            let incomplete_results = result.incomplete_results;
            items.extend(
                result
                    .items
                    .into_iter()
                    .map(|item| item.into_queue_item(review_requested, assigned)),
            );

            if incomplete_results || next_link(&headers).is_none() || page >= MAX_SEARCH_PAGES {
                break;
            }
            page += 1;
        }

        Ok(items)
    }

    async fn get_paginated_json<T: DeserializeOwned>(&self, first_url: String) -> Result<Vec<T>> {
        let mut items = Vec::new();
        let mut next_url = Some(first_url);

        while let Some(url) = next_url {
            let response = self.authorized(self.client.get(url)).send().await?;
            let status = response.status();
            let headers = response.headers().clone();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                let kind = classify_github_error(status.as_u16(), &headers, &body);
                return Err(ReviewDeskError::GitHubApi {
                    kind,
                    message: body,
                });
            }

            next_url = next_link(&headers);
            items.extend(response.json::<Vec<T>>().await?);
        }

        Ok(items)
    }

    async fn decode_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();
        let headers = response.headers().clone();
        if status.is_success() {
            return Ok(response.json().await?);
        }

        let body = response.text().await.unwrap_or_default();
        let kind = classify_github_error(status.as_u16(), &headers, &body);
        Err(ReviewDeskError::GitHubApi {
            kind,
            message: body,
        })
    }

    fn authorized(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let request = request
            .header("accept", "application/vnd.github+json")
            .header("x-github-api-version", "2026-03-10")
            .header("user-agent", "ReviewDesk/0.0.9");
        match &self.token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.api_base.trim_end_matches('/'), path)
    }
}

trait QueueItemRef {
    fn full_pr_ref(&self) -> String;
}

impl QueueItemRef for PullRequestQueueItem {
    fn full_pr_ref(&self) -> String {
        format!("{}/{}#{}", self.owner, self.repo, self.number)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RepositoryResponse {
    name: String,
    full_name: String,
    private: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchIssuesResponse {
    #[serde(default)]
    incomplete_results: bool,
    items: Vec<SearchIssueResponse>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchIssueResponse {
    number: u64,
    title: String,
    html_url: String,
    repository_url: String,
    user: Option<GitHubUser>,
    updated_at: Option<DateTime<Utc>>,
}

impl SearchIssueResponse {
    fn into_queue_item(self, review_requested: bool, assigned: bool) -> PullRequestQueueItem {
        let (owner, repo) = owner_repo_from_repository_url(&self.repository_url);
        PullRequestQueueItem {
            owner,
            repo,
            number: self.number,
            title: self.title,
            author: self
                .user
                .map(|user| user.login)
                .unwrap_or_else(|| "unknown".to_string()),
            url: self.html_url,
            draft: false,
            review_requested,
            assigned,
            review_reason: Some(review_reason(review_requested, assigned)),
            ci_status: None,
            changed_files_count: None,
            updated_at: self.updated_at,
            stale_status: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PullRequestResponse {
    number: u64,
    title: String,
    html_url: String,
    state: String,
    merged: Option<bool>,
    draft: Option<bool>,
    user: Option<GitHubUser>,
    base: GitRefResponse,
    head: GitRefResponse,
    additions: u64,
    deletions: u64,
    changed_files: u64,
    updated_at: Option<DateTime<Utc>>,
}

impl PullRequestResponse {
    fn into_queue_item(
        self,
        owner: &str,
        repo: &str,
        review_requested: bool,
        assigned: bool,
    ) -> PullRequestQueueItem {
        PullRequestQueueItem {
            owner: owner.to_string(),
            repo: repo.to_string(),
            number: self.number,
            title: self.title,
            author: self
                .user
                .map(|user| user.login)
                .unwrap_or_else(|| "unknown".to_string()),
            url: self.html_url,
            draft: self.draft.unwrap_or(false),
            review_requested,
            assigned,
            review_reason: Some(review_reason(review_requested, assigned)),
            ci_status: None,
            changed_files_count: Some(self.changed_files),
            updated_at: self.updated_at,
            stale_status: None,
        }
    }

    fn into_snapshot(self, owner: &str, repo: &str) -> PullRequestSnapshot {
        PullRequestSnapshot {
            snapshot_id: format!(
                "{}-{}-{}-{}",
                owner.replace('/', "-"),
                repo,
                self.number,
                self.head.sha
            ),
            owner: owner.to_string(),
            repo: repo.to_string(),
            number: self.number,
            base_branch: self.base.ref_name,
            head_branch: self.head.ref_name,
            head_sha: self.head.sha,
            state: self.state,
            merged: self.merged.unwrap_or(false),
            additions: self.additions,
            deletions: self.deletions,
            changed_files_metadata_path: None,
            collected_at: Utc::now(),
        }
    }
}

fn review_reason(review_requested: bool, assigned: bool) -> String {
    match (review_requested, assigned) {
        (true, true) => "both",
        (true, false) => "review_requested",
        (false, true) => "assigned",
        (false, false) => "open",
    }
    .to_string()
}

#[derive(Debug, Clone, Deserialize)]
struct GitRefResponse {
    #[serde(rename = "ref")]
    ref_name: String,
    #[serde(default)]
    sha: String,
}

fn owner_repo_from_repository_url(repository_url: &str) -> (String, String) {
    let parts = repository_url
        .rsplit_once("/repos/")
        .map(|(_, suffix)| suffix)
        .unwrap_or(repository_url);
    let mut segments = parts.split('/');
    (
        segments.next().unwrap_or_default().to_string(),
        segments.next().unwrap_or_default().to_string(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrReference {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

impl fmt::Display for PrReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}#{}", self.owner, self.repo, self.number)
    }
}

pub fn parse_pr_reference(input: &str) -> Result<PrReference> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(invalid_pr_reference(input));
    }

    if let Ok(url) = url::Url::parse(trimmed) {
        if url.host_str() != Some("github.com") {
            return Err(invalid_pr_reference(input));
        }
        let segments = url
            .path_segments()
            .map(|segments| segments.collect::<Vec<_>>())
            .unwrap_or_default();
        return parse_pr_segments(&segments).ok_or_else(|| invalid_pr_reference(input));
    }

    if let Some((repo_path, number)) = trimmed.split_once('#') {
        let segments = repo_path.split('/').collect::<Vec<_>>();
        let number = number
            .parse::<u64>()
            .map_err(|_| invalid_pr_reference(input))?;
        if segments.len() == 2 && !segments[0].is_empty() && !segments[1].is_empty() {
            return Ok(PrReference {
                owner: segments[0].to_string(),
                repo: segments[1].to_string(),
                number,
            });
        }
        return Err(invalid_pr_reference(input));
    }

    let segments = trimmed.split('/').collect::<Vec<_>>();
    parse_pr_segments(&segments).ok_or_else(|| invalid_pr_reference(input))
}

fn parse_pr_segments(segments: &[&str]) -> Option<PrReference> {
    if segments.len() != 4 || segments[2] != "pull" {
        return None;
    }
    if segments[0].is_empty() || segments[1].is_empty() {
        return None;
    }
    let number = segments[3].parse::<u64>().ok()?;
    Some(PrReference {
        owner: segments[0].to_string(),
        repo: segments[1].to_string(),
        number,
    })
}

fn invalid_pr_reference(input: &str) -> ReviewDeskError {
    ReviewDeskError::InvalidPath(format!("invalid GitHub PR reference: {input}"))
}

pub fn build_github_authorize_url(
    client_id: &str,
    redirect_uri: &str,
    scopes: &[&str],
    state: &str,
    code_challenge: &str,
) -> Result<url::Url> {
    validate_broker_redirect_uri(redirect_uri)?;
    let mut url = url::Url::parse("https://github.com/login/oauth/authorize")?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", &scopes.join(" "))
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url)
}

pub fn validate_loopback_redirect_uri(raw_url: &str) -> Result<url::Url> {
    let url = url::Url::parse(raw_url)?;
    let is_loopback = matches!(url.host_str(), Some("127.0.0.1") | Some("::1"));
    if url.scheme() != "http" || !is_loopback || url.port().is_none() {
        return Err(ReviewDeskError::InvalidPath(
            "GitHub OAuth redirect URI must use a loopback literal host and explicit port"
                .to_string(),
        ));
    }
    Ok(url)
}

pub fn validate_broker_redirect_uri(raw_url: &str) -> Result<url::Url> {
    let url = url::Url::parse(raw_url)?;
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err(ReviewDeskError::InvalidPath(
            "GitHub OAuth broker redirect URI must use https".to_string(),
        ));
    }
    Ok(url)
}

pub fn pkce_challenge_s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64_url_no_pad(&digest)
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity((bytes.len() * 4).div_ceil(3));

    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        let value = ((first as u32) << 16) | ((second as u32) << 8) | third as u32;

        output.push(TABLE[((value >> 18) & 0x3f) as usize] as char);
        output.push(TABLE[((value >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[((value >> 6) & 0x3f) as usize] as char);
        }
        if chunk.len() > 2 {
            output.push(TABLE[(value & 0x3f) as usize] as char);
        }
    }

    output
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevicePoll {
    Pending,
    SlowDown,
    Authorized(String),
    Denied(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeviceFlowStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BrowserOAuthToken {
    pub access_token: String,
    pub token_type: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewSubmitRequest {
    pub commit_id: String,
    pub event: ReviewEvent,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SubmittedReviewResponse {
    pub id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub user: Option<GitHubUser>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PullReview {
    pub id: u64,
    pub state: String,
    pub body: Option<String>,
    pub user: Option<GitHubUser>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReviewComment {
    pub id: u64,
    pub path: String,
    pub body: String,
    pub line: Option<u64>,
    pub side: Option<String>,
    pub user: Option<GitHubUser>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CheckRun {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct CheckRunsResponse {
    check_runs: Vec<CheckRun>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommitStatusSummary {
    pub state: String,
    pub statuses: Vec<CommitStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommitStatus {
    pub context: String,
    pub state: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PullFile {
    pub filename: String,
    pub patch: Option<String>,
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestContext {
    pub snapshot: PullRequestSnapshot,
    pub files: Vec<ChangedFile>,
    pub issue_comments: Vec<IssueComment>,
    pub reviews: Vec<PullReview>,
    pub review_comments: Vec<ReviewComment>,
    pub check_runs: Vec<CheckRun>,
    pub commit_status: CommitStatusSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestContextView {
    pub pr: PullRequestContextPr,
    pub files: Vec<ChangedFileContext>,
    pub conversation: ConversationSummary,
    pub ci: CiRollup,
    pub freshness: StaleStatus,
    pub patch_coverage: String,
    pub diff_hash: String,
    pub context_hash: String,
    pub collected_at: DateTime<Utc>,
    pub warnings: Vec<ContextWarning>,
    pub ai_input: ReviewInputPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestContextPr {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub author: String,
    pub url: String,
    pub state: String,
    pub draft: bool,
    pub merged: bool,
    pub base_branch: String,
    pub head_branch: String,
    pub base_sha: Option<String>,
    pub head_sha: String,
    pub additions: u64,
    pub deletions: u64,
    pub labels: Vec<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedFileContext {
    pub path: String,
    pub patch: Option<String>,
    pub status: String,
    pub previous_path: Option<String>,
    pub additions: u64,
    pub deletions: u64,
    pub changes: u64,
    pub patch_coverage: String,
    pub patch_bytes: usize,
    pub patch_hash: Option<String>,
    pub is_generated: bool,
    pub is_ignored_by_reviewdesk: bool,
    pub ai_included: bool,
    pub ui_only_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub issue_comment_count: usize,
    pub review_comment_count: usize,
    pub review_count: usize,
    pub summaries: Vec<ConversationItemSummary>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationItemSummary {
    pub kind: String,
    pub author: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub body_summary: String,
    pub unresolved: String,
    pub ai_included: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiRollupState {
    Success,
    Pending,
    Failure,
    Cancelled,
    Skipped,
    Neutral,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiRollup {
    pub state: CiRollupState,
    pub source: String,
    pub failing_names: Vec<String>,
    pub pending_count: usize,
    pub latest_completed_at: Option<DateTime<Utc>>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextWarning {
    PatchUnavailable,
    PatchTruncated,
    PartialFetch,
    UnresolvedUnknown,
    ContextStale,
    CiUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewInputPreview {
    pub transmitted_scope: String,
    pub included_file_count: usize,
    pub excluded_file_count: usize,
    pub total_patch_bytes: usize,
    pub redaction_passed: bool,
    pub private_diff_consent_required: bool,
}

impl PullRequestContextView {
    fn from_context(context: PullRequestContext) -> Self {
        let files = context
            .files
            .iter()
            .map(|file| {
                let patch_bytes = file.patch.as_ref().map(|patch| patch.len()).unwrap_or(0);
                let patch_coverage = if file.patch.is_some() {
                    "available"
                } else {
                    "missing"
                }
                .to_string();
                ChangedFileContext {
                    path: file.path.clone(),
                    patch: file.patch.clone(),
                    status: "changed".to_string(),
                    previous_path: None,
                    additions: 0,
                    deletions: 0,
                    changes: 0,
                    patch_coverage,
                    patch_bytes,
                    patch_hash: file
                        .patch
                        .as_ref()
                        .map(|patch| sha256_hex(patch.as_bytes())),
                    is_generated: is_generated_file(&file.path),
                    is_ignored_by_reviewdesk: is_ignored_file(&file.path),
                    ai_included: file.patch.is_some(),
                    ui_only_reason: file
                        .patch
                        .is_none()
                        .then(|| "patch_unavailable".to_string()),
                }
            })
            .collect::<Vec<_>>();
        let patch_coverage = if files.iter().all(|file| file.patch_coverage == "available") {
            "complete"
        } else if files.iter().any(|file| file.patch_coverage == "available") {
            "partial"
        } else {
            "missing"
        }
        .to_string();
        let diff_hash = stable_changed_files_hash(&context.files);
        let ci = rollup_ci(&context.check_runs, &context.commit_status);
        let warnings = if ci.state == CiRollupState::Unknown {
            vec![ContextWarning::CiUnavailable]
        } else {
            Vec::new()
        };
        let total_patch_bytes = files.iter().map(|file| file.patch_bytes).sum();
        let included_file_count = files.iter().filter(|file| file.ai_included).count();
        let excluded_file_count = files.len().saturating_sub(included_file_count);
        let context_hash = sha256_hex(
            format!(
                "{}:{}:{}:{}:{}",
                context.snapshot.owner,
                context.snapshot.repo,
                context.snapshot.number,
                context.snapshot.head_sha,
                diff_hash
            )
            .as_bytes(),
        );

        Self {
            pr: PullRequestContextPr {
                owner: context.snapshot.owner.clone(),
                repo: context.snapshot.repo.clone(),
                number: context.snapshot.number,
                title: String::new(),
                body: None,
                author: String::new(),
                url: format!(
                    "https://github.com/{}/{}/pull/{}",
                    context.snapshot.owner, context.snapshot.repo, context.snapshot.number
                ),
                state: context.snapshot.state.clone(),
                draft: false,
                merged: context.snapshot.merged,
                base_branch: context.snapshot.base_branch.clone(),
                head_branch: context.snapshot.head_branch.clone(),
                base_sha: None,
                head_sha: context.snapshot.head_sha.clone(),
                additions: context.snapshot.additions,
                deletions: context.snapshot.deletions,
                labels: Vec::new(),
                updated_at: None,
            },
            files,
            conversation: ConversationSummary {
                issue_comment_count: context.issue_comments.len(),
                review_comment_count: context.review_comments.len(),
                review_count: context.reviews.len(),
                summaries: Vec::new(),
                truncated: false,
            },
            ci,
            freshness: StaleStatus::Fresh,
            patch_coverage,
            diff_hash,
            context_hash,
            collected_at: context.snapshot.collected_at,
            warnings,
            ai_input: ReviewInputPreview {
                transmitted_scope: "selected_patches".to_string(),
                included_file_count,
                excluded_file_count,
                total_patch_bytes,
                redaction_passed: true,
                private_diff_consent_required: false,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GitHubUser {
    pub login: String,
}

fn rollup_ci(check_runs: &[CheckRun], commit_status: &CommitStatusSummary) -> CiRollup {
    let mut states = Vec::new();
    let mut failing_names = Vec::new();
    let mut pending_count = 0;

    for run in check_runs {
        let state = normalize_check_run(run);
        if state == CiRollupState::Failure {
            failing_names.push(run.name.clone());
        }
        if state == CiRollupState::Pending {
            pending_count += 1;
        }
        states.push(state);
    }
    for status in &commit_status.statuses {
        let state = normalize_commit_status(&status.state);
        if state == CiRollupState::Failure {
            failing_names.push(status.context.clone());
        }
        if state == CiRollupState::Pending {
            pending_count += 1;
        }
        states.push(state);
    }
    if states.is_empty() {
        states.push(normalize_commit_status(&commit_status.state));
    }

    CiRollup {
        state: highest_priority_ci_state(&states),
        source: if check_runs.is_empty() && commit_status.statuses.is_empty() {
            "unavailable"
        } else if check_runs.is_empty() {
            "statuses"
        } else if commit_status.statuses.is_empty() {
            "checks"
        } else {
            "both"
        }
        .to_string(),
        failing_names,
        pending_count,
        latest_completed_at: None,
        warnings: Vec::new(),
    }
}

fn normalize_check_run(run: &CheckRun) -> CiRollupState {
    match run.status.as_str() {
        "queued" | "in_progress" | "waiting" | "requested" | "pending" => CiRollupState::Pending,
        _ => match run.conclusion.as_deref() {
            Some("success") => CiRollupState::Success,
            Some("failure" | "startup_failure" | "timed_out" | "action_required" | "stale") => {
                CiRollupState::Failure
            }
            Some("cancelled") => CiRollupState::Cancelled,
            Some("skipped") => CiRollupState::Skipped,
            Some("neutral") => CiRollupState::Neutral,
            _ => CiRollupState::Unknown,
        },
    }
}

fn normalize_commit_status(state: &str) -> CiRollupState {
    match state {
        "success" => CiRollupState::Success,
        "pending" => CiRollupState::Pending,
        "error" | "failure" => CiRollupState::Failure,
        _ => CiRollupState::Unknown,
    }
}

fn highest_priority_ci_state(states: &[CiRollupState]) -> CiRollupState {
    for candidate in [
        CiRollupState::Failure,
        CiRollupState::Cancelled,
        CiRollupState::Pending,
        CiRollupState::Success,
        CiRollupState::Neutral,
        CiRollupState::Skipped,
        CiRollupState::Unknown,
    ] {
        if states.contains(&candidate) {
            return candidate;
        }
    }
    CiRollupState::Unknown
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn is_generated_file(path: &str) -> bool {
    path.ends_with(".lock") || path.contains("/generated/")
}

fn is_ignored_file(path: &str) -> bool {
    path.ends_with("pnpm-lock.yaml") || path.ends_with("package-lock.json")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleCheckInput {
    pub draft_head_sha: String,
    pub current_head_sha: String,
    pub state: String,
    pub merged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleStatus {
    Fresh,
    Unknown,
    HeadChanged,
    DiffChanged,
    ContextChanged,
    PrClosed,
    PrMerged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleCheckResult {
    pub is_stale: bool,
    pub reason: Option<String>,
    pub status: StaleStatus,
}

impl StaleCheckInput {
    pub fn evaluate(&self) -> StaleCheckResult {
        if self.merged {
            return StaleCheckResult {
                is_stale: true,
                reason: Some("pr_merged".to_string()),
                status: StaleStatus::PrMerged,
            };
        }
        if self.state != "open" {
            return StaleCheckResult {
                is_stale: true,
                reason: Some("pr_closed".to_string()),
                status: StaleStatus::PrClosed,
            };
        }
        if self.draft_head_sha != self.current_head_sha {
            return StaleCheckResult {
                is_stale: true,
                reason: Some("head_changed".to_string()),
                status: StaleStatus::HeadChanged,
            };
        }
        StaleCheckResult {
            is_stale: false,
            reason: None,
            status: StaleStatus::Fresh,
        }
    }
}

pub fn classify_github_error(status: u16, headers: &HeaderMap, body: &str) -> GitHubErrorKind {
    let body_lower = body.to_ascii_lowercase();
    if status == 401 {
        return GitHubErrorKind::AuthRequired;
    }
    if status == 403 && body_lower.contains("saml") {
        return GitHubErrorKind::SsoRequired;
    }
    if status == 403 && body_lower.contains("secondary rate limit") {
        return GitHubErrorKind::SecondaryRateLimited;
    }
    if status == 403
        && (headers
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            == Some("0"))
    {
        return GitHubErrorKind::RateLimited;
    }
    if headers.get(RETRY_AFTER).is_some() {
        return GitHubErrorKind::SecondaryRateLimited;
    }
    if status == 403 && body_lower.contains("resource not accessible") {
        return GitHubErrorKind::ScopeMissing;
    }
    if status == 404 {
        return GitHubErrorKind::NotFound;
    }
    GitHubErrorKind::Unknown
}

fn next_link(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(LINK)?.to_str().ok()?;
    for section in value.split(',') {
        let section = section.trim();
        let Some((url, params)) = section.split_once(';') else {
            continue;
        };
        if params.contains("rel=\"next\"") {
            return Some(
                url.trim()
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string(),
            );
        }
    }
    None
}
