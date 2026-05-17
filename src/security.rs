use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskFinding {
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskResult {
    pub text: String,
    pub findings: Vec<MaskFinding>,
}

#[derive(Debug, Clone)]
pub struct SecretMasker {
    patterns: Vec<(&'static str, Regex)>,
}

impl Default for SecretMasker {
    fn default() -> Self {
        let patterns = vec![
            (
                "private_key",
                Regex::new(
                    r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----",
                )
                .expect("private key regex"),
            ),
            (
                "bearer_token",
                Regex::new(r"(?i)\bAuthorization:\s*Bearer\s+[A-Za-z0-9._~+/=-]{16,}")
                    .expect("bearer token regex"),
            ),
            (
                "github_token",
                Regex::new(r"\bgh[pousr]_[A-Za-z0-9_]{20,}\b").expect("github token regex"),
            ),
            (
                "oauth_client_secret",
                Regex::new(r"(?i)\b(client_secret=)[^\s&]+").expect("oauth client secret regex"),
            ),
            (
                "oauth_code",
                Regex::new(r"(?i)\b(code=gh-code-[A-Za-z0-9._~-]{12,})").expect("oauth code regex"),
            ),
            (
                "jwt",
                Regex::new(r"\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b")
                    .expect("jwt regex"),
            ),
            (
                "webhook_url",
                Regex::new(r"https://hooks\.[A-Za-z0-9./_-]+").expect("webhook regex"),
            ),
            (
                "env_secret",
                Regex::new(r"(?m)^([A-Z0-9_]*(SECRET|TOKEN|PASSWORD|API_KEY)[A-Z0-9_]*=)[^\s]+")
                    .expect("env secret regex"),
            ),
            (
                "generic_api_key",
                Regex::new(r"\b(sk|pk|rk|api)[-_][A-Za-z0-9_]{12,}\b").expect("api key regex"),
            ),
        ];

        Self { patterns }
    }
}

impl SecretMasker {
    pub fn mask(&self, input: &str) -> MaskResult {
        let mut text = input.to_string();
        let mut findings = Vec::new();

        for (kind, pattern) in &self.patterns {
            if pattern.is_match(&text) {
                findings.push(MaskFinding {
                    kind: (*kind).to_string(),
                });
                text = if *kind == "env_secret" || *kind == "oauth_client_secret" {
                    pattern.replace_all(&text, "${1}[REDACTED]").to_string()
                } else if *kind == "oauth_code" {
                    pattern
                        .replace_all(&text, "code=[REDACTED:oauth_code]")
                        .to_string()
                } else {
                    pattern
                        .replace_all(&text, format!("[REDACTED:{}]", kind))
                        .to_string()
                };
            }
        }

        MaskResult { text, findings }
    }

    pub fn has_high_risk_secret(&self, input: &str) -> bool {
        self.patterns
            .iter()
            .any(|(_, pattern)| pattern.is_match(input))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateDiffConsent {
    pub repository_private: bool,
    pub provider: String,
    pub model: String,
    pub transmitted_scope: String,
    pub accepted: bool,
}

impl PrivateDiffConsent {
    pub fn required(&self) -> bool {
        self.repository_private
    }

    pub fn can_send_to_ai(&self) -> bool {
        !self.required() || self.accepted
    }
}
