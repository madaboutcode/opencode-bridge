//! The needs-you question-badge heuristic — `docs/specs/dashboard/visuals.md`
//! R6.7, explicitly owned by the opencode adapter (`client.md` R1.3: "the
//! needs-you/question-detection heuristic — opencode-specific guesswork
//! with no real wire signal"). Confirmed by
//! `docs/internal/opencode-sse-event-catalog-2026-09-01.md` §5: opencode
//! has no permission-gate or ask-question protocol event, so this is a text
//! heuristic over the final assistant message, checked once when a session
//! transitions into needs-you.
//!
//! Per the delivery profile's deferral posture, this ships as a minimal
//! rule; refinement (a longer/tuned phrase list) is explicitly deferred —
//! false negatives (a real question not badged) are tolerable, the session
//! still shows as plain needs-you.

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
