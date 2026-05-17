use reviewdesk::domain::{
    InlineCommentDraft, InlineMappingStatus, ReviewEvent, ReviewPublishPayload,
};
use reviewdesk::publish::{PreparedReviewPublishStatus, prepare_review_publish};

fn payload() -> ReviewPublishPayload {
    ReviewPublishPayload {
        owner: "company".to_string(),
        repo: "payment-web".to_string(),
        number: 582,
        expected_head_sha: "head-a".to_string(),
        expected_diff_hash: "diff-a".to_string(),
        event: ReviewEvent::Comment,
        body: "review body".to_string(),
        inline_comments: Vec::new(),
        explicit_verdict_confirmed: false,
        private_diff_consent_required: false,
        private_diff_consent_accepted: false,
    }
}

fn inline_comment(id: &str) -> InlineCommentDraft {
    InlineCommentDraft {
        id: id.to_string(),
        path: "src/lib.rs".to_string(),
        side: "RIGHT".to_string(),
        line: 42,
        start_line: None,
        start_side: None,
        body: "inline note".to_string(),
        severity: None,
        confidence: None,
        source_run_id: None,
        source_finding_id: None,
        selected_for_publish: true,
        dismissed: false,
        user_edited: false,
        mapping_status: InlineMappingStatus::Valid,
    }
}

fn prepare(payload: ReviewPublishPayload) -> reviewdesk::publish::PreparedReviewPublishView {
    prepare_review_publish(payload, true, true, false, true, false, "head-a", "diff-a")
}

#[test]
fn publish_empty_body_is_allowed_when_selected_valid_inline_comment_exists() {
    let mut payload = payload();
    payload.body = "  ".to_string();
    payload.inline_comments = vec![inline_comment("inline-a")];

    let view = prepare(payload);

    assert_eq!(view.status, PreparedReviewPublishStatus::Ready);
    assert!(view.blocked_reasons.is_empty());
    assert!(view.confirmation_id.is_some());
}

#[test]
fn publish_invalid_selected_inline_mapping_blocks() {
    let mut payload = payload();
    let mut comment = inline_comment("inline-a");
    comment.mapping_status = InlineMappingStatus::InvalidLine;
    payload.inline_comments = vec![comment];

    let view = prepare(payload);

    assert_eq!(view.status, PreparedReviewPublishStatus::Blocked);
    assert_eq!(view.blocked_reasons, vec!["inline_mapping_invalid"]);
    assert!(view.confirmation_id.is_none());
}

#[test]
fn publish_selected_blank_invalid_inline_mapping_does_not_block_top_level_review() {
    let mut payload = payload();
    payload.body = "top level review".to_string();
    let mut comment = inline_comment("blank-invalid");
    comment.body = "  ".to_string();
    comment.mapping_status = InlineMappingStatus::InvalidLine;
    payload.inline_comments = vec![comment];

    let view = prepare(payload);

    assert_eq!(view.status, PreparedReviewPublishStatus::Ready);
    assert!(view.blocked_reasons.is_empty());
    assert!(view.confirmation_id.is_some());
}

#[test]
fn publish_confirmation_id_changes_when_body_changes() {
    let mut first = payload();
    let mut second = first.clone();
    first.body = "first body".to_string();
    second.body = "second body".to_string();

    let first_id = prepare(first).confirmation_id.expect("first confirmation");
    let second_id = prepare(second)
        .confirmation_id
        .expect("second confirmation");

    assert_ne!(first_id, second_id);
}

#[test]
fn publish_confirmation_id_changes_when_selected_inline_comment_body_path_or_line_changes() {
    let mut base = payload();
    base.inline_comments = vec![inline_comment("inline-a")];
    let base_id = prepare(base.clone()).confirmation_id.expect("base");

    let mut body_changed = base.clone();
    body_changed.inline_comments[0].body = "different inline body".to_string();
    assert_ne!(
        base_id,
        prepare(body_changed).confirmation_id.expect("body changed")
    );

    let mut path_changed = base.clone();
    path_changed.inline_comments[0].path = "src/other.rs".to_string();
    assert_ne!(
        base_id,
        prepare(path_changed).confirmation_id.expect("path changed")
    );

    let mut line_changed = base;
    line_changed.inline_comments[0].line = 43;
    assert_ne!(
        base_id,
        prepare(line_changed).confirmation_id.expect("line changed")
    );
}

#[test]
fn publish_diff_hash_mismatch_blocks_with_diff_changed() {
    let view = prepare_review_publish(
        payload(),
        true,
        true,
        false,
        true,
        false,
        "head-a",
        "diff-b",
    );

    assert_eq!(view.status, PreparedReviewPublishStatus::Blocked);
    assert!(view.blocked_reasons.contains(&"diff_changed".to_string()));
}

#[test]
fn publish_empty_body_plus_only_dismissed_unselected_or_blank_inline_comments_blocks_payload_empty()
{
    let mut payload = payload();
    payload.body = " ".to_string();

    let mut dismissed = inline_comment("dismissed");
    dismissed.dismissed = true;

    let mut unselected = inline_comment("unselected");
    unselected.selected_for_publish = false;

    let mut blank = inline_comment("blank");
    blank.body = "  ".to_string();

    payload.inline_comments = vec![dismissed, unselected, blank];

    let view = prepare(payload);

    assert_eq!(view.status, PreparedReviewPublishStatus::Blocked);
    assert_eq!(view.blocked_reasons, vec!["payload_empty"]);
}
