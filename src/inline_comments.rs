use crate::domain::{ChangedFile, InlineCommentDraft, InlineMappingStatus, Result};

pub fn validate_inline_comments(
    comments: &[InlineCommentDraft],
    files: &[ChangedFile],
    diff_hash: &str,
) -> Result<Vec<InlineCommentDraft>> {
    // Retained for the IPC/API shape. Stale diff comparison belongs to publish preflight,
    // where both expected and current hashes are available.
    let _ = diff_hash;

    Ok(comments
        .iter()
        .map(|comment| {
            let mut validated = comment.clone();
            validated.mapping_status = validate_inline_comment(comment, files);
            validated
        })
        .collect())
}

fn validate_inline_comment(
    comment: &InlineCommentDraft,
    files: &[ChangedFile],
) -> InlineMappingStatus {
    if !comment.selected_for_publish || comment.dismissed {
        return InlineMappingStatus::Valid;
    }

    if comment.side != "RIGHT" {
        return InlineMappingStatus::InvalidSide;
    }

    let Some(patch) = files
        .iter()
        .find(|file| file.path == comment.path)
        .and_then(|file| file.patch.as_deref())
    else {
        return InlineMappingStatus::MissingPath;
    };

    let hunks = right_side_hunks(patch);

    if !right_side_line_maps(&hunks, comment.line) {
        return InlineMappingStatus::InvalidLine;
    }

    if let Some(start_line) = comment.start_line {
        if comment
            .start_side
            .as_deref()
            .is_some_and(|side| side != "RIGHT")
        {
            return InlineMappingStatus::InvalidLine;
        }
        if start_line > comment.line || !right_side_range_maps(&hunks, start_line, comment.line) {
            return InlineMappingStatus::InvalidLine;
        }
    }

    InlineMappingStatus::Valid
}

fn right_side_line_maps(hunks: &[Vec<u64>], target_line: u64) -> bool {
    hunks.iter().any(|hunk| hunk.contains(&target_line))
}

fn right_side_range_maps(hunks: &[Vec<u64>], start_line: u64, end_line: u64) -> bool {
    hunks
        .iter()
        .any(|hunk| (start_line..=end_line).all(|line| hunk.contains(&line)))
}

fn right_side_hunks(patch: &str) -> Vec<Vec<u64>> {
    let mut hunks = Vec::new();
    let mut current_hunk = Vec::new();
    let mut new_line = None;

    for patch_line in patch.lines() {
        if patch_line.starts_with("@@") {
            if !current_hunk.is_empty() {
                hunks.push(current_hunk);
                current_hunk = Vec::new();
            }
            new_line = parse_new_start_line(patch_line);
            continue;
        }

        let Some(current_new_line) = new_line else {
            continue;
        };

        match patch_line.as_bytes().first().copied() {
            Some(b' ') => {
                current_hunk.push(current_new_line);
                new_line = current_new_line.checked_add(1);
            }
            Some(b'+') => {
                current_hunk.push(current_new_line);
                new_line = current_new_line.checked_add(1);
            }
            Some(b'-') => {}
            Some(b'\\') => {}
            _ => {}
        }
    }

    if !current_hunk.is_empty() {
        hunks.push(current_hunk);
    }

    hunks
}

fn parse_new_start_line(hunk_header: &str) -> Option<u64> {
    let plus_index = hunk_header.find('+')?;
    let new_range = hunk_header[plus_index + 1..].split_whitespace().next()?;
    let start = new_range.split(',').next()?;
    start.parse().ok()
}
