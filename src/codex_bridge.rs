use crate::app_core::{
    AiAccountView, AiBlockedReason, AiConnectionStatus, AiConnectionStatusView, AiModelUpgradeView,
    AiModelView, AiRateLimitSnapshot, AiRateLimitStatus, ReasoningEffort,
};
use crate::domain::{Result, ReviewDeskError};
use crate::security::SecretMasker;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

const MIN_APP_SERVER_MAJOR: u64 = 0;
const MIN_APP_SERVER_MINOR: u64 = 130;
const APP_SERVER_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexTransport {
    AppServerStdio,
    ExecJson,
    Unavailable,
}

impl Default for CodexTransport {
    fn default() -> Self {
        Self::AppServerStdio
    }
}

impl CodexTransport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AppServerStdio => "app_server_stdio",
            Self::ExecJson => "exec_json",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexCliVersion {
    pub binary_name: String,
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl CodexCliVersion {
    pub fn supports_app_server(&self) -> bool {
        self.major > MIN_APP_SERVER_MAJOR
            || (self.major == MIN_APP_SERVER_MAJOR && self.minor >= MIN_APP_SERVER_MINOR)
    }
}

pub fn version_from_output(output: &str) -> Option<CodexCliVersion> {
    let mut parts = output.split_whitespace();
    let binary_name = parts.next()?.to_string();
    let version = parts.next()?;
    let mut numbers = version.split('.');
    Some(CodexCliVersion {
        binary_name,
        major: numbers.next()?.parse().ok()?,
        minor: numbers.next()?.parse().ok()?,
        patch: numbers.next().unwrap_or("0").parse().ok()?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexAuthMode {
    ChatGpt,
    ApiKey,
    ChatGptAuthTokens,
    AmazonBedrock,
    Missing,
    Unknown,
}

impl CodexAuthMode {
    pub fn from_raw(value: &str) -> Self {
        match value {
            "chatgpt" => Self::ChatGpt,
            "apiKey" | "apikey" | "api_key" => Self::ApiKey,
            "chatgptAuthTokens" | "chatgpt_auth_tokens" => Self::ChatGptAuthTokens,
            "amazonBedrock" | "amazon_bedrock" => Self::AmazonBedrock,
            "" => Self::Missing,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChatGpt => "chatgpt",
            Self::ApiKey => "apikey",
            Self::ChatGptAuthTokens => "chatgpt_auth_tokens",
            Self::AmazonBedrock => "amazon_bedrock",
            Self::Missing => "missing",
            Self::Unknown => "unknown",
        }
    }

    pub fn blocked_reason(self) -> Option<AiBlockedReason> {
        match self {
            Self::ChatGpt => None,
            Self::Missing => Some(AiBlockedReason::CodexChatgptAuthRequired),
            Self::ApiKey | Self::ChatGptAuthTokens | Self::AmazonBedrock | Self::Unknown => {
                Some(AiBlockedReason::CodexAuthModeUnsupported)
            }
        }
    }
}

pub fn account_status_from_app_server_response(value: &Value) -> AiConnectionStatusView {
    let Some(account) = value.get("account").filter(|account| !account.is_null()) else {
        return AiConnectionStatusView::missing();
    };
    let auth_mode = account
        .get("type")
        .and_then(Value::as_str)
        .map(CodexAuthMode::from_raw)
        .unwrap_or(CodexAuthMode::Unknown);

    if auth_mode != CodexAuthMode::ChatGpt {
        return AiConnectionStatusView {
            provider: "codex_chatgpt".to_string(),
            status: AiConnectionStatus::Unsupported,
            auth_mode: Some(auth_mode.as_str().to_string()),
            account: None,
            plan_type: None,
            last_refreshed_at: None,
            blocked_reason: auth_mode.blocked_reason(),
        };
    }

    let plan_type = account
        .get("planType")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let email = account
        .get("email")
        .and_then(Value::as_str)
        .unwrap_or("chatgpt");
    let account_id_hash = stable_hash(&format!("{email}:{}", plan_type.as_deref().unwrap_or("")));
    AiConnectionStatusView::connected(AiAccountView {
        account_id_hash,
        display_label: Some("Codex ChatGPT".to_string()),
        workspace_name: None,
        plan_type,
    })
}

pub fn map_app_server_models(
    value: &Value,
) -> std::result::Result<Vec<AiModelView>, AiBlockedReason> {
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or(AiBlockedReason::ModelListUnavailable)?;
    let mut models = Vec::with_capacity(data.len());
    for item in data {
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .ok_or(AiBlockedReason::ModelListUnavailable)?;
        let display_name = item
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or(id);
        let hidden = item.get("hidden").and_then(Value::as_bool).unwrap_or(false);
        let supported = item
            .get("supportedReasoningEfforts")
            .and_then(Value::as_array)
            .map(|efforts| {
                efforts
                    .iter()
                    .filter_map(|effort| {
                        effort
                            .get("reasoningEffort")
                            .and_then(Value::as_str)
                            .and_then(reasoning_effort_from_codex)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let default_reasoning_effort = item
            .get("defaultReasoningEffort")
            .and_then(Value::as_str)
            .and_then(reasoning_effort_from_codex)
            .or_else(|| supported.first().copied())
            .unwrap_or(ReasoningEffort::Medium);
        let input_modalities = item
            .get("inputModalities")
            .and_then(Value::as_array)
            .map(|modalities| {
                modalities
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .filter(|modalities| !modalities.is_empty())
            .unwrap_or_else(|| vec!["text".to_string()]);
        let upgrade_model = item.get("upgrade").and_then(Value::as_str);
        let upgrade_info = item.get("upgradeInfo").filter(|value| !value.is_null());
        let available = upgrade_model.is_none() && upgrade_info.is_none();
        let mut model = AiModelView {
            id: id.to_string(),
            display_name: display_name.to_string(),
            is_default: item
                .get("isDefault")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            hidden,
            available,
            unavailable_reason: (!available).then(|| "upgrade_required".to_string()),
            supported_reasoning_efforts: supported,
            default_reasoning_effort,
            input_modalities,
            upgrade: None,
        };
        if let Some(required) = upgrade_model {
            model.upgrade = Some(AiModelUpgradeView {
                required_plan: None,
                message: format!("Use {required} instead"),
            });
        }
        models.push(model);
    }
    Ok(models)
}

pub fn reasoning_effort_from_codex(value: &str) -> Option<ReasoningEffort> {
    match value {
        "minimal" => Some(ReasoningEffort::Minimal),
        "low" => Some(ReasoningEffort::Low),
        "medium" => Some(ReasoningEffort::Medium),
        "high" => Some(ReasoningEffort::High),
        "xhigh" => Some(ReasoningEffort::Xhigh),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexJsonRpcMessageKind {
    Response,
    Notification,
    ServerRequest,
    Unknown,
}

pub fn classify_json_rpc_message(value: &Value) -> CodexJsonRpcMessageKind {
    let has_id = value.get("id").is_some();
    let has_method = value.get("method").and_then(Value::as_str).is_some();
    let has_result_or_error = value.get("result").is_some() || value.get("error").is_some();
    match (has_id, has_method, has_result_or_error) {
        (true, false, true) => CodexJsonRpcMessageKind::Response,
        (false, true, _) => CodexJsonRpcMessageKind::Notification,
        (true, true, _) => CodexJsonRpcMessageKind::ServerRequest,
        _ => CodexJsonRpcMessageKind::Unknown,
    }
}

pub fn extract_codex_exec_draft_from_jsonl(
    jsonl: &str,
) -> std::result::Result<String, AiBlockedReason> {
    let mut deltas = String::new();
    let mut final_text = String::new();

    for line in jsonl.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let value: Value =
            serde_json::from_str(line).map_err(|_| AiBlockedReason::CodexGenerationFailed)?;
        if server_request_requires_block(&value) {
            return Err(AiBlockedReason::CodexToolUseNotAllowed);
        }
        if let Some(delta) = value
            .pointer("/params/delta")
            .and_then(Value::as_str)
            .or_else(|| value.get("delta").and_then(Value::as_str))
        {
            deltas.push_str(delta);
        }
        collect_output_text(&value, &mut final_text);
        if value.get("type").and_then(Value::as_str) == Some("error")
            || value.get("method").and_then(Value::as_str) == Some("error")
        {
            return Err(AiBlockedReason::CodexGenerationFailed);
        }
    }

    let draft = if deltas.trim().is_empty() {
        final_text.trim().to_string()
    } else {
        deltas.trim().to_string()
    };
    if draft.is_empty() {
        Err(AiBlockedReason::CodexGenerationFailed)
    } else {
        Ok(draft)
    }
}

fn collect_output_text(value: &Value, output: &mut String) {
    if let Some(text) = value.get("text").and_then(Value::as_str) {
        output.push_str(text);
    }
    if let Some(text) = value.get("message").and_then(Value::as_str) {
        output.push_str(text);
    }
    if let Some(content) = value.pointer("/item/content").and_then(Value::as_array) {
        for item in content {
            if item.get("type").and_then(Value::as_str) == Some("output_text") {
                if let Some(text) = item.get("text").and_then(Value::as_str) {
                    output.push_str(text);
                }
            }
        }
    }
    if let Some(output_items) = value.pointer("/response/output").and_then(Value::as_array) {
        for item in output_items {
            collect_output_text(item, output);
        }
    }
}

fn server_request_requires_block(value: &Value) -> bool {
    matches!(
        value.get("method").and_then(Value::as_str),
        Some("item/commandExecution/requestApproval")
            | Some("item/fileChange/requestApproval")
            | Some("item/permissions/requestApproval")
            | Some("account/chatgptAuthTokens/refresh")
            | Some("applyPatchApproval")
            | Some("execCommandApproval")
    )
}

pub fn payload_contains_forbidden_fields(value: &Value) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(key, value)| forbidden_key(key) || payload_contains_forbidden_fields(value)),
        Value::Array(values) => values.iter().any(payload_contains_forbidden_fields),
        Value::String(value) => forbidden_text(value),
        _ => false,
    }
}

fn forbidden_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['_', '-'], "");
    matches!(
        normalized.as_str(),
        "accesstoken"
            | "refreshtoken"
            | "apikey"
            | "authorization"
            | "chatgptauthtokens"
            | "openaikey"
    )
}

fn forbidden_text(value: &str) -> bool {
    value.contains("auth.json") || value.contains("chatgptAuthTokens")
}

pub fn sanitize_codex_diagnostics(input: &str) -> String {
    let mut text = SecretMasker::default().mask(input).text;
    for forbidden in [
        "accessToken",
        "refreshToken",
        "Authorization",
        "chatgptAuthTokens",
        "auth.json",
        ".codex/auth.json",
    ] {
        text = text.replace(forbidden, "[REDACTED]");
    }
    text
}

#[derive(Debug, Clone, Default)]
pub struct CodexProcessEnvPolicy {
    input: BTreeMap<String, String>,
}

impl CodexProcessEnvPolicy {
    pub fn from_current_env() -> Self {
        Self {
            input: env::vars().collect(),
        }
    }

    pub fn insert_input(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.input.insert(key.into(), value.into());
    }

    pub fn filtered_env(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        for key in ["PATH", "HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG"] {
            if let Some(value) = self.input.get(key) {
                env.insert(key.to_string(), value.clone());
            }
        }
        env
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexBridgeStatusView {
    pub transport: CodexTransport,
    pub codex_path: Option<String>,
    pub version: Option<String>,
    pub app_server_available: bool,
    pub exec_available: bool,
    pub blocked_reason: Option<AiBlockedReason>,
    pub diagnostics: Vec<String>,
}

impl CodexBridgeStatusView {
    pub fn blocked(reason: AiBlockedReason, diagnostics: Vec<String>) -> Self {
        Self {
            transport: CodexTransport::Unavailable,
            codex_path: None,
            version: None,
            app_server_available: false,
            exec_available: false,
            blocked_reason: Some(reason),
            diagnostics,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodexBridge {
    codex_path: PathBuf,
}

impl CodexBridge {
    pub fn resolve_from_env() -> Result<Self> {
        let path = resolve_codex_binary()
            .ok_or_else(|| ReviewDeskError::AiBlocked("codex_cli_missing".to_string()))?;
        Ok(Self { codex_path: path })
    }

    pub fn codex_path(&self) -> &Path {
        &self.codex_path
    }

    pub async fn status(&self) -> CodexBridgeStatusView {
        let version_output = match run_codex_output(&self.codex_path, &["--version"], None).await {
            Ok(output) => output,
            Err(error) => {
                return CodexBridgeStatusView::blocked(
                    AiBlockedReason::CodexCliMissing,
                    vec![sanitize_codex_diagnostics(&error.to_string())],
                );
            }
        };
        let Some(version) = version_from_output(&version_output) else {
            return CodexBridgeStatusView::blocked(
                AiBlockedReason::CodexVersionUnsupported,
                vec!["codex_version_parse_failed".to_string()],
            );
        };
        if !version.supports_app_server() {
            return CodexBridgeStatusView {
                transport: CodexTransport::Unavailable,
                codex_path: Some(self.codex_path.display().to_string()),
                version: Some(version_output.trim().to_string()),
                app_server_available: false,
                exec_available: false,
                blocked_reason: Some(AiBlockedReason::CodexVersionUnsupported),
                diagnostics: Vec::new(),
            };
        }
        CodexBridgeStatusView {
            transport: CodexTransport::AppServerStdio,
            codex_path: Some(self.codex_path.display().to_string()),
            version: Some(version_output.trim().to_string()),
            app_server_available: true,
            exec_available: true,
            blocked_reason: None,
            diagnostics: Vec::new(),
        }
    }

    pub async fn read_account(&self, refresh_token: bool) -> Result<AiConnectionStatusView> {
        let result = self
            .app_server_requests(vec![json_rpc_request(
                2,
                "account/read",
                serde_json::json!({ "refreshToken": refresh_token }),
            )])
            .await?;
        Ok(result
            .get(&2)
            .map(account_status_from_app_server_response)
            .unwrap_or_else(AiConnectionStatusView::missing))
    }

    pub async fn list_models(&self) -> std::result::Result<Vec<AiModelView>, AiBlockedReason> {
        let result = self
            .app_server_requests(vec![json_rpc_request(
                2,
                "model/list",
                serde_json::json!({ "includeHidden": false, "limit": 100 }),
            )])
            .await
            .map_err(|_| AiBlockedReason::ModelListUnavailable)?;
        result
            .get(&2)
            .ok_or(AiBlockedReason::ModelListUnavailable)
            .and_then(map_app_server_models)
    }

    pub async fn read_rate_limits(&self) -> AiRateLimitSnapshot {
        let Ok(result) = self
            .app_server_requests(vec![json_rpc_request(
                2,
                "account/rateLimits/read",
                Value::Null,
            )])
            .await
        else {
            return AiRateLimitSnapshot::unknown();
        };
        result
            .get(&2)
            .map(rate_limit_from_app_server_response)
            .unwrap_or_else(AiRateLimitSnapshot::unknown)
    }

    pub async fn logout_account(&self) -> Result<()> {
        self.app_server_requests(vec![json_rpc_request(2, "account/logout", Value::Null)])
            .await?;
        Ok(())
    }

    pub async fn start_login(
        &self,
        mode: &str,
    ) -> Result<Option<(String, Option<String>, Option<String>, Option<String>)>> {
        let params = match mode {
            "device" => serde_json::json!({ "type": "chatgptDeviceCode" }),
            _ => serde_json::json!({ "type": "chatgpt", "codexStreamlinedLogin": true }),
        };
        let result = self
            .app_server_requests(vec![json_rpc_request(2, "account/login/start", params)])
            .await?;
        let Some(response) = result.get(&2) else {
            return Ok(None);
        };
        let response_type = response.get("type").and_then(Value::as_str).unwrap_or("");
        match response_type {
            "chatgpt" => Ok(Some((
                response
                    .get("loginId")
                    .and_then(Value::as_str)
                    .unwrap_or("codex-chatgpt-login")
                    .to_string(),
                response
                    .get("authUrl")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                None,
                None,
            ))),
            "chatgptDeviceCode" => Ok(Some((
                response
                    .get("loginId")
                    .and_then(Value::as_str)
                    .unwrap_or("codex-chatgpt-device-login")
                    .to_string(),
                None,
                response
                    .get("verificationUrl")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                response
                    .get("userCode")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
            ))),
            "chatgptAuthTokens" => Err(ReviewDeskError::AiBlocked(
                AiBlockedReason::CodexAuthModeUnsupported
                    .as_str()
                    .to_string(),
            )),
            _ => Ok(None),
        }
    }

    pub async fn generate_with_exec_json(
        &self,
        prompt: &str,
        model: &str,
        reasoning_effort: ReasoningEffort,
        cwd: Option<&Path>,
    ) -> std::result::Result<String, AiBlockedReason> {
        let mut args = vec![
            "exec".to_string(),
            "--json".to_string(),
            "--ephemeral".to_string(),
            "--sandbox".to_string(),
            "read-only".to_string(),
            "--skip-git-repo-check".to_string(),
            "--model".to_string(),
            model.to_string(),
            "-c".to_string(),
            format!("model_reasoning_effort=\"{}\"", reasoning_effort.as_str()),
            "-c".to_string(),
            "approval_policy=\"never\"".to_string(),
            "-".to_string(),
        ];
        if let Some(cwd) = cwd {
            args.insert(6, cwd.display().to_string());
            args.insert(6, "--cd".to_string());
        }
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = run_codex_output(&self.codex_path, &arg_refs, Some(prompt))
            .await
            .map_err(|_| AiBlockedReason::CodexGenerationFailed)?;
        extract_codex_exec_draft_from_jsonl(&output)
    }

    async fn app_server_requests(&self, requests: Vec<Value>) -> Result<BTreeMap<u64, Value>> {
        let env_policy = CodexProcessEnvPolicy::from_current_env();
        let mut command = Command::new(&self.codex_path);
        command.args(["app-server", "--listen", "stdio://"]);
        command.env_clear();
        command.envs(env_policy.filtered_env());
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn()?;
        let mut child_stdin = child.stdin.take().ok_or_else(|| {
            ReviewDeskError::AiBlocked("codex_app_server_stdin_missing".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ReviewDeskError::AiBlocked("codex_app_server_stdout_missing".to_string())
        })?;
        let mut stderr = child.stderr.take().ok_or_else(|| {
            ReviewDeskError::AiBlocked("codex_app_server_stderr_missing".to_string())
        })?;
        let mut lines = BufReader::new(stdout).lines();
        let mut results = BTreeMap::new();

        let initialize = json_rpc_request(
            1,
            "initialize",
            serde_json::json!({
                "clientInfo": {
                    "name": "reviewdesk",
                    "title": "ReviewDesk",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "experimentalApi": true
                }
            }),
        );
        write_json_rpc_line(&mut child_stdin, &initialize).await?;
        read_json_rpc_results_until(&mut lines, &mut results, &[1]).await?;

        let expected_ids = requests
            .iter()
            .filter_map(|request| request.get("id").and_then(Value::as_u64))
            .collect::<Vec<_>>();
        for request in &requests {
            write_json_rpc_line(&mut child_stdin, request).await?;
        }
        child_stdin.shutdown().await?;
        if !expected_ids.is_empty() {
            read_json_rpc_results_until(&mut lines, &mut results, &expected_ids).await?;
        }

        let wait_result = timeout(Duration::from_secs(2), child.wait()).await;
        if wait_result.is_err() {
            let _ = child.start_kill();
        }
        let mut stderr_text = String::new();
        let _ = stderr.read_to_string(&mut stderr_text).await;
        if let Ok(Ok(status)) = wait_result {
            if !status.success() && results.is_empty() {
                return Err(ReviewDeskError::AiBlocked(sanitize_codex_diagnostics(
                    &stderr_text,
                )));
            }
        }

        Ok(results)
    }
}

async fn write_json_rpc_line<W>(writer: &mut W, value: &Value) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    writer.write_all(value.to_string().as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

async fn read_json_rpc_results_until<R>(
    lines: &mut tokio::io::Lines<R>,
    results: &mut BTreeMap<u64, Value>,
    expected_ids: &[u64],
) -> Result<()>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    timeout(APP_SERVER_TIMEOUT, async {
        loop {
            if expected_ids.iter().all(|id| results.contains_key(id)) {
                return Ok(());
            }
            let Some(line) = lines.next_line().await? else {
                return Err(ReviewDeskError::AiBlocked(
                    "codex_app_server_response_missing".to_string(),
                ));
            };
            parse_json_rpc_line_into_results(&line, results)?;
        }
    })
    .await
    .map_err(|_| ReviewDeskError::AiBlocked("codex_process_timeout".to_string()))?
}

fn parse_json_rpc_line_into_results(line: &str, results: &mut BTreeMap<u64, Value>) -> Result<()> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(());
    }
    let value = match serde_json::from_str::<Value>(line) {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };
    if payload_contains_forbidden_fields(&value) {
        return Err(ReviewDeskError::AiBlocked(
            "codex_response_contains_forbidden_auth_fields".to_string(),
        ));
    }
    if server_request_requires_block(&value) {
        return Err(ReviewDeskError::AiBlocked(
            AiBlockedReason::CodexToolUseNotAllowed.as_str().to_string(),
        ));
    }
    if classify_json_rpc_message(&value) == CodexJsonRpcMessageKind::Response {
        let id = value.get("id").and_then(Value::as_u64).unwrap_or_default();
        if let Some(result) = value.get("result") {
            results.insert(id, result.clone());
        } else if let Some(error) = value.get("error") {
            return Err(ReviewDeskError::AiBlocked(sanitize_codex_diagnostics(
                &error.to_string(),
            )));
        }
    }
    Ok(())
}

pub fn rate_limit_from_app_server_response(value: &Value) -> AiRateLimitSnapshot {
    let rate_limits = value.get("rateLimits").unwrap_or(value);
    let reached = rate_limits
        .get("rateLimitReachedType")
        .and_then(Value::as_str);
    let plan_type = rate_limits
        .get("planType")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let resets_at = rate_limits
        .pointer("/primary/resetsAt")
        .and_then(Value::as_i64)
        .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
        .map(|timestamp| timestamp.to_rfc3339());
    match reached {
        Some("primary") | Some("secondary") => AiRateLimitSnapshot {
            status: AiRateLimitStatus::RateLimited,
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
            resets_at,
            blocked_reason: Some(AiBlockedReason::CodexChatgptRateLimited),
        },
        Some("credits") => AiRateLimitSnapshot {
            status: AiRateLimitStatus::CreditsDepleted,
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
            resets_at: None,
            blocked_reason: Some(AiBlockedReason::CodexChatgptCreditsDepleted),
        },
        _ => AiRateLimitSnapshot {
            status: AiRateLimitStatus::Ok,
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
            resets_at,
            blocked_reason: None,
        },
    }
    .with_plan_fallback(plan_type)
}

trait RateLimitPlanFallback {
    fn with_plan_fallback(self, _plan_type: Option<String>) -> Self;
}

impl RateLimitPlanFallback for AiRateLimitSnapshot {
    fn with_plan_fallback(self, _plan_type: Option<String>) -> Self {
        self
    }
}

fn json_rpc_request(id: u64, method: &str, params: Value) -> Value {
    if params.is_null() {
        serde_json::json!({ "id": id, "method": method })
    } else {
        serde_json::json!({ "id": id, "method": method, "params": params })
    }
}

pub fn resolve_codex_binary() -> Option<PathBuf> {
    if let Ok(configured) = env::var("REVIEWDESK_CODEX_BIN") {
        let path = PathBuf::from(configured);
        if path.is_absolute() && path.exists() {
            return Some(path);
        }
    }
    find_on_path("codex")
}

fn find_on_path(binary: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

async fn run_codex_output(path: &Path, args: &[&str], stdin: Option<&str>) -> Result<String> {
    let env_policy = CodexProcessEnvPolicy::from_current_env();
    let mut command = Command::new(path);
    command.args(args);
    command.env_clear();
    command.envs(env_policy.filtered_env());
    command.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn()?;
    if let Some(input) = stdin {
        if let Some(mut child_stdin) = child.stdin.take() {
            child_stdin.write_all(input.as_bytes()).await?;
        }
    }
    let output = timeout(APP_SERVER_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| ReviewDeskError::AiBlocked("codex_process_timeout".to_string()))??;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ReviewDeskError::AiBlocked(sanitize_codex_diagnostics(
            &stderr,
        )))
    }
}

fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}
