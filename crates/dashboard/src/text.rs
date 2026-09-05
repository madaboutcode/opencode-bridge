//! Shared, provider-agnostic text helpers. Nothing here knows any harness's
//! tool-name vocabulary or wire shape. `collapse_newlines`/`basename` are
//! neutral rendering mechanics reused by more than one adapter's action-line
//! rendering; `looks_like_question` is the R6.7 needs-you question-badge
//! heuristic (`docs/specs/dashboard/visuals.md`), consumed on the
//! attention-state path (`opencode/reconcile.rs`), not a rendering path.

use std::path::Path;

const QUESTION_PHRASES: &[&str] = &[
    "which",
    "should i",
    "do you want",
    "would you like",
    "let me know",
    "please confirm",
    "can you confirm",
];

/// R6.7: "ends in `?`, or matches a short phrase list."
pub(crate) fn looks_like_question(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.ends_with('?') {
        return true;
    }
    let lower = trimmed.to_lowercase();
    QUESTION_PHRASES.iter().any(|phrase| lower.contains(phrase))
}

pub(crate) fn collapse_newlines(text: &str) -> String {
    text.lines().collect::<Vec<_>>().join(" · ")
}

pub(crate) fn basename(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ends_with_question_mark_is_a_question() {
        assert!(looks_like_question(
            "Which file would you like me to delete?"
        ));
    }

    #[test]
    fn phrase_match_without_question_mark_is_a_question() {
        assert!(looks_like_question(
            "Let me know which approach you prefer before I continue."
        ));
    }

    #[test]
    fn plain_summary_is_not_a_question() {
        assert!(!looks_like_question("Done — ran the tests, all 12 passed."));
    }

    #[test]
    fn empty_text_is_not_a_question() {
        assert!(!looks_like_question(""));
        assert!(!looks_like_question("   "));
    }
}
