use crate::ai::AiProvider;
use crate::domain::{ChangedFile, Result, ReviewDraft, ReviewEvent, ReviewRun, ReviewRunStatus};
use chrono::Utc;
use globset::{Glob, GlobSetBuilder};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};

static ANALYSIS_RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewInput {
    pub pr_ref: String,
    pub files: Vec<ChangedFile>,
    pub context_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewOutput {
    pub summary: String,
    pub findings: Vec<String>,
    pub checklist: Vec<String>,
    pub draft_body: String,
    pub suggested_event: ReviewEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilteredFiles {
    pub included: Vec<ChangedFile>,
    pub excluded: Vec<ExcludedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcludedFile {
    pub path: String,
    pub reason: String,
}

pub fn filter_changed_files(files: &[ChangedFile], patterns: &[&str]) -> Result<FilteredFiles> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    let set = builder.build()?;

    let mut included = Vec::new();
    let mut excluded = Vec::new();

    for file in files {
        if set.is_match(&file.path) {
            excluded.push(ExcludedFile {
                path: file.path.clone(),
                reason: "matched_ignore_pattern".to_string(),
            });
        } else {
            included.push(file.clone());
        }
    }

    Ok(FilteredFiles { included, excluded })
}

pub fn build_review_input(
    pr_ref: impl Into<String>,
    files: Vec<ChangedFile>,
    context_notes: Vec<String>,
) -> ReviewInput {
    ReviewInput {
        pr_ref: pr_ref.into(),
        files,
        context_notes,
    }
}

#[derive(Debug, Clone)]
pub struct ReviewPipeline {
    prompt_version: String,
}

impl Default for ReviewPipeline {
    fn default() -> Self {
        Self {
            prompt_version: "reviewdesk-0.0.9".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineResult {
    pub status: ReviewRunStatus,
    pub run: ReviewRun,
    pub draft: ReviewDraft,
    pub markdown: String,
}

impl ReviewPipeline {
    pub async fn generate_or_block<P: AiProvider>(
        &self,
        provider: &P,
        input: ReviewInput,
    ) -> Result<PipelineResult> {
        let now = Utc::now();
        let (owner, repo, number) = parse_pr_ref(&input.pr_ref);
        let run_id = new_analysis_run_id(&owner, &repo, number);
        let mut run = ReviewRun {
            id: run_id.clone(),
            owner,
            repo,
            number,
            snapshot_id: None,
            head_sha: String::new(),
            diff_hash: Some(stable_changed_files_hash(&input.files)),
            status: ReviewRunStatus::Analyzing,
            risk_level: None,
            suggested_verdict: None,
            summary: None,
            prompt_version: self.prompt_version.clone(),
            provider: "chatgpt-oauth".to_string(),
            model: None,
            reasoning_depth: None,
            error: None,
            created_at: now,
            completed_at: None,
        };

        match provider.auth_status().await? {
            crate::domain::AiProviderStatus::Ready => {
                let output = provider.generate_review(input.clone()).await?;
                run.status = ReviewRunStatus::DraftReady;
                run.summary = Some(output.summary.clone());
                run.suggested_verdict = Some(output.suggested_event);
                run.completed_at = Some(Utc::now());

                let draft = ReviewDraft {
                    draft_id: format!("draft-{}", run.id),
                    owner: run.owner.clone(),
                    repo: run.repo.clone(),
                    number: run.number,
                    source_run_ids: vec![run.id.clone()],
                    base_head_sha: run.head_sha.clone(),
                    base_diff_hash: run.diff_hash.clone().unwrap_or_default(),
                    verdict: output.suggested_event,
                    body: output.draft_body.clone(),
                    inline_comments: vec![],
                    user_edited: false,
                    stale: false,
                    created_at: now.to_rfc3339(),
                    updated_at: now.to_rfc3339(),
                };
                Ok(PipelineResult {
                    status: ReviewRunStatus::DraftReady,
                    markdown: markdown_report(&input, Some(&output), None),
                    run,
                    draft,
                })
            }
            blocked => {
                let reason = match blocked {
                    crate::domain::AiProviderStatus::BlockedUnsupportedAuth => {
                        "unsupported auth".to_string()
                    }
                    crate::domain::AiProviderStatus::AuthRequired => "auth required".to_string(),
                    crate::domain::AiProviderStatus::ModelListUnavailable => {
                        "model list unavailable".to_string()
                    }
                    crate::domain::AiProviderStatus::Ready => unreachable!(),
                };
                run.status = ReviewRunStatus::AiBlockedUnsupportedAuth;
                run.error = Some(reason.clone());
                run.completed_at = Some(Utc::now());
                let draft = ReviewDraft {
                    draft_id: format!("draft-{}", run.id),
                    owner: run.owner.clone(),
                    repo: run.repo.clone(),
                    number: run.number,
                    source_run_ids: vec![run.id.clone()],
                    base_head_sha: run.head_sha.clone(),
                    base_diff_hash: run.diff_hash.clone().unwrap_or_default(),
                    verdict: ReviewEvent::Comment,
                    body: String::new(),
                    inline_comments: vec![],
                    user_edited: false,
                    stale: false,
                    created_at: now.to_rfc3339(),
                    updated_at: now.to_rfc3339(),
                };
                Ok(PipelineResult {
                    status: ReviewRunStatus::AiBlockedUnsupportedAuth,
                    markdown: markdown_report(&input, None, Some(&reason)),
                    run,
                    draft,
                })
            }
        }
    }
}

fn markdown_report(
    input: &ReviewInput,
    output: Option<&ReviewOutput>,
    blocked_reason: Option<&str>,
) -> String {
    let mut report = format!("# ReviewDesk Report\n\nPR: {}\n\n", input.pr_ref);
    report.push_str("## Files\n\n");
    for file in &input.files {
        report.push_str(&format!("- `{}`\n", file.path));
    }
    if let Some(output) = output {
        report.push_str("\n## Summary\n\n");
        report.push_str(&output.summary);
        report.push_str("\n\n## Draft\n\n");
        report.push_str(&output.draft_body);
        report.push('\n');
    }
    if let Some(reason) = blocked_reason {
        report.push_str("\n## AI Blocked\n\n");
        report.push_str(reason);
        report.push('\n');
    }
    report
}

fn parse_pr_ref(pr_ref: &str) -> (String, String, u64) {
    let normalized = pr_ref.replace("/pull/", "#");
    let mut owner_repo_and_number = normalized.split('#');
    let owner_repo = owner_repo_and_number.next().unwrap_or_default();
    let number = owner_repo_and_number
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_default();
    let mut pieces = owner_repo.split('/');
    let owner = pieces.next().unwrap_or_default().to_string();
    let repo = pieces.next().unwrap_or_default().to_string();
    (owner, repo, number)
}

pub fn new_analysis_run_id(owner: &str, repo: &str, number: u64) -> String {
    let now = Utc::now();
    let timestamp = now
        .timestamp_nanos_opt()
        .map_or_else(|| now.timestamp_micros() * 1_000, |value| value);
    let counter = ANALYSIS_RUN_COUNTER.fetch_add(1, Ordering::Relaxed);
    let entropy = run_id_entropy(owner, repo, number, timestamp, counter);

    format!(
        "run-{}-{}-{}-{}-{}-{}",
        safe_id_segment(owner),
        safe_id_segment(repo),
        number,
        timestamp,
        counter,
        entropy
    )
}

pub fn stable_changed_files_hash(files: &[ChangedFile]) -> String {
    let mut hasher = Sha256::new();
    let mut ordered_files = files.iter().collect::<Vec<_>>();
    ordered_files.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.patch.cmp(&right.patch))
    });

    for file in ordered_files {
        hasher.update(b"file\0path\0");
        hasher.update((file.path.len() as u64).to_be_bytes());
        hasher.update(file.path.as_bytes());
        hasher.update(b"\0patch\0");
        match &file.patch {
            Some(patch) => {
                hasher.update(b"some\0");
                hasher.update((patch.len() as u64).to_be_bytes());
                hasher.update(patch.as_bytes());
            }
            None => hasher.update(b"none\0"),
        }
    }
    format!("{:x}", hasher.finalize())
}

fn safe_id_segment(value: &str) -> String {
    let segment = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    if segment.is_empty() {
        "unknown".to_string()
    } else {
        segment
    }
}

fn run_id_entropy(owner: &str, repo: &str, number: u64, timestamp: i64, counter: u64) -> String {
    let mut bytes = [0_u8; 8];
    if getrandom::fill(&mut bytes).is_ok() {
        return format!("{:016x}", u64::from_be_bytes(bytes));
    }

    let mut hasher = Sha256::new();
    hasher.update(owner.as_bytes());
    hasher.update(b"\0");
    hasher.update(repo.as_bytes());
    hasher.update(b"\0");
    hasher.update(number.to_be_bytes());
    hasher.update(timestamp.to_be_bytes());
    hasher.update(counter.to_be_bytes());
    hasher.update(std::process::id().to_be_bytes());
    hasher.update(format!("{:?}", std::thread::current().id()).as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}
