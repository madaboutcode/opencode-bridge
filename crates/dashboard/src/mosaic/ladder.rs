//! Tile content ladder: a total function of (inner width `wi`, height `h`)
//! per session — `layout.md` R5.3's regime table, ported from the verified
//! spike (`tmp/20260901-prototype-dashboard-layout/src/ladder.rs`). Adapted
//! to read `crate::mosaic::view::SessionView` (real T09/T10 data through
//! `view.rs`) instead of the spike's fixture `Session` — the content rules
//! themselves are unchanged (T11 contract, acceptance criterion 1).
//!
//! One adaptation beyond the field rename: the spike's `status_line`
//! appended a `· N calls` suffix at `wi >= 24`, sourced from a fixture-only
//! `calls: Option<u32>` field that has no counterpart in T09's
//! `SessionSnapshot` (no tool-call-count field exists there) and, on
//! checking, no counterpart in `layout.md`'s R5.3 status-line-forms text
//! either — the table only ever shows `question · 9m` / `needs-you · 22m` /
//! `running · 3m` / `idle · 51m`, never a call count. Dropping it here
//! matches the spec exactly; it was spike-only decoration, not a spec rule
//! to preserve.
//!
//! ASCII-width simplification carried over from the spike: widths are char
//! counts, not unicode display width. Flagged as scope, not fixed, same as
//! the spike's own note.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::mosaic::palette;
use crate::mosaic::view::{SessionView, State, SubagentView};

pub struct TileContent {
    pub lines: Vec<Line<'static>>,
    pub blocks_rendered: Vec<&'static str>,
    pub blank_rows_left: usize,
    pub regime: &'static str,
}

pub(crate) fn truncate_ellipsis(s: &str, w: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if w == 0 {
        return String::new();
    }
    if chars.len() <= w {
        return s.to_string();
    }
    if w == 1 {
        return "…".to_string();
    }
    let mut out: String = chars[..w - 1].iter().collect();
    out.push('…');
    out
}

fn wrap(text: &str, w: usize) -> Vec<String> {
    if w == 0 {
        return vec![];
    }
    let mut lines: Vec<String> = vec![];
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let mut remaining: String = word.to_string();
        loop {
            let rem_chars: Vec<char> = remaining.chars().collect();
            let rem_len = rem_chars.len();
            if rem_len == 0 {
                break;
            }
            let cur_len = cur.chars().count();
            if cur_len == 0 {
                if rem_len <= w {
                    cur = remaining;
                    break;
                } else {
                    let head: String = rem_chars[..w].iter().collect();
                    lines.push(head);
                    remaining = rem_chars[w..].iter().collect();
                    continue;
                }
            } else {
                let candidate = cur_len + 1 + rem_len;
                if candidate <= w {
                    cur.push(' ');
                    cur.push_str(&remaining);
                    break;
                } else {
                    lines.push(std::mem::take(&mut cur));
                    continue;
                }
            }
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn wrap_multiline(text: &str, w: usize) -> Vec<String> {
    let mut out = vec![];
    for raw_line in text.split('\n') {
        if raw_line.trim().is_empty() {
            out.push(String::new());
        } else {
            out.extend(wrap(raw_line, w));
        }
    }
    out
}

/// word-wrap, truncated with `…` on the LAST kept row if it overflows `max_rows`.
fn wrap_capped(text: &str, w: usize, max_rows: usize) -> Vec<String> {
    let lines = wrap(text, w);
    if lines.len() <= max_rows || max_rows == 0 {
        return lines.into_iter().take(max_rows).collect();
    }
    let mut kept: Vec<String> = lines[..max_rows].to_vec();
    if let Some(last) = kept.last_mut() {
        let max_len = w.saturating_sub(1);
        let chars: Vec<char> = last.chars().collect();
        let truncated: String = if chars.len() > max_len {
            chars[..max_len].iter().collect()
        } else {
            last.clone()
        };
        *last = format!("{truncated}…");
    }
    kept
}

/// word-wrap, tail-kept: if it overflows `budget` rows, keep the END and replace the
/// first shown row with `⋯` (the interesting content — the question, the conclusion — is
/// at the end).
fn wrap_tail_multiline(text: &str, w: usize, budget: usize) -> Vec<String> {
    if budget == 0 {
        return vec![];
    }
    let all = wrap_multiline(text, w);
    if all.len() <= budget {
        return all;
    }
    let start = all.len() - budget;
    let mut kept = all[start..].to_vec();
    kept[0] = "⋯".to_string();
    kept
}

fn dim(text: String) -> Line<'static> {
    Line::from(Span::styled(text, Style::new().fg(palette::TEXT_DIM)))
}

fn body(text: String, state: State) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::new().fg(palette::tile_body(state)),
    ))
}

