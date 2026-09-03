use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Doing,
    Thinking,
    Waiting,
    Stalled,
}

#[allow(dead_code)]
impl Status {
    pub fn glyph(&self) -> &'static str {
        match self {
            Status::Doing => "●",
            Status::Thinking => "◐",
            Status::Waiting => "◐",
            Status::Stalled => "■",
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Session {
    pub id: &'static str,
    pub title: &'static str,
    pub status: Status,
    pub ago: Duration,
    pub parent: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: &'static str,
    pub name: &'static str,
    pub sessions: Vec<Session>,
}

pub fn is_active(ago: Duration, window: Duration) -> bool {
    ago <= window
}

pub fn build_dummy_projects(now: Instant) -> Vec<Project> {
    vec![
        Project {
            id: "proj_mcp",
            name: "opencode-mcp",
            sessions: vec![
                Session { id: "ses_stall", title: "hung bash", status: Status::Stalled, ago: now.elapsed() + Duration::from_secs(90), parent: None },
                Session { id: "ses_child", title: "explore", status: Status::Thinking, ago: now.elapsed() + Duration::from_secs(120), parent: Some("ses_stall") },
                Session { id: "ses_edit", title: "editing spec", status: Status::Doing, ago: now.elapsed() + Duration::from_secs(30), parent: None },
                Session { id: "ses_old", title: "idle review", status: Status::Waiting, ago: now.elapsed() + Duration::from_secs(2400), parent: None },
            ],
        },
        Project {
            id: "proj_web",
            name: "web",
            sessions: vec![
                Session { id: "ses_web", title: "page spike", status: Status::Thinking, ago: now.elapsed() + Duration::from_secs(300), parent: None },
            ],
        },
        Project {
            id: "proj_infra",
            name: "infra",
            sessions: vec![
                Session { id: "ses_i1", title: "tf plan", status: Status::Doing, ago: now.elapsed() + Duration::from_secs(60), parent: None },
                Session { id: "ses_i2", title: "wait apply", status: Status::Waiting, ago: now.elapsed() + Duration::from_secs(480), parent: None },
                Session { id: "ses_i3", title: "stale logs", status: Status::Waiting, ago: now.elapsed() + Duration::from_secs(1500), parent: None },
                Session { id: "ses_i4", title: "hung ssh", status: Status::Stalled, ago: now.elapsed() + Duration::from_secs(180), parent: None },
                Session { id: "ses_i5", title: "old nightly", status: Status::Waiting, ago: now.elapsed() + Duration::from_secs(3000), parent: None },
            ],
        },
        Project {
            id: "proj_idle",
            name: "idle-only",
            sessions: vec![
                Session { id: "ses_z", title: "ancient", status: Status::Waiting, ago: now.elapsed() + Duration::from_secs(10800), parent: None },
            ],
        },
    ]
}
