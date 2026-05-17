import type { InlineCommentDraftView } from "./workspace-view-models";

export type { InlineCommentDraftView };

export function editInlineComment(
  comment: InlineCommentDraftView,
  updates: Partial<Omit<InlineCommentDraftView, "id">>,
): InlineCommentDraftView {
  return {
    ...comment,
    ...updates,
  };
}

export function editInlineCommentBody(
  comments: InlineCommentDraftView[],
  id: string,
  body: string,
): InlineCommentDraftView[] {
  return updateInlineComment(comments, id, (comment) => ({
    ...comment,
    body,
    user_edited: true,
  }));
}

export function selectInlineComment(
  comment: InlineCommentDraftView,
  selected = true,
): InlineCommentDraftView {
  return {
    ...comment,
    selected_for_publish: selected,
  };
}

export function deselectInlineComment(comment: InlineCommentDraftView): InlineCommentDraftView {
  return selectInlineComment(comment, false);
}

export function setInlineCommentSelection(
  comments: InlineCommentDraftView[],
  id: string,
  selected: boolean,
): InlineCommentDraftView[] {
  return updateInlineComment(comments, id, (comment) => selectInlineComment(comment, selected));
}

export function dismissInlineComment(comment: InlineCommentDraftView): InlineCommentDraftView {
  return {
    ...comment,
    dismissed: true,
  };
}

export function restoreInlineComment(commentOrComments: InlineCommentDraftView): InlineCommentDraftView;
export function restoreInlineComment(comments: InlineCommentDraftView[], id: string): InlineCommentDraftView[];
export function restoreInlineComment(
  commentOrComments: InlineCommentDraftView | InlineCommentDraftView[],
  id?: string,
): InlineCommentDraftView | InlineCommentDraftView[] {
  if (Array.isArray(commentOrComments)) {
    return updateInlineComment(commentOrComments, requiredId(id), restoreInlineComment);
  }
  return {
    ...commentOrComments,
    dismissed: false,
  };
}

export function dismissInlineCommentById(
  comments: InlineCommentDraftView[],
  id: string,
): InlineCommentDraftView[] {
  return updateInlineComment(comments, id, dismissInlineComment);
}

export function publishableInlineComments(comments: InlineCommentDraftView[]): InlineCommentDraftView[] {
  return comments.filter(isPublishableInlineComment);
}

export function invalidSelectedInlineComments(comments: InlineCommentDraftView[]): InlineCommentDraftView[] {
  return comments.filter(hasInvalidSelectedMapping);
}

export function isPublishableInlineComment(comment: InlineCommentDraftView): boolean {
  return (
    comment.selected_for_publish &&
    !comment.dismissed &&
    comment.mapping_status === "valid" &&
    comment.body.trim().length > 0
  );
}

export function hasInvalidSelectedMapping(comment: InlineCommentDraftView): boolean {
  return (
    comment.selected_for_publish &&
    !comment.dismissed &&
    comment.body.trim().length > 0 &&
    comment.mapping_status !== "valid"
  );
}

export function updateInlineComment(
  comments: InlineCommentDraftView[],
  id: string,
  update: (comment: InlineCommentDraftView) => InlineCommentDraftView,
): InlineCommentDraftView[] {
  return comments.map((comment) => (comment.id === id ? update(comment) : comment));
}

function requiredId(id: string | undefined): string {
  if (!id) {
    throw new Error("Inline comment id is required.");
  }
  return id;
}
