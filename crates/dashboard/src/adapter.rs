//! The `HarnessAdapter` boundary itself — `docs/specs/dashboard/client.md`
//! R1.3 (full) and R1.4. This is the seam the T09 contract calls
//! "disproportionately expensive to retrofit": the core depends on this
//! trait and these two types only, never on any harness's wire protocol.
//!
//! An adapter's mechanism (REST polling, SSE, a hook-based listener,
//! file-tailing — R1.3 lists all four as equally valid) is entirely its own
//! business. All this trait asks for is: watch your harness however you
//! need to, and push whole-session-state upserts (or "this session is
//! gone") onto the channel you're handed.

use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use crate::snapshot::{HarnessKind, SessionId, SessionSnapshot};

/// What an adapter ever puts on the shared channel (`client.md` R1.4): a
/// whole-state upsert, keyed by session identity, or an explicit
/// "this session is gone" tombstone (R1.7's mechanism half — see
/// `opencode/reconcile.rs` for where the opencode adapter fires this).
/// There is no third, incremental variant — an adapter that wants to change
/// one field of a session re-sends the whole snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    // Boxed: `SessionSnapshot` is a few hundred bytes (it carries several
    // `String`/`Vec<String>` fields worth of tile content), and `Gone` is
    // just an id — without boxing, every `SessionEvent` on the channel
    // would pay `Snapshot`'s size even when it's a `Gone`.
    Snapshot(Box<SessionSnapshot>),
    Gone(SessionId),
}

/// One adapter per harness the dashboard can watch. `kind()` identifies
/// which harness this adapter speaks for (used to build every `SessionId`
/// it emits, and by R1.8's harness-tag slot once a second adapter exists).
/// `run` starts whatever background work the adapter needs and returns a
/// handle to it; the adapter owns its own task(s) and pushes every event it
/// produces onto `sink` until that handle is dropped or aborted.
///
/// `self: Box<Self>` (rather than `&self`) because starting an adapter
/// consumes it — there's no meaningful "adapter that hasn't been started
/// yet but might be started twice" state to preserve.
pub trait HarnessAdapter: Send {
    fn kind(&self) -> HarnessKind;

    fn run(self: Box<Self>, sink: UnboundedSender<SessionEvent>) -> JoinHandle<()>;
}
