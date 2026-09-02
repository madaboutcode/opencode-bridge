//! The core-facing session model — `docs/specs/dashboard/client.md` R1.4-R1.6.
//!
//! Everything in this file is harness-agnostic: no opencode wire type, no
//! `serde_json::Value`, no raw tool name ever appears here (T09 contract,
//! acceptance criterion 1). An adapter's job is to translate whatever its
//! harness says into these types before anything downstream ever sees it.

use std::path::{Path, PathBuf};

/// Milliseconds since the Unix epoch. This is the timestamp representation
/// the whole snapshot model uses — it happens to match opencode's own wire
/// timestamps (`SessionTime.created`/`updated`, epoch-ms) so the opencode
/// adapter doesn't do pointless conversions, but the type itself carries no
/// opencode dependency: it's just an integer with a clock attached.
///
/// Render code (T11) is expected to turn this into an elapsed-time string
/// (`"9m"`, `"45s"`) at redraw time, every ~250ms per `client.md` R1.4's
/// design note — which is exactly why this is a timestamp and not a
/// pre-rendered string: a baked "Nm ago" string goes stale between
/// snapshots, a timestamp never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub i64);

impl Timestamp {
    pub fn from_epoch_millis(millis: i64) -> Self {
        Timestamp(millis)
    }

    pub fn now() -> Self {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_millis() as i64;
        Timestamp(millis)
    }

    pub fn epoch_millis(self) -> i64 {
        self.0
    }
}

/// Tags which harness a session came from. A plain string newtype rather
/// than an enum — the core boundary (`client.md` R1.3) is built so a second
/// harness can register itself without the core's own types changing; an
/// enum listing harness kinds would defeat that. `R1.8`'s harness-tag slot
/// reads this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HarnessKind(pub &'static str);

/// Session identity is the full `(harness kind, harness-native session id)`
/// tuple (`client.md` R1.5) — not the raw id alone, so two harnesses that
/// both happen to hand out small sequential ids (or the literal same string)
/// never collide.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId {
    pub harness: HarnessKind,
    pub native_id: String,
}

impl SessionId {
    pub fn new(harness: HarnessKind, native_id: impl Into<String>) -> Self {
        Self {
            harness,
            native_id: native_id.into(),
        }
    }
}

/// Project identity is the canonical git-repository toplevel path of a
/// session's working directory, or the canonicalized working directory
/// itself when there's no repo (`client.md` R1.6, ported from the T01
/// spike — see `project_identity.rs`). Never an adapter-specific
/// placeholder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectId(PathBuf);

impl ProjectId {
    /// Wraps an already-canonicalized path. Kept private-ish (only
    /// `project_identity.rs`'s resolver and, for the documented degraded
    /// path, this crate's opencode adapter construct one) so nothing
    /// downstream can manufacture a `ProjectId` from an un-canonicalized
    /// string and quietly violate R1.6's identity guarantee.
    pub(crate) fn from_canonical(path: PathBuf) -> Self {
        ProjectId(path)
    }

    /// Degraded-path constructor: wraps a directory path as-is, without
    /// canonicalizing or resolving it against git. Used only when
    /// resolution itself failed (e.g. the directory no longer exists) —
    /// see the `FALLBACK-OK` comment at its one call site in
    /// `opencode/reconcile.rs`. A project box keyed by this identity may
    /// not compare equal to one keyed by the canonical form of the same
    /// physical location; that's the accepted cost of not crashing the
    /// whole adapter over one session's bad directory.
    pub(crate) fn from_uncanonicalized(path: &Path) -> Self {
        ProjectId(path.to_path_buf())
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// The three-state attention model (`visuals.md` R6.7), each carrying the
/// timestamp basis its own elapsed-time display needs — turn-start for
/// running, turn-end for needs-you, last-update for idle. Storing the
/// timestamp (not a rendered "Nm ago" string) is what lets a redraw compute
/// fresh elapsed time every frame without the adapter re-emitting anything.
///
/// The opencode adapter (this task) only ever constructs `Running` and
/// `NeedsYou` — see `opencode/reconcile.rs`'s `is_running`. `Idle` exists in
/// this shared type because `overview.md` R3's active-window filter is a
/// *display* reclassification of a long-quiet `NeedsYou` session, computed
/// by the core/T12 from the snapshot's dedicated `last_updated` field (see
/// `SessionSnapshot` below) — not something the adapter can compute itself,
/// since the window `W` is a core-owned, keyboard-adjustable setting
/// (`interactions.md` R8) the adapter has no visibility into. Flagging this
/// as an explicit design call: T09's contract lists `Idle` as part of this
/// enum's shape, but also states T12 owns the active/idle window
/// computation from `last_updated` — the two are consistent only if `Idle`
/// is something downstream constructs, not something this adapter emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionState {
    Running {
        turn_started: Timestamp,
    },
    NeedsYou {
        question: bool,
        turn_ended: Timestamp,
    },
    Idle {
        last_update: Timestamp,
    },
}

/// One session's complete current state, as handed across the
/// `HarnessAdapter` boundary (`client.md` R1.4). Adapters push whole-state
/// upserts of this type — never a partial/incremental update — so the core
/// never has to fold adapter-side state into something it tracks itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub project_id: ProjectId,
    /// Set when this session is a subagent delegation — points at the
    /// parent session's identity (`layout.md` R5.6). `None` for an
    /// ordinary top-level session.
    pub parent_id: Option<SessionId>,
    pub attention: AttentionState,
    /// Already-rendered current-action text (`client.md` R6.5), e.g.
    /// `"editing: foo.rs"` — never a raw tool name or argument object.
    /// `None` until the session's first tool call.
    pub current_action: Option<String>,
    /// The harness's own session title/description, verbatim.
    pub wire_title: Option<String>,
    /// The most recent completed turn's final assistant text — feeds the
    /// question/needs-you elastic blocks (`layout.md` R5.3). `None` if no
    /// turn has completed yet, or the adapter hasn't fetched it (see
    /// `opencode/reconcile.rs`'s "only fetched for non-running sessions"
    /// note).
    pub final_assistant_text: Option<String>,
    /// The most recent user prompt's text, rendered as `you: <text>`
    /// content (`layout.md` R5.3's needs-you/question blocks reference
    /// this verbatim, without the `you: ` prefix — that prefix is a render
    /// concern).
    pub last_user_prompt: Option<String>,
    /// Files touched during the session's current turn. Reset when a new
    /// turn starts.
    pub files_touched: Vec<String>,
    /// Bounded ring of past action lines, oldest first, never including the
    /// current value of `current_action` (`layout.md` R5.3's "extended"
    /// running block, priority 7).
    pub recent_actions: Vec<String>,
    /// When the session itself was created — T10's claim-order resolution
    /// reads this (`visuals.md` R6.8, "claim order is pinned").
    pub created_at: Timestamp,
    /// Refreshed on every snapshot this adapter emits for this session,
    /// SSE- or reconcile-driven alike. This is deliberately a *different*
    /// field from the per-state timestamp inside `attention` — it's what
    /// `overview.md` R3's active/idle window filter reads (T12 computes
    /// "active" from time-since-this-field, never from `attention`'s own
    /// basis or any opencode-native "updated" value).
    pub last_updated: Timestamp,
}
