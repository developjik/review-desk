use reviewdesk::domain::{ChangedFile, InlineCommentDraft, InlineMappingStatus};
use reviewdesk::inline_comments::validate_inline_comments;

const PATCH: &str = "\
@@ -10,4 +10,5 @@ fn example() {
 context before
-old line
+new line
 context after
+extra new line
}";

fn draft(
    id: &str,
    path: &str,
    side: &str,
    line: u64,
    selected_for_publish: bool,
    dismissed: bool,
) -> InlineCommentDraft {
    InlineCommentDraft {
        id: id.to_string(),
        path: path.to_string(),
        side: side.to_string(),
        line,
        start_line: None,
        start_side: None,
        body: "inline note".to_string(),
        severity: None,
        confidence: None,
        source_run_id: None,
        source_finding_id: None,
        selected_for_publish,
        dismissed,
        user_edited: false,
        mapping_status: InlineMappingStatus::StaleDiff,
    }
}

#[test]
fn inline_comment_selected_right_side_comment_on_new_side_line_is_valid() {
    let comments = vec![draft("valid", "src/lib.rs", "RIGHT", 11, true, false)];
    let files = vec![ChangedFile::new("src/lib.rs", Some(PATCH))];

    let validated = validate_inline_comments(&comments, &files, "diff-hash").unwrap();

    assert_eq!(validated[0].mapping_status, InlineMappingStatus::Valid);
}

#[test]
fn inline_comment_selected_comment_for_missing_path_is_missing_path() {
    let comments = vec![draft("missing", "src/missing.rs", "RIGHT", 11, true, false)];
    let files = vec![ChangedFile::new("src/lib.rs", Some(PATCH))];

    let validated = validate_inline_comments(&comments, &files, "diff-hash").unwrap();

    assert_eq!(
        validated[0].mapping_status,
        InlineMappingStatus::MissingPath
    );
}

#[test]
fn inline_comment_selected_non_right_side_comment_is_invalid_side() {
    let comments = vec![draft("left", "src/lib.rs", "LEFT", 11, true, false)];
    let files = vec![ChangedFile::new("src/lib.rs", Some(PATCH))];

    let validated = validate_inline_comments(&comments, &files, "diff-hash").unwrap();

    assert_eq!(
        validated[0].mapping_status,
        InlineMappingStatus::InvalidSide
    );
}

#[test]
fn inline_comment_selected_right_side_comment_outside_diff_is_invalid_line() {
    let comments = vec![draft("outside", "src/lib.rs", "RIGHT", 99, true, false)];
    let files = vec![ChangedFile::new("src/lib.rs", Some(PATCH))];

    let validated = validate_inline_comments(&comments, &files, "diff-hash").unwrap();

    assert_eq!(
        validated[0].mapping_status,
        InlineMappingStatus::InvalidLine
    );
}

#[test]
fn inline_comment_non_selected_or_dismissed_comments_stay_publish_safe_as_valid() {
    let comments = vec![
        draft("not-selected", "src/missing.rs", "LEFT", 99, false, false),
        draft("dismissed", "src/missing.rs", "LEFT", 99, true, true),
    ];
    let files = vec![ChangedFile::new("src/lib.rs", Some(PATCH))];

    let validated = validate_inline_comments(&comments, &files, "diff-hash").unwrap();

    assert_eq!(validated[0].mapping_status, InlineMappingStatus::Valid);
    assert_eq!(validated[1].mapping_status, InlineMappingStatus::Valid);
}

#[test]
fn inline_comment_selected_right_side_same_side_range_is_valid() {
    let mut comment = draft("range", "src/lib.rs", "RIGHT", 13, true, false);
    comment.start_line = Some(11);
    comment.start_side = Some("RIGHT".to_string());
    let files = vec![ChangedFile::new("src/lib.rs", Some(PATCH))];

    let validated = validate_inline_comments(&[comment], &files, "diff-hash").unwrap();

    assert_eq!(validated[0].mapping_status, InlineMappingStatus::Valid);
}

#[test]
fn inline_comment_selected_range_with_invalid_start_side_or_order_is_invalid_line() {
    let mut bad_start_side = draft("bad-start-side", "src/lib.rs", "RIGHT", 13, true, false);
    bad_start_side.start_line = Some(11);
    bad_start_side.start_side = Some("LEFT".to_string());

    let mut reversed = draft("reversed", "src/lib.rs", "RIGHT", 11, true, false);
    reversed.start_line = Some(13);

    let files = vec![ChangedFile::new("src/lib.rs", Some(PATCH))];

    let validated =
        validate_inline_comments(&[bad_start_side, reversed], &files, "diff-hash").unwrap();

    assert_eq!(
        validated[0].mapping_status,
        InlineMappingStatus::InvalidLine
    );
    assert_eq!(
        validated[1].mapping_status,
        InlineMappingStatus::InvalidLine
    );
}
