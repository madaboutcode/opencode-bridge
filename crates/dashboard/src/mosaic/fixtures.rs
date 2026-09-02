//! Representative `SessionSnapshot`/`NamingClaimMap` scenarios for this
//! module's own tests and for the `mosaic_dump` example's render evidence
//! (T11 contract, acceptance criterion 5: capture evidence at the R5.8
//! design center, a zero-active case, a below-40x12 case, and a
//! single-low-weight-project sliver check). Plays the role the spike's
//! `fixture.rs` played, except built from the real `SessionSnapshot`/
//! `NamingClaimMap` types instead of throwaway fixture structs — this is
//! what a real `HarnessAdapter` + T10's claim map would hand a caller,
//! shaped by hand instead of by a live opencode server.
//!
//! `#[cfg(any(test, feature = "mosaic-fixtures"))]` gates aren't used here
//! deliberately: this module has no opencode-specific knowledge (T11
//! contract, acceptance criterion 6 applies to it too), so there's no
//! encapsulation reason to hide it behind a feature flag, and the
//! `mosaic_dump` example needs it visible as an ordinary crate item.

use crate::naming::{LiveSession, NamingClaimMap};
use crate::snapshot::{
    AttentionState, HarnessKind, ProjectId, SessionId, SessionSnapshot, Timestamp,
};
use std::path::PathBuf;

const KIND: HarnessKind = HarnessKind("fixture");

fn sid(id: &str) -> SessionId {
    SessionId::new(KIND, id)
}

fn pid(path: &str) -> ProjectId {
    ProjectId::from_canonical(PathBuf::from(path))
}

fn ms_ago(now: Timestamp, secs: i64) -> Timestamp {
    Timestamp::from_epoch_millis(now.epoch_millis() - secs * 1000)
}

struct Builder {
    now: Timestamp,
    sessions: Vec<SessionSnapshot>,
    live: Vec<LiveSession>,
}

