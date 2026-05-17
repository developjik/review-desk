use crate::domain::{AiModel, AiProviderStatus, ReasoningDepth, Result, ReviewDeskError};
use crate::review::{ReviewInput, ReviewOutput};
use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn auth_status(&self) -> Result<AiProviderStatus>;
    async fn list_models(&self) -> Result<Vec<AiModel>>;
    async fn list_reasoning_depths(&self, model: &str) -> Result<Vec<ReasoningDepth>>;
    async fn generate_review(&self, input: ReviewInput) -> Result<ReviewOutput>;
}

#[derive(Debug, Clone)]
pub struct BlockedAiProvider {
    reason: String,
}

impl BlockedAiProvider {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[async_trait]
impl AiProvider for BlockedAiProvider {
    async fn auth_status(&self) -> Result<AiProviderStatus> {
        Ok(AiProviderStatus::BlockedUnsupportedAuth)
    }

    async fn list_models(&self) -> Result<Vec<AiModel>> {
        Err(ReviewDeskError::AiBlocked(self.reason.clone()))
    }

    async fn list_reasoning_depths(&self, _model: &str) -> Result<Vec<ReasoningDepth>> {
        Err(ReviewDeskError::AiBlocked(self.reason.clone()))
    }

    async fn generate_review(&self, _input: ReviewInput) -> Result<ReviewOutput> {
        Err(ReviewDeskError::AiBlocked(self.reason.clone()))
    }
}

#[derive(Debug, Clone)]
pub struct MockAiProvider {
    model: AiModel,
}

impl Default for MockAiProvider {
    fn default() -> Self {
        Self {
            model: AiModel {
                id: "mock-gpt".to_string(),
                display_name: "Mock GPT".to_string(),
                supported_reasoning_depths: vec![
                    ReasoningDepth::Fast,
                    ReasoningDepth::Balanced,
                    ReasoningDepth::Deep,
                ],
                default_reasoning_depth: Some(ReasoningDepth::Balanced),
                unavailable_reason: None,
            },
        }
    }
}

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn auth_status(&self) -> Result<AiProviderStatus> {
        Ok(AiProviderStatus::Ready)
    }

    async fn list_models(&self) -> Result<Vec<AiModel>> {
        Ok(vec![self.model.clone()])
    }

    async fn list_reasoning_depths(&self, _model: &str) -> Result<Vec<ReasoningDepth>> {
        Ok(self.model.supported_reasoning_depths.clone())
    }

    async fn generate_review(&self, input: ReviewInput) -> Result<ReviewOutput> {
        let file_list = input
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Ok(ReviewOutput {
            summary: format!("Mock review for {}", input.pr_ref),
            findings: vec![format!("Reviewed files: {file_list}")],
            checklist: input.context_notes,
            draft_body:
                "Mock review draft. Please verify the highlighted changes before submitting."
                    .to_string(),
            suggested_event: crate::domain::ReviewEvent::Comment,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CodexAppServerProvider {
    codex_path: PathBuf,
}

impl Default for CodexAppServerProvider {
    fn default() -> Self {
        Self {
            codex_path: PathBuf::from("codex"),
        }
    }
}

impl CodexAppServerProvider {
    pub fn new(codex_path: impl Into<PathBuf>) -> Self {
        Self {
            codex_path: codex_path.into(),
        }
    }

    pub fn is_command_available(&self) -> bool {
        std::process::Command::new(&self.codex_path)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[async_trait]
impl AiProvider for CodexAppServerProvider {
    async fn auth_status(&self) -> Result<AiProviderStatus> {
        if self.is_command_available() {
            Ok(AiProviderStatus::AuthRequired)
        } else {
            Ok(AiProviderStatus::BlockedUnsupportedAuth)
        }
    }

    async fn list_models(&self) -> Result<Vec<AiModel>> {
        if self.is_command_available() {
            Ok(Vec::new())
        } else {
            Err(ReviewDeskError::AiBlocked(
                "codex app-server command unavailable".to_string(),
            ))
        }
    }

    async fn list_reasoning_depths(&self, _model: &str) -> Result<Vec<ReasoningDepth>> {
        if self.is_command_available() {
            Ok(vec![
                ReasoningDepth::Fast,
                ReasoningDepth::Balanced,
                ReasoningDepth::Deep,
            ])
        } else {
            Err(ReviewDeskError::AiBlocked(
                "codex app-server command unavailable".to_string(),
            ))
        }
    }

    async fn generate_review(&self, _input: ReviewInput) -> Result<ReviewOutput> {
        Err(ReviewDeskError::AiBlocked(
            "Codex app-server generation is not initialized in this build".to_string(),
        ))
    }
}
