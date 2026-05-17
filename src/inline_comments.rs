use crate::domain::{ChangedFile, InlineCommentDraft, InlineMappingStatus, Result};

pub fn validate_inline_comments(
    comments: &[InlineCommentDraft],
    files: &[ChangedFile],
    diff_hash: &str,
) -> Result<Vec<InlineCommentDraft>> {
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

    if !right_side_line_maps(patch, comment.line) {
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
        if start_line > comment.line || !right_side_line_maps(patch, start_line) {
            return InlineMappingStatus::InvalidLine;
        }
    }

    InlineMappingStatus::Valid
}

fn right_side_line_maps(patch: &str, target_line: u64) -> bool {
    let mut new_line = None;

    for patch_line in patch.lines() {
        if patch_line.starts_with("@@") {
            new_line = parse_new_start_line(patch_line);
            continue;
        }

        let Some(current_new_line) = new_line else {
            continue;
        };

        match patch_line.as_bytes().first().copied() {
            Some(b' ') => {
                if current_new_line == target_line {
                    return true;
                }
                new_line = current_new_line.checked_add(1);
            }
            Some(b'+') => {
                if current_new_line == target_line {
                    return true;
                }
                new_line = current_new_line.checked_add(1);
            }
            Some(b'-') => {}
            Some(b'\\') => {}
            _ => {}
        }
    }

    false
}

fn parse_new_start_line(hunk_header: &str) -> Option<u64> {
    let plus_index = hunk_header.find('+')?;
    let new_range = hunk_header[plus_index + 1..].split_whitespace().next()?;
    let start = new_range.split(',').next()?;
    start.parse().ok()
}