fn plain_lines(strs: Vec<String>, style: Style) -> Vec<Line<'static>> {
    strs.into_iter()
        .map(|s| Line::from(Span::styled(s, style)))
        .collect()
}

fn nick_line(s: &SessionView, wi: u16, tick: usize) -> Line<'static> {
    let glyph = palette::state_glyph(s.state, tick);
    let avail = (wi as usize).saturating_sub(2);
    let nick = truncate_ellipsis(&s.nick, avail);
    let text = format!("{glyph} {nick}");
    let mut style = Style::new()
        .fg(palette::tile_text(s.state))
        .add_modifier(Modifier::BOLD);
    if s.state == State::Idle {
        style = style.add_modifier(Modifier::DIM);
    }
    Line::from(Span::styled(text, style))
}

fn subagent_line(sub: &SubagentView, wi: u16) -> Line<'static> {
    let text = format!("↳ {} {}", sub.nick, sub.action);
    let truncated = truncate_ellipsis(&text, wi as usize);
    Line::from(Span::styled(truncated, Style::new().fg(palette::SUBAGENT)))
}

/// Status line forms (`layout.md` R5.3 "Status line forms"). `subagent_lines_rendered`
/// suppresses the trailing ` ↳N` tail (it only appears when subagent lines have no room
/// of their own elsewhere in the tile).
fn status_line(s: &SessionView, wi: u16, subagent_lines_rendered: bool) -> Line<'static> {
    let wi_i = wi as i64;
    let age = s.age.clone();
    let mut spans: Vec<Span<'static>> = vec![];
    let base_len: i64;

    match s.state {
        State::Question => {
            let text = if wi >= 16 {
                format!("question · {age}")
            } else {
                format!("? {age}")
            };
            let badge = format!(" {text} ");
            base_len = badge.chars().count() as i64;
            spans.push(Span::styled(
                badge,
                Style::new()
                    .fg(palette::GUTTER)
                    .bg(palette::STATUS_QUESTION)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        State::NeedsYou => {
            let text = if wi >= 16 {
                format!("needs-you · {age}")
            } else {
                format!("need · {age}")
            };
            base_len = text.chars().count() as i64;
            spans.push(Span::styled(
                text,
                Style::new().fg(palette::STATUS_NEEDS_YOU),
            ));
        }
        State::Running => {
            let text = if wi >= 16 {
                format!("running · {age}")
            } else {
                format!("run · {age}")
            };
            base_len = text.chars().count() as i64;
            spans.push(Span::styled(text, Style::new().fg(palette::STATUS_RUNNING)));
        }
        State::Idle => {
            let text = format!("idle · {age}");
            base_len = text.chars().count() as i64;
            spans.push(Span::styled(text, Style::new().fg(palette::STATUS_IDLE)));
        }
    }

    if !s.subs.is_empty() && !subagent_lines_rendered {
        let remaining = wi_i - base_len;
        if remaining >= 4 {
            let tail = format!(" ↳{}", s.subs.len());
            spans.push(Span::styled(
                tail,
                Style::new()
                    .fg(palette::SUBAGENT)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    Line::from(spans)
}

fn compact_content_text(s: &SessionView) -> String {
    match s.state {
        State::Question => s
            .assistant_text
            .split('\n')
            .next()
            .unwrap_or("")
            .to_string(),
        State::Running => s.action.clone().unwrap_or_default(),
        State::NeedsYou | State::Idle => s.title.clone(),
    }
}

fn compact(s: &SessionView, wi: u16, h: u16, tick: usize) -> TileContent {
    let mut lines = vec![];
    let mut blocks = vec![];

    lines.push(nick_line(s, wi, tick));
    blocks.push("nick");

    let content = truncate_ellipsis(&compact_content_text(s), wi as usize);
    lines.push(body(content, s.state));
    blocks.push("content");

    let row4_sub = h == 4 && !s.subs.is_empty();
    lines.push(status_line(s, wi, row4_sub));
    blocks.push("status");

    if row4_sub {
        lines.push(subagent_line(&s.subs[0], wi));
        blocks.push("subagent");
    }

    let blank_rows_left = (h as usize).saturating_sub(lines.len());
    TileContent {
        lines,
        blocks_rendered: blocks,
        blank_rows_left,
        regime: "compact",
    }
}

// ---- extended: priority blocks + one elastic block ----

struct Block {
    id: &'static str,
    blank_prefix: bool,
    lines: Vec<Line<'static>>,
}

impl Block {
    fn rows(&self) -> usize {
        if self.lines.is_empty() {
            0
        } else {
            self.lines.len() + if self.blank_prefix { 1 } else { 0 }
        }
    }
    fn renders(&self) -> bool {
        !self.lines.is_empty()
    }
}

/// Runs the fixed-block layout algorithm: keep block[0] (and any other `protected`
/// blocks) always; drop from `droppable` (given lowest-priority-first) while the total
/// exceeds `h`; return the surviving blocks in original order plus the elastic budget.
fn fit_fixed_blocks(
    mut blocks: Vec<Block>,
    protected: usize,
    droppable_order: &[usize],
    h: usize,
) -> (Vec<Block>, usize) {
    let mut dropped: Vec<usize> = vec![];
    loop {
        let total: usize = blocks
            .iter()
            .enumerate()
            .filter(|(i, _)| !dropped.contains(i))
            .map(|(_, b)| b.rows())
            .sum();
        if total <= h {
            let _ = protected;
            let kept: Vec<Block> = blocks
                .drain(..)
                .enumerate()
                .filter(|(i, _)| !dropped.contains(i))
                .map(|(_, b)| b)
                .collect();
            let remaining = h.saturating_sub(kept.iter().map(|b| b.rows()).sum());
            return (kept, remaining);
        }
        match droppable_order.iter().find(|i| !dropped.contains(i)) {
            Some(&i) => dropped.push(i),
            None => {
                // Fixed blocks alone exceed h even after dropping everything droppable —
                // a finding, not silently patched. Keep everything; render() will cap.
                let kept: Vec<Block> = blocks
                    .drain(..)
                    .enumerate()
                    .filter(|(i, _)| !dropped.contains(i))
                    .map(|(_, b)| b)
                    .collect();
                return (kept, 0);
            }
        }
    }
}

fn elastic_budget_split(raw_budget: usize, has_blank_prefix: bool) -> (bool, usize) {
    if !has_blank_prefix {
        return (false, raw_budget);
    }
    if raw_budget >= 2 {
        (true, raw_budget - 1)
    } else if raw_budget == 1 {
        (false, 1) // sacrifice the blank to guarantee >=1 content row
    } else {
        (false, 0)
    }
}

fn recent_actions_elastic(recent: &[String], wi: u16, budget: usize) -> Vec<Line<'static>> {
    if budget == 0 || recent.is_empty() {
        return vec![];
    }
    let k = budget.min(recent.len());
    let start = recent.len() - k;
    let shown = &recent[start..];
    shown
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let color = if i >= shown.len().saturating_sub(2) {
                palette::TEXT_SECONDARY
            } else {
                palette::TEXT_DIM
            };
            let text = truncate_ellipsis(entry, wi as usize);
            Line::from(Span::styled(text, Style::new().fg(color)))
        })
        .collect()
}

fn assemble(blocks: Vec<Block>) -> (Vec<Line<'static>>, Vec<&'static str>) {
    let mut lines = vec![];
    let mut names = vec![];
    for b in blocks {
        if !b.renders() {
            continue;
        }
        if b.blank_prefix {
            lines.push(Line::default());
        }
        names.push(b.id);
        lines.extend(b.lines);
    }
    (lines, names)
}

fn extended_running(s: &SessionView, wi: u16, h: u16, tick: usize) -> TileContent {
    let h = h as usize;
    let action_text = s.action.clone().unwrap_or_default();
    let files_text = if s.files.is_empty() {
        String::new()
    } else {
        format!("files: {}", s.files.join(", "))
    };

    let b1 = Block {
        id: "nick",
        blank_prefix: false,
        lines: vec![nick_line(s, wi, tick)],
    };
    let b2 = Block {
        id: "action",
        blank_prefix: false,
        lines: plain_lines(
            wrap_capped(&action_text, wi as usize, 2),
            Style::new().fg(palette::tile_body(s.state)),
        ),
    };
    let b3_placeholder = Block {
        id: "status",
        blank_prefix: false,
        lines: vec![Line::default()],
    }; // resolved after drop decision
    let b4 = Block {
        id: "subagent",
        blank_prefix: false,
        lines: s.subs.iter().map(|sub| subagent_line(sub, wi)).collect(),
    };
    let b5 = Block {
        id: "title",
        blank_prefix: true,
        lines: plain_lines(
            wrap_capped(&s.title, wi as usize, 2),
            Style::new().fg(palette::TEXT_DIM),
        ),
    };
    let b6 = Block {
        id: "files",
        blank_prefix: true,
        lines: if files_text.is_empty() {
            vec![]
        } else {
            plain_lines(
                wrap_capped(&files_text, wi as usize, 2),
                Style::new().fg(palette::TEXT_DIM),
            )
        },
    };

    // indices: 0=nick 1=action 2=status(placeholder) 3=subagent 4=title 5=files
    let blocks = vec![b1, b2, b3_placeholder, b4, b5, b6];
    let (mut kept, elastic_raw) = fit_fixed_blocks(blocks, 3, &[5, 4, 3], h);

    let subagent_kept = kept.iter().any(|b| b.id == "subagent");
    if let Some(status_block) = kept.iter_mut().find(|b| b.id == "status") {
        status_block.lines = vec![status_line(s, wi, subagent_kept)];
    }

    let (elastic_blank, elastic_content_budget) = elastic_budget_split(elastic_raw, false);
    let _ = elastic_blank; // running's elastic has no blank prefix per the brief
    let elastic_lines = recent_actions_elastic(&s.recent, wi, elastic_content_budget);
    let elastic_rendered = !elastic_lines.is_empty();

    let (mut lines, mut blocks_rendered) = assemble(kept);
    if elastic_rendered {
        lines.extend(elastic_lines);
        blocks_rendered.push("recent");
    }
    lines.truncate(h);

    let blank_rows_left = h.saturating_sub(lines.len());
    TileContent {
        lines,
        blocks_rendered,
        blank_rows_left,
        regime: "extended",
    }
}

fn extended_question(s: &SessionView, wi: u16, h: u16, tick: usize) -> TileContent {
    let h = h as usize;
    let you_text = if s.user_prompt.is_empty() {
        String::new()
    } else {
        format!("you: {}", s.user_prompt)
    };

    let b1 = Block {
        id: "nick",
        blank_prefix: false,
        lines: vec![nick_line(s, wi, tick)],
    };
    let b2 = Block {
        id: "badge",
        blank_prefix: false,
        lines: vec![status_line(s, wi, true)],
    };
    let b3_placeholder = Block {
        id: "assistant",
        blank_prefix: true,
        lines: vec![Line::default()],
    };
    let b4 = Block {
        id: "you",
        blank_prefix: true,
        lines: if you_text.is_empty() {
            vec![]
        } else {
            plain_lines(
                wrap_capped(&you_text, wi as usize, 3),
                Style::new().fg(palette::TEXT_DIM),
            )
        },
    };
    let b5 = Block {
        id: "title",
        blank_prefix: true,
        lines: plain_lines(
            wrap_capped(&s.title, wi as usize, 2),
            Style::new().fg(palette::TEXT_DIM),
        ),
    };
    let b6 = Block {
        id: "subagent",
        blank_prefix: false,
        lines: s.subs.iter().map(|sub| subagent_line(sub, wi)).collect(),
    };

    // indices: 0=nick 1=badge 2=assistant(elastic placeholder) 3=you 4=title 5=subagent
    let blocks = vec![b1, b2, b3_placeholder, b4, b5, b6];
    let (mut kept, elastic_raw) = fit_fixed_blocks(blocks, 2, &[5, 4, 3], h);

    let (elastic_blank, elastic_budget) = elastic_budget_split(elastic_raw, true);
    let elastic_lines: Vec<String> =
        wrap_tail_multiline(&s.assistant_text, wi as usize, elastic_budget);
    if let Some(ab) = kept.iter_mut().find(|b| b.id == "assistant") {
        ab.blank_prefix = elastic_blank;
        ab.lines = plain_lines(
            elastic_lines,
            Style::new().fg(palette::tile_body(State::Question)),
        );
    }

    let (lines_v, blocks_rendered) = assemble(kept);
    let mut lines = lines_v;
    lines.truncate(h);
    let blank_rows_left = h.saturating_sub(lines.len());
    TileContent {
        lines,
        blocks_rendered,
        blank_rows_left,
        regime: "extended",
    }
}

fn extended_needs_you(s: &SessionView, wi: u16, h: u16, tick: usize) -> TileContent {
    let h = h as usize;
    let you_text = if s.user_prompt.is_empty() {
        String::new()
    } else {
        format!("you: {}", s.user_prompt)
    };

    let b1 = Block {
        id: "nick",
        blank_prefix: false,
        lines: vec![nick_line(s, wi, tick)],
    };
    let b2 = Block {
        id: "title",
        blank_prefix: false,
        lines: plain_lines(
            wrap_capped(&s.title, wi as usize, 2),
            Style::new().fg(palette::tile_body(State::NeedsYou)),
        ),
    };
    let b3_placeholder = Block {
        id: "status",
        blank_prefix: false,
        lines: vec![Line::default()],
    };
    let b4 = Block {
        id: "subagent",
        blank_prefix: false,
        lines: s.subs.iter().map(|sub| subagent_line(sub, wi)).collect(),
    };
    let b5_placeholder = Block {
        id: "assistant",
        blank_prefix: true,
        lines: vec![Line::default()],
    };
    let b6 = Block {
        id: "you",
        blank_prefix: true,
        lines: if you_text.is_empty() {
            vec![]
        } else {
            plain_lines(
                wrap_capped(&you_text, wi as usize, 3),
                Style::new().fg(palette::TEXT_DIM),
            )
        },
    };

    // indices: 0=nick 1=title 2=status 3=subagent 4=assistant(elastic) 5=you
    // drop order (generalized from Question's 6,5,4 pattern; the brief only spells this
    // out for Question): highest-numbered non-elastic fixed block first.
    let blocks = vec![b1, b2, b3_placeholder, b4, b5_placeholder, b6];
    let (mut kept, elastic_raw) = fit_fixed_blocks(blocks, 1, &[5, 3, 2, 1], h);

    let subagent_kept = kept.iter().any(|b| b.id == "subagent");
    if let Some(status_block) = kept.iter_mut().find(|b| b.id == "status") {
        status_block.lines = vec![status_line(s, wi, subagent_kept)];
    }

    let (elastic_blank, elastic_budget) = elastic_budget_split(elastic_raw, true);
    let elastic_lines: Vec<String> =
        wrap_tail_multiline(&s.assistant_text, wi as usize, elastic_budget);
    if let Some(ab) = kept.iter_mut().find(|b| b.id == "assistant") {
        ab.blank_prefix = elastic_blank;
        ab.lines = plain_lines(
            elastic_lines,
            Style::new().fg(palette::tile_body(State::NeedsYou)),
        );
    }

    let (lines_v, blocks_rendered) = assemble(kept);
    let mut lines = lines_v;
    lines.truncate(h);
    let blank_rows_left = h.saturating_sub(lines.len());
    TileContent {
        lines,
        blocks_rendered,
        blank_rows_left,
        regime: "extended",
    }
}

/// Spec-complete but currently unreachable via the layout path: idle sessions are pulled
/// out into the chip row before squarify ever runs over tiles (`layout.md` R5.2), so no
/// idle session gets a tile rect. Kept because the regime table is defined as total over
/// all four states.
fn extended_idle(s: &SessionView, wi: u16, h: u16, tick: usize) -> TileContent {
    let h = h as usize;
    let b1 = Block {
        id: "nick",
        blank_prefix: false,
        lines: vec![nick_line(s, wi, tick)],
    };
    let b2 = Block {
        id: "title",
        blank_prefix: false,
        lines: plain_lines(
            wrap_capped(&s.title, wi as usize, 2),
            Style::new().fg(palette::TEXT_DIM),
        ),
    };
    let b3_placeholder = Block {
        id: "status",
        blank_prefix: false,
        lines: vec![status_line(s, wi, true)],
    };
    let b4_placeholder = Block {
        id: "assistant",
        blank_prefix: true,
        lines: vec![Line::default()],
    };

    let blocks = vec![b1, b2, b3_placeholder, b4_placeholder];
    let (mut kept, elastic_raw) = fit_fixed_blocks(blocks, 1, &[2, 1], h);

    let (elastic_blank, elastic_budget) = elastic_budget_split(elastic_raw, true);
    let elastic_lines: Vec<String> =
        wrap_tail_multiline(&s.assistant_text, wi as usize, elastic_budget);
    if let Some(ab) = kept.iter_mut().find(|b| b.id == "assistant") {
        ab.blank_prefix = elastic_blank;
        ab.lines = plain_lines(elastic_lines, Style::new().fg(palette::TEXT_DIM));
    }

    let (lines_v, blocks_rendered) = assemble(kept);
    let mut lines = lines_v;
    lines.truncate(h);
    let blank_rows_left = h.saturating_sub(lines.len());
    TileContent {
        lines,
        blocks_rendered,
        blank_rows_left,
        regime: "extended",
    }
}

fn tiny(h: u16) -> TileContent {
    TileContent {
        lines: vec![],
        blocks_rendered: vec![],
        blank_rows_left: h as usize,
        regime: "tiny",
    }
}

fn narrow(s: &SessionView, wi: u16, h: u16, tick: usize) -> TileContent {
    let glyph = palette::state_glyph(s.state, tick);
    if h == 1 {
        let lines = vec![Line::from(Span::styled(
            glyph.to_string(),
            Style::new().fg(palette::tile_text(s.state)),
        ))];
        return TileContent {
            lines,
            blocks_rendered: vec!["glyph"],
            blank_rows_left: 0,
            regime: "narrow",
        };
    }
    let age = truncate_ellipsis(&s.age, wi as usize);
    let lines = vec![
        Line::from(Span::styled(
            glyph.to_string(),
            Style::new().fg(palette::tile_text(s.state)),
        )),
        dim(age),
    ];
    let blank_rows_left = (h as usize).saturating_sub(2);
    TileContent {
        lines,
        blocks_rendered: vec!["glyph", "age"],
        blank_rows_left,
        regime: "narrow",
    }
}

fn medium(s: &SessionView, wi: u16, h: u16, tick: usize) -> TileContent {
    let glyph = palette::state_glyph(s.state, tick);
    let glyph_age = truncate_ellipsis(&format!("{glyph} {}", s.age), wi as usize);
    if h == 1 {
        let lines = vec![Line::from(Span::styled(
            glyph_age,
            Style::new().fg(palette::tile_text(s.state)),
        ))];
        return TileContent {
            lines,
            blocks_rendered: vec!["glyph_age"],
            blank_rows_left: 0,
            regime: "medium",
        };
    }
    let nick = truncate_ellipsis(&s.nick, wi as usize);
    let lines = vec![
        Line::from(Span::styled(
            glyph_age,
            Style::new().fg(palette::tile_text(s.state)),
        )),
        Line::from(Span::styled(
            nick,
            Style::new().fg(palette::tile_text(s.state)),
        )),
    ];
    // h=3-4 and h>=5 show the same two rows — no `—` filler line drawn (matches the
    // regime table's `same as h=2` cell literally).
    let blank_rows_left = (h as usize).saturating_sub(2);
    TileContent {
        lines,
        blocks_rendered: vec!["glyph_age", "nick"],
        blank_rows_left,
        regime: "medium",
    }
}

fn wide_h1(s: &SessionView, wi: u16, tick: usize) -> TileContent {
    let glyph = palette::state_glyph(s.state, tick);
    let age = s.age.clone();
    let fixed_len = 1 + 1 + 3 + age.chars().count(); // glyph + ' ' + " · " + age
    let nick_budget = (wi as usize).saturating_sub(fixed_len);
    let nick = truncate_ellipsis(&s.nick, nick_budget);
    let text = format!("{glyph} {nick} · {age}");
    let lines = vec![Line::from(Span::styled(
        text,
        Style::new().fg(palette::tile_text(s.state)),
    ))];
    TileContent {
        lines,
        blocks_rendered: vec!["glyph_nick_age"],
        blank_rows_left: 0,
        regime: "wide-h1",
    }
}

fn wide_h2(s: &SessionView, wi: u16, tick: usize) -> TileContent {
    let lines = vec![nick_line(s, wi, tick), status_line(s, wi, false)];
    TileContent {
        lines,
        blocks_rendered: vec!["nick", "status"],
        blank_rows_left: 0,
        regime: "wide-h2",
    }
}

pub fn build_tile_content(s: &SessionView, wi: u16, h: u16, tick: usize) -> TileContent {
    if h == 0 || wi == 0 {
        return TileContent {
            lines: vec![],
            blocks_rendered: vec![],
            blank_rows_left: h as usize,
            regime: "degenerate",
        };
    }
    if wi < 3 {
        return tiny(h);
    }
    if wi <= 5 {
        return narrow(s, wi, h, tick);
    }
    if wi <= 11 {
        return medium(s, wi, h, tick);
    }
    // wi >= 12
    match h {
        1 => wide_h1(s, wi, tick),
        2 => wide_h2(s, wi, tick),
        3..=4 => compact(s, wi, h, tick),
        _ => match s.state {
            State::Running => extended_running(s, wi, h, tick),
            State::Question => extended_question(s, wi, h, tick),
            State::NeedsYou => extended_needs_you(s, wi, h, tick),
            State::Idle => extended_idle(s, wi, h, tick),
        },
    }
}
