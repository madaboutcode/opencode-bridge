// Mosaic spike fixture data — REAL (BRIEF-v2.md fixture section) and STRESS
// (redesign-specs.md §0.5.2), plus the runtime mutation ops the brief's keybindings need.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Question,
    NeedsYou,
    Running,
    Idle,
}

#[derive(Clone, Debug)]
pub struct Subagent {
    pub nick: String,
    pub action: String,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub nick: String,
    pub title: String,
    pub state: State,
    pub age: String,
    /// needs-you / question sort key (longest wait first); minutes.
    pub wait_m: Option<u32>,
    /// running only: current action line (R6.5 format).
    pub action: Option<String>,
    /// running only (B3): tool-call pairs this turn.
    pub calls: Option<u32>,
    pub subs: Vec<Subagent>,
    /// oldest -> newest, excludes the current action.
    pub recent: Vec<String>,
    pub files: Vec<String>,
    /// multi-line, newline-separated.
    pub assistant_text: String,
    pub user_prompt: String,
}

impl Session {
    fn blank(nick: &str, title: &str, state: State, age: &str) -> Self {
        Session {
            nick: nick.into(),
            title: title.into(),
            state,
            age: age.into(),
            wait_m: None,
            action: None,
            calls: None,
            subs: vec![],
            recent: vec![],
            files: vec![],
            assistant_text: String::new(),
            user_prompt: String::new(),
        }
    }

    fn wait(mut self, m: u32) -> Self {
        self.wait_m = Some(m);
        self
    }
    fn action(mut self, a: &str) -> Self {
        self.action = Some(a.into());
        self
    }
    fn calls(mut self, c: u32) -> Self {
        self.calls = Some(c);
        self
    }
    fn subs(mut self, s: Vec<(&str, &str)>) -> Self {
        self.subs = s
            .into_iter()
            .map(|(nick, action)| Subagent { nick: nick.into(), action: action.into() })
            .collect();
        self
    }
    fn recent(mut self, r: Vec<&str>) -> Self {
        self.recent = r.into_iter().map(String::from).collect();
        self
    }
    fn files(mut self, f: Vec<&str>) -> Self {
        self.files = f.into_iter().map(String::from).collect();
        self
    }
    fn assistant(mut self, t: &str) -> Self {
        self.assistant_text = t.into();
        self
    }
    fn user_prompt(mut self, t: &str) -> Self {
        self.user_prompt = t.into();
        self
    }