impl Builder {
    fn new(now: Timestamp) -> Self {
        Self {
            now,
            sessions: vec![],
            live: vec![],
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        project: &str,
        id: &str,
        parent: Option<&str>,
        attention: AttentionState,
        current_action: Option<&str>,
        wire_title: Option<&str>,
        final_assistant_text: Option<&str>,
        last_user_prompt: Option<&str>,
        files_touched: Vec<&str>,
        recent_actions: Vec<&str>,
        created_secs_ago: i64,
    ) -> &mut Self {
        let created_at = ms_ago(self.now, created_secs_ago);
        self.sessions.push(SessionSnapshot {
            session_id: sid(id),
            project_id: pid(project),
            parent_id: parent.map(sid),
            attention,
            current_action: current_action.map(String::from),
            wire_title: wire_title.map(String::from),
            final_assistant_text: final_assistant_text.map(String::from),
            last_user_prompt: last_user_prompt.map(String::from),
            files_touched: files_touched.into_iter().map(String::from).collect(),
            recent_actions: recent_actions.into_iter().map(String::from).collect(),
            created_at,
            last_updated: self.now,
        });
        if parent.is_none() {
            self.live.push(LiveSession {
                project_id: pid(project),
                session_id: sid(id),
                created_at,
            });
        }
        self
    }

    fn finish(self) -> (Vec<SessionSnapshot>, NamingClaimMap, Timestamp) {
        let mut naming = NamingClaimMap::new();
        naming.claim_batch(self.live);
        (self.sessions, naming, self.now)
    }
}

/// `overview.md` R5.8's design center: ~4 projects, ~2 sessions each (8
/// sessions total), including one subagent and one idle-only project (to
/// exercise the footer's `hidden: <name> (N idle)` line alongside the main
/// packing).
pub fn design_center() -> (Vec<SessionSnapshot>, NamingClaimMap, Timestamp) {
    let now = Timestamp::from_epoch_millis(1_000_000_000);
    let mut b = Builder::new(now);

    b.push(
        "/repo/web-dashboard",
        "wd-1",
        None,
        AttentionState::NeedsYou {
            question: true,
            turn_ended: ms_ago(now, 9 * 60),
        },
        None,
        Some("Add multiple code titles with edit support"),
        Some("I found three candidates for removal. Which file would you like me to delete?"),
        Some("Clean up the old title components and remove whatever's unused"),
        vec![],
        vec!["grep: LegacyTitle", "read: LegacyTitle.tsx"],
        20 * 60,
    );
    b.push(
        "/repo/web-dashboard",
        "wd-2",
        None,
        AttentionState::Running {
            turn_started: ms_ago(now, 3 * 60),
        },
        Some("editing: render.rs"),
        Some("Reviewing dashboard requirements doc for session card layout"),
        None,
        Some("Fold the brainstorm learnings back into the requirements doc"),
        vec!["requirements.md", "render.rs"],
        vec!["read: requirements.md", "grep: R5.2", "shell: cargo build"],
        30 * 60,
    );
    b.push(
        "/repo/web-dashboard",
        "wd-2-sub",
        Some("wd-2"),
        AttentionState::Running {
            turn_started: ms_ago(now, 60),
        },
        Some("editing: layout.rs"),
        None,
        None,
        None,
        vec![],
        vec![],
        60,
    );

    b.push(
        "/repo/infra-tools",
        "it-1",
        None,
        AttentionState::NeedsYou { question: true, turn_ended: ms_ago(now, 6 * 60) },
        None,
        Some("Apply terraform destroy for staging, are you sure?"),
        Some("terraform plan -destroy reports 12 resources in staging. Confirm destroy of 12 resources?"),
        Some("tear down staging, we're rebuilding it from the new module"),
        vec![],
        vec!["shell: terraform init", "shell: terraform plan -destroy"],
        40 * 60,
    );
    b.push(
        "/repo/infra-tools",
        "it-2",
        None,
        AttentionState::Idle {
            last_update: ms_ago(now, 51 * 60),
        },
        None,
        Some("Rewrite deploy script"),
        Some("Rewrote deploy.sh as deploy/run.py with --dry-run."),
        Some("rewrite the deploy script in python with a dry run flag"),
        vec![],
        vec![],
        90 * 60,
    );

    b.push(
        "/repo/mobile-app",
        "ma-1",
        None,
        AttentionState::Running {
            turn_started: ms_ago(now, 6 * 60),
        },
        Some("editing: NotificationManager.swift"),
        Some("Fix push notification token refresh"),
        None,
        Some("Push tokens go stale after the app is backgrounded for a day; fix the refresh path"),
        vec!["NotificationManager.swift", "TokenStore.swift"],
        vec!["grep: didRegister", "read: AppDelegate.swift"],
        25 * 60,
    );
    b.push(
        "/repo/mobile-app",
        "ma-2",
        None,
        AttentionState::NeedsYou {
            question: false,
            turn_ended: ms_ago(now, 60),
        },
        None,
        Some("Update app icon assets"),
        Some("Regenerated all 18 icon sizes. Done."),
        Some("update the app icon from the new 1024 png in Design/"),
        vec![],
        vec![],
        15 * 60,
    );

    b.push(
        "/repo/scratch-cli",
        "sc-1",
        None,
        AttentionState::Running {
            turn_started: ms_ago(now, 45),
        },
        Some("shell: cargo bench"),
        Some("Prototype a faster arg parser"),
        None,
        Some("try a hand-rolled parser and bench it against clap"),
        vec!["src/args.rs"],
        vec!["read: src/main.rs", "write: src/args.rs"],
        10 * 60,
    );

    // Idle-only project — excluded from region packing (R5.1), appears
    // only in the footer's `hidden:` line.
    b.push(
        "/repo/docs-site",
        "ds-1",
        None,
        AttentionState::Idle {
            last_update: ms_ago(now, 40 * 60),
        },
        None,
        Some("Fix broken anchor links"),
        Some("Fixed."),
        None,
        vec![],
        vec![],
        120 * 60,
    );

    b.finish()
}

/// R9: every session idle — nothing active in the current window.
pub fn zero_active() -> (Vec<SessionSnapshot>, NamingClaimMap, Timestamp) {
    let now = Timestamp::from_epoch_millis(1_000_000_000);
    let mut b = Builder::new(now);
    b.push(
        "/repo/web-dashboard",
        "wd-1",
        None,
        AttentionState::Idle {
            last_update: ms_ago(now, 25 * 60),
        },
        None,
        Some("Add multiple code titles with edit support"),
        None,
        None,
        vec![],
        vec![],
        60 * 60,
    );
    b.push(
        "/repo/infra-tools",
        "it-1",
        None,
        AttentionState::Idle {
            last_update: ms_ago(now, 40 * 60),
        },
        None,
        Some("Rewrite deploy script"),
        None,
        None,
        vec![],
        vec![],
        90 * 60,
    );
    for i in 0..4 {
        b.push(
            "/repo/mobile-app",
            &format!("ma-{i}"),
            None,
            AttentionState::Idle {
                last_update: ms_ago(now, (30 + i * 5) * 60),
            },
            None,
            Some("Localize onboarding screens"),
            None,
            None,
            vec![],
            vec![],
            (100 + i * 10) * 60,
        );
    }
    b.finish()
}

/// One project with a single low-weight (idle-excluded-none, one active)
/// session against a much heavier one — a sliver check: R5's squarify pass
/// must not draw the light project as an unreadable sliver (`layout.md`
/// R5.5's minimum sizes / `squarify.rs`'s own "no sliver bars" guarantee).
pub fn single_low_weight_project() -> (Vec<SessionSnapshot>, NamingClaimMap, Timestamp) {
    let now = Timestamp::from_epoch_millis(1_000_000_000);
    let mut b = Builder::new(now);
    for i in 0..6 {
        b.push(
            "/repo/big-monorepo",
            &format!("bm-{i}"),
            None,
            AttentionState::Running {
                turn_started: ms_ago(now, (i + 1) * 60),
            },
            Some("running: bazel build //..."),
            Some("Build the monorepo"),
            None,
            None,
            vec![],
            vec!["shell: bazel build //...", "shell: bazel test //..."],
            (i + 1) * 120,
        );
    }
    b.push(
        "/repo/tiny-service",
        "ts-1",
        None,
        AttentionState::Running {
            turn_started: ms_ago(now, 30),
        },
        Some("shell: cargo check"),
        Some("Bump lockfile"),
        None,
        None,
        vec![],
        vec![],
        30,
    );
    b.finish()
}
