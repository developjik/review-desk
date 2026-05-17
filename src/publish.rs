use crate::domain::{InlineCommentDraft, InlineMappingStatus, ReviewEvent, ReviewPublishPayload};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedReviewPublishStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedReviewPublishView {
    pub status: PreparedReviewPublishStatus,
    pub blocked_reasons: Vec<String>,
    pub confirmation_id: Option<String>,
    pub payload: ReviewPublishPayload,
}

pub fn prepare_review_publish(
    payload: ReviewPublishPayload,
    github_connected: bool,
    write_scope_valid: bool,
    sso_required: bool,
    pr_open: bool,
    pr_merged: bool,
    current_head_sha: &str,
    current_diff_hash: &str,
) -> PreparedReviewPublishView {
    let mut blocked_reasons = Vec::new();

    if !github_connected {
        blocked_reasons.push("github_auth_required".to_string());
    }
    if !write_scope_valid {
        blocked_reasons.push("github_scope_insufficient".to_string());
    }
    if sso_required {
        blocked_reasons.push("github_sso_required".to_string());
    }
    if !pr_open {
        blocked_reasons.push("pr_closed".to_string());
    }
    if pr_merged {
        blocked_reasons.push("pr_merged".to_string());
    }
    if payload.expected_head_sha != current_head_sha {
        blocked_reasons.push("head_changed".to_string());
    }
    if payload.expected_diff_hash != current_diff_hash {
        blocked_reasons.push("diff_changed".to_string());
    }
    if payload
        .inline_comments
        .iter()
        .any(is_selected_not_dismissed_invalid_mapping)
    {
        blocked_reasons.push("inline_mapping_invalid".to_string());
    }
    if payload.body.trim().is_empty() && !payload.inline_comments.iter().any(is_publishable_inline)
    {
        blocked_reasons.push("payload_empty".to_string());
    }
    if payload.event != ReviewEvent::Comment && !payload.explicit_verdict_confirmed {
        blocked_reasons.push("explicit_verdict_confirmation_required".to_string());
    }
    if payload.private_diff_consent_required && !payload.private_diff_consent_accepted {
        blocked_reasons.push("private_diff_consent_required".to_string());
    }

    if blocked_reasons.is_empty() {
        PreparedReviewPublishView {
            confirmation_id: Some(confirmation_id(&payload)),
            status: PreparedReviewPublishStatus::Ready,
            blocked_reasons,
            payload,
        }
    } else {
        PreparedReviewPublishView {
            status: PreparedReviewPublishStatus::Blocked,
            blocked_reasons,
            confirmation_id: None,
            payload,
        }
    }
}

fn is_selected_not_dismissed(comment: &InlineCommentDraft) -> bool {
    comment.selected_for_publish && !comment.dismissed
}

fn is_selected_not_dismissed_invalid_mapping(comment: &InlineCommentDraft) -> bool {
    is_selected_not_dismissed(comment)
        && !comment.body.trim().is_empty()
        && comment.mapping_status != InlineMappingStatus::Valid
}

fn is_publishable_inline(comment: &InlineCommentDraft) -> bool {
    is_selected_not_dismissed(comment)
        && comment.mapping_status == InlineMappingStatus::Valid
        && !comment.body.trim().is_empty()
}

fn confirmation_id(payload: &ReviewPublishPayload) -> String {
    let surface = ConfirmationSurface {
        owner: &payload.owner,
        repo: &payload.repo,
        number: payload.number,
        expected_head_sha: &payload.expected_head_sha,
        expected_diff_hash: &payload.expected_diff_hash,
        event: payload.event,
        body: &payload.body,
        inline_comments: payload
            .inline_comments
            .iter()
            .filter(|comment| is_publishable_inline(comment))
            .map(|comment| ConfirmationInlineComment {
                path: &comment.path,
                side: &comment.side,
                line: comment.line,
                start_line: comment.start_line,
                start_side: comment.start_side.as_deref(),
                body: &comment.body,
            })
            .collect(),
    };
    let mut hasher = Sha256::new();
    let encoded = serde_json::to_vec(&surface).expect("confirmation surface serializes");
    hasher.update(encoded);
    hex_string(&hasher.finalize())
}

#[derive(Serialize)]
struct ConfirmationSurface<'a> {
    owner: &'a str,
    repo: &'a str,
    number: u64,
    expected_head_sha: &'a str,
    expected_diff_hash: &'a str,
    event: ReviewEvent,
    body: &'a str,
    inline_comments: Vec<ConfirmationInlineComment<'a>>,
}

#[derive(Serialize)]
struct ConfirmationInlineComment<'a> {
    path: &'a str,
    side: &'a str,
    line: u64,
    start_line: Option<u64>,
    start_side: Option<&'a str>,
    body: &'a str,
}

fn hex_string(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(TABLE[(byte >> 4) as usize] as char);
        output.push(TABLE[(byte & 0x0f) as usize] as char);
    }
    output
}