    /// Parses fixture age strings (`9m`, `45s`, `2h`) into seconds, for idle-chip
    /// most-recent-first ordering. The fixture schema has no numeric recency field, so
    /// this is a spike-only convenience parse, not a production concern.
    pub fn age_secs(&self) -> u64 {
        let s = self.age.trim();
        let (num, unit) = s.split_at(s.len().saturating_sub(1));
        let n: u64 = num.parse().unwrap_or(0);
        match unit {
            "s" => n,
            "m" => n * 60,
            "h" => n * 3600,
            _ => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Project {
    pub name: String,
    pub sessions: Vec<Session>,
}

impl Project {
    pub fn is_all_idle(&self) -> bool {
        !self.sessions.is_empty() && self.sessions.iter().all(|s| s.state == State::Idle)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Fixture {
    pub projects: Vec<Project>,
    /// counter for synthetic sessions appended via `+`.
    pub synthetic_n: u32,
    /// counter for `.` tick-appended recent actions (cycles a 3-entry pattern).
    pub tick_n: u32,
}

const SYNTH_RECENT_CYCLE: [&str; 3] = ["read: a.rs", "shell: cargo test", "editing: b.rs"];
const TICK_CYCLE: [&str; 3] = ["read: a.rs", "shell: cargo check", "editing: b.rs"];

impl Fixture {
    /// `+`: append a synthetic running session to the last project.
    pub fn add_session(&mut self) {
        let Some(p) = self.projects.last_mut() else { return };
        self.synthetic_n += 1;
        let n = self.synthetic_n;
        let recent: Vec<&str> = (0..3).map(|i| SYNTH_RECENT_CYCLE[(i as usize) % 3]).collect();
        let s = Session::blank(&format!("new-{n}"), "Synthetic session", State::Running, "0s")
            .action("shell: sleep 1")
            .calls(1)
            .recent(recent);
        p.sessions.push(s);
    }

    /// `-`: remove the last session of the last project, never below 1.
    pub fn remove_session(&mut self) {
        let Some(p) = self.projects.last_mut() else { return };
        if p.sessions.len() > 1 {
            p.sessions.pop();
        }
    }

    /// `p`: append a new project `late-arrival` with one running session.
    pub fn add_project(&mut self) {
        let s = Session::blank("fresh-wren", "Late arrival", State::Running, "0s")
            .action("shell: echo hello")
            .calls(1)
            .recent(vec!["shell: echo hello"]);
        self.projects.push(Project { name: "late-arrival".into(), sessions: vec![s] });
    }

    /// `.`: append one cycling action to every running session's recent list.
    pub fn tick_recent(&mut self) {
        let entry = TICK_CYCLE[(self.tick_n as usize) % 3];
        self.tick_n += 1;
        for p in &mut self.projects {
            for s in &mut p.sessions {
                if s.state == State::Running {
                    s.recent.push(entry.to_string());
                }
            }
        }
    }
}

/// REAL fixture — 4 projects, 8 sessions, 3/2/2/1. BRIEF-v2.md "Fixture" section.
pub fn build_real() -> Fixture {
    use State::*;

    let web_dashboard = Project {
        name: "web-dashboard".into(),
        sessions: vec![
            Session::blank(
                "brave-otter",
                "Add multiple code titles with edit support",
                Question,
                "9m",
            )
            .wait(9)
            .user_prompt("Clean up the old title components and remove whatever's unused")
            .assistant(
                "I found three candidates for removal under src/components/titles/:\n\n\
                 1. LegacyTitle.tsx — no imports anywhere\n\
                 2. TitleEditor.old.tsx — imported only by the storybook story\n\
                 3. title-utils.ts — 2 helpers still used by TitleBar.tsx\n\n\
                 Deleting 3 would break TitleBar. Which file would you like me to delete?",
            )
            .recent(vec![
                "grep: LegacyTitle",
                "read: src/components/titles/LegacyTitle.tsx",
                "grep: TitleEditor",
                "read: src/components/titles/TitleEditor.old.tsx",
                "grep: title-utils",
                "read: src/components/TitleBar.tsx",
            ]),
            Session::blank(
                "amber-falcon",
                "Reviewing opencode dashboard requirements doc for session card layout",
                Running,
                "3m",
            )
            .action("editing: requirements.md")
            .calls(11)
            .subs(vec![("cinder-wisp", "editing: render.rs")])
            .files(vec!["requirements.md", "render.rs", "layout-brainstorm.md"])
            .user_prompt("Fold the brainstorm learnings back into the requirements doc")
            .recent(vec![
                "read: tasks/2026-09-01-opencode-dashboard.requirements.md",
                "grep: R5.2",
                "read: src/render.rs",
                "shell: cargo build",
                "editing: render.rs",
                "shell: cargo test -p dashboard",
                "read: tasks/2026-09-01-opencode-dashboard.layout-brainstorm.md",
                "editing: layout-brainstorm.md",
                "shell: git diff --stat",
                "read: BRIEF.md",
            ]),
            Session::blank(
                "extraordinarily-verbose-nickname-case",
                "Investigate why the CI pipeline intermittently fails on the integration \
                 test suite for the payments service when run under load on Tuesdays",
                Running,
                "11m",
            )
            .action("running: cargo test --package payments-integration")
            .calls(15)
            .files(vec!["payments/tests/integration.rs", "ci/pipeline.yml"])
            .user_prompt("The payments integration suite flakes under load on Tuesdays. Find out why.")
            .recent(vec![
                "read: ci/pipeline.yml",
                "grep: test-threads",
                "shell: gh run list --workflow ci.yml --limit 50 --json conclusion,createdAt",
                "read: payments/tests/integration.rs",
                "shell: cargo test --package payments-integration -- --test-threads=1 integration::tuesday_load_profile",
                "grep: sleep_until",
                "read: payments/src/clock.rs",
                "editing: payments/src/clock.rs",
                "shell: cargo test --package payments-integration -- --test-threads=8",
                "read: ci/cache.yml",
                "shell: cargo test --package payments-integration -- --test-threads=8 --nocapture 2>&1 | tail -80",
                "grep: RUST_TEST_THREADS",
                "editing: ci/pipeline.yml",
                "shell: cargo build --release",
            ]),
        ],
    };

    let infra_tools = Project {
        name: "infra-tools".into(),
        sessions: vec![
            Session::blank(
                "golden-hawk",
                "Apply terraform destroy for staging, are you sure?",
                Question,
                "6m",
            )
            .wait(6)
            .user_prompt("tear down staging, we're rebuilding it from the new module")
            .assistant(
                "terraform plan -destroy reports 12 resources in staging:\n\n  \
                 aws_rds_instance.staging        (db.t3.medium, 40 GB)\n  \
                 aws_ecs_service.api             ×3\n  \
                 aws_lb.staging\n  \
                 aws_s3_bucket.staging-assets    (2.1 GB, versioning off)\n  \
                 aws_iam_role.staging-task       ×5\n\n\
                 The S3 bucket is not backed up. Confirm destroy of 12 resources in staging?",
            )
            .recent(vec![
                "shell: terraform init",
                "shell: terraform workspace select staging",
                "shell: terraform plan -destroy -out=destroy.tfplan",
                "read: destroy.tfplan",
            ]),
            Session::blank("hawk-otter", "Rewrite deploy script", Idle, "51m")
                .user_prompt("rewrite the deploy script in python with a dry run flag")
                .assistant(
                    "Rewrote deploy.sh as deploy/run.py with --dry-run and per-service targets. \
                     All 6 services deploy cleanly against the staging cluster.",
                ),
        ],
    };

    let mobile_app = Project {
        name: "mobile-app".into(),
        sessions: vec![
            Session::blank(
                "sable-heron",
                "Fix push notification token refresh",
                Running,
                "6m",
            )
            .action("editing: NotificationManager.swift")
            .calls(8)
            .subs(vec![
                ("pebble-owl", "editing: TokenStore.swift"),
                ("misty-vole", "running: xcodebuild test"),
                ("ashen-crane", "reviewing: PushDelegate.swift"),
            ])
            .files(vec![
                "NotificationManager.swift",
                "TokenStore.swift",
                "PushDelegate.swift",
                "AppDelegate.swift",
            ])
            .user_prompt("Push tokens go stale after the app is backgrounded for a day; fix the refresh path")
            .recent(vec![
                "grep: didRegisterForRemoteNotifications",
                "read: AppDelegate.swift",
                "read: NotificationManager.swift",
                "editing: AppDelegate.swift",
                "shell: xcodebuild -scheme App test -only-testing:PushTests",
                "read: TokenStore.swift",
                "subagent: 3 started",
            ]),
            Session::blank("quartz-badger", "Update app icon assets", NeedsYou, "1m")
                .wait(1)
                .user_prompt("update the app icon from the new 1024 png in Design/")
                .assistant(
                    "Regenerated all 18 icon sizes from Design/Icon-1024.png and updated \
                     Assets.xcassets/AppIcon.appiconset/Contents.json. Removed the two \
                     deprecated iPad Pro entries. Xcode build succeeds with no warnings.\n\nDone.",
                ),
        ],
    };

    let scratch_cli = Project {
        name: "scratch-cli".into(),
        sessions: vec![Session::blank(
            "violet-otter",
            "Prototype a faster arg parser",
            Running,
            "45s",
        )
        .action("shell: cargo bench")
        .calls(6)
        .files(vec!["src/args.rs", "benches/parse.rs"])
        .user_prompt("try a hand-rolled parser and bench it against clap")
        .recent(vec![
            "read: src/main.rs",
            "write: src/args.rs",
            "write: benches/parse.rs",
            "shell: cargo build --release",
            "shell: cargo bench -- --warm-up-time 1",
        ])],
    };

    Fixture {
        projects: vec![web_dashboard, infra_tools, mobile_app, scratch_cli],
        synthetic_n: 0,
        tick_n: 0,
    }
}

/// STRESS fixture — redesign-specs.md §0.5.2, 8 projects, 54 sessions. New fields
/// (recent/assistant_text/user_prompt/files) filled generically per BRIEF-v2's
/// instruction since §0.5.2 predates them.
pub fn build_stress() -> Fixture {
    use State::*;

    fn generic_recent() -> Vec<&'static str> {
        vec![
            "read: x.rs",
            "shell: cargo test",
            "editing: y.rs",
            "read: x.rs",
            "shell: cargo test",
            "editing: y.rs",
        ]
    }
    fn generic_assistant(title: &str) -> String {
        format!("{title} — done.")
    }

    let web_dashboard = Project {
        name: "web-dashboard".into(),
        sessions: vec![
            Session::blank(
                "amber-falcon",
                "Reviewing opencode dashboard requirements doc for session card layout",
                Running,
                "3m",
            )
            .action("editing: requirements.md")
            .calls(7)
            .subs(vec![("cinder-wisp", "editing: render.rs")])
            .recent(generic_recent())
            .assistant(&generic_assistant("Reviewing opencode dashboard requirements doc for session card layout")),
            Session::blank(
                "brave-otter",
                "Add multiple code titles with edit support",
                Question,
                "9m",
            )
            .wait(9)
            .assistant("Which file would you like me to delete?")
            .recent(generic_recent()),
            Session::blank(
                "extraordinarily-verbose-nickname-case",
                "Investigate why the CI pipeline intermittently fails on the integration \
                 test suite for the payments service when run under load on Tuesdays",
                Running,
                "11m",
            )
            .action("running: cargo test --package payments-integration")
            .calls(7)
            .recent(generic_recent())
            .assistant(&generic_assistant("Investigate why the CI pipeline intermittently fails on the integration test suite for the payments service when run under load on Tuesdays")),
            Session::blank("cobalt-wren", "Resolve dashboard branch merge conflict", NeedsYou, "22m")
                .wait(22)
                .assistant(&generic_assistant("Resolve dashboard branch merge conflict")),
            Session::blank("dusty-lynx", "Update footer legend copy", Idle, "12m")
                .assistant(&generic_assistant("Update footer legend copy")),
        ],
    };

    let infra_tools = Project {
        name: "infra-tools".into(),
        sessions: vec![
            Session::blank("silver-marlin", "Plan terraform infra changes", Running, "1m")
                .action("running: terraform plan")
                .calls(7)
                .recent(generic_recent())
                .assistant(&generic_assistant("Plan terraform infra changes")),
            Session::blank("ember-quail", "Rotate staging API keys", NeedsYou, "4m")
                .wait(4)
                .assistant(&generic_assistant("Rotate staging API keys")),
            Session::blank(
                "golden-hawk",
                "Apply terraform destroy for staging, are you sure?",
                Question,
                "6m",
            )
            .wait(6)
            .assistant("Confirm destroy of 12 resources in staging?")
            .recent(generic_recent()),
            Session::blank("hawk-otter", "Rewrite deploy script", Idle, "51m")
                .assistant(&generic_assistant("Rewrite deploy script")),
        ],
    };

    let scratch_cli = Project {
        name: "scratch-cli".into(),
        sessions: vec![Session::blank(
            "violet-otter",
            "Prototype a faster arg parser",
            Running,
            "45s",
        )
        .action("shell: cargo bench")
        .calls(7)
        .recent(generic_recent())
        .assistant(&generic_assistant("Prototype a faster arg parser"))],
    };

    let docs_site = Project {
        name: "docs-site".into(),
        sessions: vec![
            Session::blank("quartz-badger", "Fix broken anchor links", Idle, "2h")
                .assistant(&generic_assistant("Fix broken anchor links")),
            Session::blank("copper-hawk", "Update install instructions", Idle, "40m")
                .assistant(&generic_assistant("Update install instructions")),
            Session::blank("marble-finch", "Regenerate API reference", Idle, "3h")
                .assistant(&generic_assistant("Regenerate API reference")),
            Session::blank("linen-otter", "Proofread changelog", Idle, "18m")
                .assistant(&generic_assistant("Proofread changelog")),
        ],
    };

    let mobile_app = Project {
        name: "mobile-app".into(),
        sessions: vec![
            Session::blank(
                "sable-heron",
                "Fix push notification token refresh",
                Running,
                "6m",
            )
            .action("editing: NotificationManager.swift")
            .calls(7)
            .subs(vec![
                ("pebble-owl", "editing: TokenStore.swift"),
                ("misty-vole", "running: xcodebuild test"),
                ("ashen-crane", "reviewing: PushDelegate.swift"),
            ])
            .recent(generic_recent())
            .assistant(&generic_assistant("Fix push notification token refresh")),
            Session::blank("quartz-badger", "Update app icon assets", NeedsYou, "1m")
                .wait(1)
                .assistant(&generic_assistant("Update app icon assets")),
            Session::blank("willow-stag", "Bump minimum iOS deployment target", Question, "3m")
                .wait(3)
                .assistant("OK to drop iOS 15 support?")
                .recent(generic_recent()),
            Session::blank("flint-osprey", "Wire up crash reporting SDK", Running, "2m")
                .action("running: pod install")
                .calls(7)
                .recent(generic_recent())
                .assistant(&generic_assistant("Wire up crash reporting SDK")),
            Session::blank("brass-heron", "Localize onboarding screens", Idle, "35m")
                .assistant(&generic_assistant("Localize onboarding screens")),
            Session::blank("cider-fox", "Audit accessibility labels", Idle, "1h")
                .assistant(&generic_assistant("Audit accessibility labels")),
        ],
    };

    let big_monorepo = Project {
        name: "big-monorepo".into(),
        sessions: vec![
            Session::blank("north-tiger", "Build the monorepo", Running, "1m")
                .action("running: bazel build //...")
                .calls(7)
                .recent(generic_recent())
                .assistant(&generic_assistant("Build the monorepo")),
            Session::blank("east-lynx", "Run e2e suite", Running, "2m")
                .action("running: pnpm test:e2e")
                .calls(7)
                .recent(generic_recent())
                .assistant(&generic_assistant("Run e2e suite")),
            Session::blank("south-heron", "Tidy eslint config", Running, "4m")
                .action("editing: .eslintrc.js")
                .calls(7)
                .recent(generic_recent())
                .assistant(&generic_assistant("Tidy eslint config")),
            Session::blank("west-badger", "Typecheck the monorepo", Running, "5m")
                .action("running: tsc --noEmit")
                .calls(7)
                .recent(generic_recent())
                .assistant(&generic_assistant("Typecheck the monorepo")),
            Session::blank("cinder-wolf", "Clarify billing service ownership", Question, "7m")
                .wait(7)
                .assistant("Which service owns the billing table?")
                .recent(generic_recent()),
            Session::blank("moss-otter", "Fix CI cache key", NeedsYou, "9m")
                .wait(9)
                .subs(vec![("quiet-lark", "running: cache-key-diff.sh")])
                .assistant(&generic_assistant("Fix CI cache key")),
            Session::blank("plum-heron", "Test webhooks", Running, "3m")
                .action("editing: webhooks.test.ts")
                .calls(7)
                .recent(generic_recent())
                .assistant(&generic_assistant("Test webhooks")),
            Session::blank("amber-stag", "Clean up dead feature flags", NeedsYou, "16m")
                .wait(16)
                .assistant(&generic_assistant("Clean up dead feature flags")),
            Session::blank("teal-mole", "Watch pod status", Running, "8m")
                .action("shell: kubectl get pods -w")
                .calls(7)
                .recent(generic_recent())
                .assistant(&generic_assistant("Watch pod status")),
            Session::blank("rust-crane", "Archive old migrations", Idle, "25m")
                .assistant(&generic_assistant("Archive old migrations")),
            Session::blank("frost-wren", "Tidy up README", Idle, "50m")
                .assistant(&generic_assistant("Tidy up README")),
        ],
    };

    let ci_names = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta"];
    let mut ci_sessions = Vec::with_capacity(22);
    for i in 0..22u32 {
        let nick = format!("runner-{}", ci_names[(i as usize) % 8]);
        let s = if i % 7 == 0 {
            Session::blank(&nick, "Investigate flaky runner shard", NeedsYou, &format!("{}m", 5 + i))
                .wait(5 + i)
                .assistant(&generic_assistant("Investigate flaky runner shard"))
        } else if i % 11 == 0 {
            Session::blank(&nick, "Scale runner pool to 40?", Question, "2m")
                .wait(2)
                .assistant("Scale runner pool to 40?")
                .recent(generic_recent())
        } else if i % 5 == 0 {
            Session::blank(&nick, "Retired shard cleanup", Idle, "30m")
                .assistant(&generic_assistant("Retired shard cleanup"))
        } else {
            Session::blank(&nick, "Fleet health check", Running, "1m")
                .action("running: fleet-health-check.sh")
                .calls(7)
                .recent(generic_recent())
                .assistant(&generic_assistant("Fleet health check"))
        };
        ci_sessions.push(s);
    }
    let ci_fleet_runner = Project { name: "ci-fleet-runner".into(), sessions: ci_sessions };

    let tiny_service = Project {
        name: "tiny-service".into(),
        sessions: vec![Session::blank("ivory-crane", "Bump lockfile", Idle, "5m")
            .assistant(&generic_assistant("Bump lockfile"))],
    };

    Fixture {
        projects: vec![
            web_dashboard,
            infra_tools,
            scratch_cli,
            docs_site,
            mobile_app,
            big_monorepo,
            ci_fleet_runner,
            tiny_service,
        ],
        synthetic_n: 0,
        tick_n: 0,
    }
}
