pub mod common;
pub mod intro;
pub mod option_a;
pub mod option_b;
pub mod option_c;

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::fixture::{Fixture, Project, ProjectKind, Session};
use crate::layout::{flow_rows, BoxSpec};
use crate::palette;
use common::{CARD_GAP, PROJECT_GAP, ROW_GAP, CARD_SLOT_WIDTH};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChromeOption {
    A,
    B,
    C,
}

impl ChromeOption {
    pub fn label(self) -> &'static str {
        match self {
            ChromeOption::A => "Option A (border on both levels)",
            ChromeOption::B => "Option B (no borders)",
            ChromeOption::C => "Option C (project border only)",
        }
    }

    /// (horizontal overhead, vertical overhead, card height) for the project box chrome.
    fn overhead(self) -> (u16, u16, u16) {
        match self {
            ChromeOption::A => (2, 2, 5),
            ChromeOption::B => (0, 1, 3),
            ChromeOption::C => (2, 2, 3),
        }
    }
}

pub enum ContentItem<'a> {
    Card(&'a Session),
    Overflow(String),
}

pub struct ProjectLayout<'a> {
    pub rows: Vec<Vec<ContentItem<'a>>>,
    pub width: u16,
    pub height: u16,
    pub all_idle_chip: Option<String>,
}

fn compute_project_layout(project: &Project, card_height: u16, budget_width: u16) -> ProjectLayout<'_> {
    match &project.kind {
        ProjectKind::AllIdle { count } => {
            let text = format!("{}  {} idle", project.name, count);
            let w = text.chars().count() as u16;
            ProjectLayout {
                rows: vec![],
                width: w,
                height: 1,
                all_idle_chip: Some(text),
            }
        }
        ProjectKind::Cards { visible, idle_overflow } => {
            let mut specs: Vec<BoxSpec> = visible
                .iter()
                .map(|_| BoxSpec { width: CARD_SLOT_WIDTH, height: card_height })
                .collect();
            let overflow_text = if *idle_overflow > 0 {
                Some(format!("+{} idle", idle_overflow))
            } else {
                None
            };
            if let Some(t) = &overflow_text {
                let w = (t.chars().count() as u16 + 2).max(8);
                specs.push(BoxSpec { width: w, height: card_height });
            }

            let row_groups = flow_rows(budget_width.max(CARD_SLOT_WIDTH), CARD_GAP, &specs);
            let mut rows: Vec<Vec<ContentItem>> = Vec::new();
            let mut max_row_width = 0u16;
            for group in &row_groups {
                let mut row_items = Vec::new();
                let mut row_width = 0u16;
                for (pos, &idx) in group.iter().enumerate() {
                    if pos > 0 {
                        row_width += CARD_GAP;
                    }
                    row_width += specs[idx].width;
                    if idx < visible.len() {
                        row_items.push(ContentItem::Card(&visible[idx]));
                    } else {
                        row_items.push(ContentItem::Overflow(overflow_text.clone().unwrap()));
                    }
                }
                max_row_width = max_row_width.max(row_width);
                rows.push(row_items);
            }
            let height = rows.len() as u16 * card_height
                + rows.len().saturating_sub(1) as u16 * ROW_GAP;
            ProjectLayout {
                rows,
                width: max_row_width,
                height,
                all_idle_chip: None,
            }
        }
    }
}

pub fn render_dashboard(f: &mut Frame, area: Rect, option: ChromeOption, fixture: &Fixture) {
    let (h_overhead, v_overhead, card_height) = option.overhead();

    let layouts: Vec<ProjectLayout> = fixture
        .projects
        .iter()
        .map(|p| {
            let budget = area.width.saturating_sub(h_overhead);
            compute_project_layout(p, card_height, budget)
        })
        .collect();

    let specs: Vec<BoxSpec> = layouts
        .iter()
        .map(|l| BoxSpec {
            width: l.width + h_overhead,
            height: l.height + v_overhead,
        })
        .collect();

    let outer_rows = flow_rows(area.width, PROJECT_GAP, &specs);

    let mut y = area.y;
    for row in outer_rows {
        let row_height = row.iter().map(|&i| specs[i].height).max().unwrap_or(0);
        if y + row_height > area.y + area.height {
            break;
        }
        let mut x = area.x;
        for &idx in &row {
            let w = specs[idx].width.min(area.x + area.width - x);
            let rect = Rect { x, y, width: w, height: row_height };
            let project = &fixture.projects[idx];
            let layout = &layouts[idx];
            let color = palette::project_color(project.color_idx);
            match option {
                ChromeOption::A => option_a::draw_project(f, rect, project, layout, color),
                ChromeOption::B => option_b::draw_project(f, rect, project, layout, color),
                ChromeOption::C => option_c::draw_project(f, rect, project, layout, color),
            }
            x += specs[idx].width + PROJECT_GAP;
        }
        y += row_height + ROW_GAP;
    }
}
