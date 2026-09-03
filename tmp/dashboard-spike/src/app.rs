use std::time::{Duration, Instant};

use crate::data::{self, Project, Session, Status};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VisibleSession {
    pub session: Session,
    pub is_idle: bool,
    pub is_overflow: bool,
    pub overflow_count: usize,
    pub overflow_label: String,
}

#[derive(Debug, Clone)]
pub struct VisibleProject {
    pub project: Project,
    pub visible_sessions: Vec<VisibleSession>,
    pub active_count: usize,
    pub idle_count: usize,
    pub weight: f64,
}

#[allow(dead_code)]
pub struct App {
    pub projects: Vec<Project>,
    pub window: Duration,
    pub show_all: bool,
    pub selected: usize,
    pub total_items: usize,
    pub now: Instant,
    pub status: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let now = Instant::now();
        let projects = data::build_dummy_projects(now);
        let mut app = App {
            projects,
            window: Duration::from_secs(600), // 10 minutes
            show_all: false,
            selected: 0,
            total_items: 0,
            now,
            status: None,
        };
        app.recompute();
        app
    }

    pub fn adjust_window(&mut self, delta: i64) {
        let secs = self.window.as_secs() as i64 + delta;
        let clamped = secs.clamp(60, 3600);
        self.window = Duration::from_secs(clamped as u64);
        self.recompute();
    }

    pub fn reset_window(&mut self) {
        self.window = Duration::from_secs(600);
        self.recompute();
    }

    pub fn toggle_show_all(&mut self) {
        self.show_all = !self.show_all;
        if !self.show_all && self.window.as_secs() > 3600 {
            self.window = Duration::from_secs(3600);
        }
        self.recompute();
    }

    pub fn select_next(&mut self) {
        if self.total_items > 0 {
            self.selected = (self.selected + 1) % self.total_items;
        }
    }

    pub fn select_prev(&mut self) {
        if self.total_items > 0 {
            self.selected = if self.selected == 0 { self.total_items - 1 } else { self.selected - 1 };
        }
    }

    /// Returns the display label for the currently selected tile.
    pub fn selected_label(&self) -> String {
        let visible_projects = self.get_visible_projects();
        let mut idx = 0;
        for vp in &visible_projects {
            for vs in &vp.visible_sessions {
                if idx == self.selected {
                    if vs.is_overflow {
                        return vs.overflow_label.clone();
                    } else {
                        return vs.session.id.to_string();
                    }
                }
                idx += 1;
            }
        }
        "(none)".to_string()
    }

    pub fn recompute(&mut self) {
        let mut visible_projects = Vec::new();

        for project in &self.projects {
            let mut active_sessions = Vec::new();
            let mut idle_sessions = Vec::new();

            for session in &project.sessions {
                let active = data::is_active(session.ago, self.window);
                if active {
                    active_sessions.push((session, false));
                } else {
                    idle_sessions.push((session, true));
                }
            }

            // Check if project should be shown
            let has_active = !active_sessions.is_empty();
            if !has_active && !self.show_all {
                continue;
            }

            // Order: stalled first, then most recent active, then most recent idle
            active_sessions.sort_by(|a, b| {
                let a_stalled = a.0.status == Status::Stalled;
                let b_stalled = b.0.status == Status::Stalled;
                match (a_stalled, b_stalled) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.0.ago.cmp(&a.0.ago), // most recent first (smaller ago = more recent)
                }
            });

            idle_sessions.sort_by(|a, b| b.0.ago.cmp(&a.0.ago));

            let all_sorted: Vec<(&Session, bool)> = active_sessions
                .into_iter()
                .chain(idle_sessions.into_iter())
                .collect();

            let max_visible = 3;
            let visible: Vec<VisibleSession> = all_sorted
                .iter()
                .take(max_visible)
                .map(|(s, is_idle)| VisibleSession {
                    session: (*s).clone(),
                    is_idle: *is_idle,
                    is_overflow: false,
                    overflow_count: 0,
                    overflow_label: String::new(),
                })
                .collect();

            let overflow_count = all_sorted.len().saturating_sub(max_visible);
            let mut vis = visible;

            if overflow_count > 0 {
                let all_idle_overflow = all_sorted[max_visible..].iter().all(|(_, idle)| *idle);
                let label = if all_idle_overflow {
                    format!("+{overflow_count} idle")
                } else {
                    format!("+{overflow_count} sessions")
                };
                vis.push(VisibleSession {
                    session: Session {
                        id: "overflow",
                        title: "",
                        status: Status::Waiting,
                        ago: Duration::from_secs(0),
                        parent: None,
                    },
                    is_idle: all_idle_overflow,
                    is_overflow: true,
                    overflow_count,
                    overflow_label: label,
                });
            }

            let active_count = vis.iter().filter(|s| !s.is_idle && !s.is_overflow).count();
            let idle_count = vis.iter().filter(|s| s.is_idle && !s.is_overflow).count();
            let weight: f64 = vis
                .iter()
                .filter(|s| !s.is_overflow)
                .map(|s| if s.is_idle { 1.0 } else { 3.0 })
                .sum();

            visible_projects.push(VisibleProject {
                project: project.clone(),
                visible_sessions: vis,
                active_count,
                idle_count,
                weight,
            });
        }

        // Sort by weight descending
        visible_projects.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());

        self.total_items = visible_projects.iter().map(|p| p.visible_sessions.len()).sum();
        if self.selected >= self.total_items && self.total_items > 0 {
            self.selected = self.total_items - 1;
        }
    }

    pub fn get_visible_projects(&self) -> Vec<VisibleProject> {
        let mut visible_projects = Vec::new();

        for project in &self.projects {
            let mut active_sessions = Vec::new();
            let mut idle_sessions = Vec::new();

            for session in &project.sessions {
                let active = data::is_active(session.ago, self.window);
                if active {
                    active_sessions.push((session, false));
                } else {
                    idle_sessions.push((session, true));
                }
            }

            let has_active = !active_sessions.is_empty();
            if !has_active && !self.show_all {
                continue;
            }

            active_sessions.sort_by(|a, b| {
                let a_stalled = a.0.status == Status::Stalled;
                let b_stalled = b.0.status == Status::Stalled;
                match (a_stalled, b_stalled) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.0.ago.cmp(&a.0.ago),
                }
            });

            idle_sessions.sort_by(|a, b| b.0.ago.cmp(&a.0.ago));

            let all_sorted: Vec<(&Session, bool)> = active_sessions
                .into_iter()
                .chain(idle_sessions.into_iter())
                .collect();

            let max_visible = 3;
            let visible: Vec<VisibleSession> = all_sorted
                .iter()
                .take(max_visible)
                .map(|(s, is_idle)| VisibleSession {
                    session: (*s).clone(),
                    is_idle: *is_idle,
                    is_overflow: false,
                    overflow_count: 0,
                    overflow_label: String::new(),
                })
                .collect();

            let overflow_count = all_sorted.len().saturating_sub(max_visible);
            let mut vis = visible;

            if overflow_count > 0 {
                let all_idle_overflow = all_sorted[max_visible..].iter().all(|(_, idle)| *idle);
                let label = if all_idle_overflow {
                    format!("+{overflow_count} idle")
                } else {
                    format!("+{overflow_count} sessions")
                };
                vis.push(VisibleSession {
                    session: Session {
                        id: "overflow",
                        title: "",
                        status: Status::Waiting,
                        ago: Duration::from_secs(0),
                        parent: None,
                    },
                    is_idle: all_idle_overflow,
                    is_overflow: true,
                    overflow_count,
                    overflow_label: label,
                });
            }

            let active_count = vis.iter().filter(|s| !s.is_idle && !s.is_overflow).count();
            let idle_count = vis.iter().filter(|s| s.is_idle && !s.is_overflow).count();
            let weight: f64 = vis
                .iter()
                .filter(|s| !s.is_overflow)
                .map(|s| if s.is_idle { 1.0 } else { 3.0 })
                .sum();

            visible_projects.push(VisibleProject {
                project: project.clone(),
                visible_sessions: vis,
                active_count,
                idle_count,
                weight,
            });
        }

        visible_projects.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
        visible_projects
    }
}
