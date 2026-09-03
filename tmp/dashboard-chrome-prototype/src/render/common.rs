use crate::fixture::{Session, Status};

pub const CARD_SLOT_WIDTH: u16 = 26;
pub const CARD_GAP: u16 = 2;
pub const PROJECT_GAP: u16 = 2;
pub const ROW_GAP: u16 = 1;

/// Truncate to `max` visible chars, replacing the tail with an ellipsis when cut.
pub fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    if max == 1 {
        return "…".to_string();
    }
    let mut out: String = chars[..max - 1].iter().collect();
    out.push('…');
    out
}

pub fn nickname_display(session: &Session) -> String {
    if session.subagent {
        format!("↳ {}", session.nickname)
    } else {
        session.nickname.to_string()
    }
}

pub fn status_line2(status: &Status) -> String {
    match status {
        Status::Running { elapsed, .. } => format!("running · {elapsed}"),
        Status::NeedsYouQuestion { .. } => "needs-you · question".to_string(),
        Status::NeedsYouPlain { elapsed } => format!("needs-you · {elapsed}"),
    }
}

pub fn status_line3(status: &Status) -> Option<String> {
    match status {
        Status::Running { action, .. } => Some(action.to_string()),
        Status::NeedsYouQuestion { last_line } => last_line.map(|s| s.to_string()),
        Status::NeedsYouPlain { .. } => None,
    }
}
