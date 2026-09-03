// Hardcoded fixture data. No live data, no network calls — this is a chrome-comparison
// prototype only. Some titles/elapsed values were not specified in the brief for every
// session (e.g. running sessions' elapsed, a few needs-you-plain sessions' titles) and
// were invented here to satisfy the "nickname · title" / "status · elapsed" line formats.

#[derive(Clone)]
pub enum Status {
    Running {
        elapsed: &'static str,
        action: &'static str,
    },
    NeedsYouQuestion {
        last_line: Option<&'static str>,
    },
    NeedsYouPlain {
        elapsed: &'static str,
    },
}

#[derive(Clone)]
pub struct Session {
    pub nickname: &'static str,
    pub title: &'static str,
    pub status: Status,
    pub subagent: bool,
}

impl Session {
    fn new(nickname: &'static str, title: &'static str, status: Status) -> Self {
        Session {
            nickname,
            title,
            status,
            subagent: false,
        }
    }

    fn subagent(nickname: &'static str, title: &'static str, status: Status) -> Self {
        Session {
            nickname,
            title,
            status,
            subagent: true,
        }
    }
}

pub enum ProjectKind {
    Cards {
        visible: Vec<Session>,
        idle_overflow: usize,
    },
    AllIdle {
        count: usize,
    },
}

pub struct Project {
    pub name: &'static str,
    pub color_idx: usize,
    pub kind: ProjectKind,
}

pub struct Fixture {
    pub label: &'static str,
    pub projects: Vec<Project>,
}

pub fn sparse() -> Fixture {
    Fixture {
        label: "sparse fixture",
        projects: vec![
            Project {
                name: "opencode-mcp",
                color_idx: 0,
                kind: ProjectKind::Cards {
                    visible: vec![
                        Session::new(
                            "amber-falcon",
                            "Reviewing opencode dashboard requirements",
                            Status::Running {
                                elapsed: "3m ago",
                                action: "editing: requirements.md",
                            },
                        ),
                        Session::new(
                            "brave-otter",
                            "Add multiple code titles with edit support",
                            Status::NeedsYouQuestion {
                                last_line: Some("Which file would you like me to delete?"),
                            },
                        ),
                    ],
                    idle_overflow: 0,
                },
            },
            Project {
                name: "infra",
                color_idx: 1,
                kind: ProjectKind::Cards {
                    visible: vec![Session::new(
                        "cobalt-wren",
                        "Apply terraform plan for prod-db",
                        Status::NeedsYouPlain { elapsed: "14m" },
                    )],
                    idle_overflow: 0,
                },
            },
            Project {
                name: "web",
                color_idx: 2,
                kind: ProjectKind::Cards {
                    visible: vec![Session::new(
                        "dusty-lynx",
                        "Fix TUI hygiene and Enter stub",
                        Status::Running {
                            elapsed: "1m ago",
                            action: "running: pnpm build",
                        },
                    )],
                    idle_overflow: 0,
                },
            },
        ],
    }
}

pub fn busy() -> Fixture {
    Fixture {
        label: "busy fixture",
        projects: vec![
            Project {
                name: "opencode-mcp",
                color_idx: 0,
                kind: ProjectKind::Cards {
                    visible: vec![
                        Session::new(
                            "amber-falcon",
                            "Add Multiple Code Titles with Edit Support and Some Extra Detail That Truncates",
                            Status::NeedsYouQuestion { last_line: None },
                        ),
                        Session::new(
                            "brave-otter",
                            "Refactor hierarchy component tests",
                            Status::Running {
                                elapsed: "9m ago",
                                action: "shell: cd apps/web/src/components/organization/hierarchy && pnpm typecheck",
                            },
                        ),
                        Session::new(
                            "cobalt-wren",
                            "Resolve dashboard branch merge conflict",
                            Status::NeedsYouPlain { elapsed: "22m" },
                        ),
                    ],
                    idle_overflow: 2,
                },
            },
            Project {
                name: "infra",
                color_idx: 1,
                kind: ProjectKind::Cards {
                    visible: vec![
                        Session::new(
                            "dusty-lynx",
                            "Apply terraform destroy for staging — are you sure?",
                            Status::NeedsYouQuestion { last_line: None },
                        ),
                        Session::new(
                            "silver-marlin",
                            "Plan terraform infra changes",
                            Status::Running {
                                elapsed: "1m ago",
                                action: "running: terraform plan",
                            },
                        ),
                        Session::new(
                            "ember-quail",
                            "Rotate staging API keys",
                            Status::NeedsYouPlain { elapsed: "4m" },
                        ),
                    ],
                    idle_overflow: 1,
                },
            },
            Project {
                name: "web",
                color_idx: 2,
                kind: ProjectKind::Cards {
                    visible: vec![
                        Session::new(
                            "golden-hawk",
                            "Ship the chrome prototype",
                            Status::Running {
                                elapsed: "4m ago",
                                action: "writing: main.rs",
                            },
                        ),
                        Session::subagent(
                            "violet-lynx",
                            "Delegated: render option C",
                            Status::Running {
                                elapsed: "2m ago",
                                action: "editing: render.rs",
                            },
                        ),
                        Session::new(
                            "hawk-otter",
                            "Fix flaky e2e test",
                            Status::NeedsYouPlain { elapsed: "6m" },
                        ),
                    ],
                    idle_overflow: 0,
                },
            },
            Project {
                name: "scratch-tool",
                color_idx: 3,
                kind: ProjectKind::AllIdle { count: 3 },
            },
            Project {
                name: "mobile-app",
                color_idx: 4,
                kind: ProjectKind::Cards {
                    visible: vec![
                        Session::new(
                            "copper-hawk",
                            "Fix push notification token refresh",
                            Status::Running {
                                elapsed: "6m ago",
                                action: "editing: NotificationManager.swift",
                            },
                        ),
                        Session::new(
                            "quartz-badger",
                            "Update app icon assets",
                            Status::NeedsYouPlain { elapsed: "1m" },
                        ),
                    ],
                    idle_overflow: 0,
                },
            },
            Project {
                name: "api",
                color_idx: 5,
                kind: ProjectKind::Cards {
                    visible: vec![Session::new(
                        "violet-otter",
                        "Add rate limit middleware",
                        Status::Running {
                            elapsed: "45s ago",
                            action: "shell: cargo test rate_limit",
                        },
                    )],
                    idle_overflow: 0,
                },
            },
        ],
    }
}
