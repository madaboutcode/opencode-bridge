//! In-memory map of sessions this bridge launched or has been told to
//! follow up on. opencode2's server is the source of truth for session
//! state (SPEC.md §3 "Statefulness") — this registry only remembers what
//! *this bridge process* needs to know: whether a session is ours to
//! notify on, and the idempotency guard for that notification.

use crate::error::Result;
use crate::opencode::ModelRef;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Running,
    Succeeded,
    Failed,
    Interrupted,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Running => "running",
            Status::Succeeded => "succeeded",
            Status::Failed => "failed",
            Status::Interrupted => "interrupted",
        }
    }

    /// Parses opencode's terminal `outcome` string (SPEC.md §1: "Terminal
    /// turn events ... session.execution.succeeded / failed / interrupted").
    /// Anything else means the API contract changed underneath us — reject
    /// loudly rather than guess.
    pub fn from_outcome(outcome: &str) -> Result<Self> {
        match outcome {
            "succeeded" => Ok(Status::Succeeded),
            "failed" => Ok(Status::Failed),
            "interrupted" => Ok(Status::Interrupted),
            other => Err(format!(
                "unrecognized opencode outcome {other:?} — SPEC.md §1 only documents succeeded/failed/interrupted"
            )
            .into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tracked {
    pub prompt: String,
    pub model: Option<ModelRef>,
    pub agent: Option<String>,
    pub notify: bool,
    pub status: Status,
    pub last_text: Option<String>,
    pub created: SystemTime,
    /// True once the CC callback has fired for the current turn. Reset by
    /// `reset_for_followup` so a second `opencode_send` on the same session
    /// can notify again.
    notified: bool,
}

pub struct Registry {
    sessions: Mutex<HashMap<String, Tracked>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Register a session BEFORE calling `/prompt` (SPEC.md §5 race guard):
    /// the SSE consumer must never observe an event for a session it
    /// doesn't know about yet.
    pub fn register(
        &self,
        session_id: String,
        prompt: String,
        model: Option<ModelRef>,
        agent: Option<String>,
        notify: bool,
    ) {
        let tracked = Tracked {
            prompt,
            model,
            agent,
            notify,
            status: Status::Running,
            last_text: None,
            created: SystemTime::now(),
            notified: false,
        };
        let mut guard = self.sessions.lock().expect("registry mutex poisoned");
        guard.insert(session_id, tracked);
    }

    /// Removes a tracked session. Used to roll back a registration whose
    /// `/prompt` failed — see `tools::task`. We don't want to leave the
    /// SSE consumer watching an "ours" session we never actually drove
    /// (and potentially fire a CC callback on the orphan's terminal event
    /// later).
    pub fn unregister(&self, session_id: &str) {
        self.sessions
            .lock()
            .expect("registry mutex poisoned")
            .remove(session_id);
    }

    pub fn is_tracked(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .expect("registry mutex poisoned")
            .contains_key(session_id)
    }

    /// Re-arms an already-tracked session for a new followup turn: resets
    /// status to running and clears the notified flag so the next terminal
    /// event fires the callback again.
    pub fn reset_for_followup(&self, session_id: &str, prompt: String, notify: bool) {
        let mut guard = self.sessions.lock().expect("registry mutex poisoned");
        if let Some(t) = guard.get_mut(session_id) {
            t.prompt = prompt;
            t.notify = notify;
            t.status = Status::Running;
            t.notified = false;
        }
    }

    /// Atomically claims the right to notify for this session's current
    /// turn: returns the tracked entry the first time it's called since the
    /// session was (re)armed, and `None` on every later call for the same
    /// turn. This is the idempotency guard for the SSE missed-event/
    /// reconnect race (SPEC.md §5) — a terminal event can be observed twice
    /// (once live, once via reconcile) and must only notify once.
    ///
    /// INVARIANT (SPEC.md §8): this is the ONLY path that fires a CC
    /// callback, and it only ever fires for a `session_id` this process put
    /// in the registry itself (via `register`/`reset_for_followup`). A
    /// session merely discovered via title/metadata origin-match (see
    /// `opencode_list`'s `same_origin` tag) is never inserted here and can
    /// never be notified. Origin is a label for listing, never a capability.
    pub fn claim_notification(&self, session_id: &str) -> Option<Tracked> {
        let mut guard = self.sessions.lock().expect("registry mutex poisoned");
        let t = guard.get_mut(session_id)?;
        if t.notified {
            return None;
        }
        t.notified = true;
        Some(t.clone())
    }

    /// Same atomic claim as `claim_notification`, but returns an RAII
    /// guard instead of the tracked snapshot directly. Use this instead of
    /// `claim_notification` whenever the claim needs to be held across a
    /// fallible/cancellable operation (currently: `wait_and_finish`'s
    /// `/wait` call) — see `NotifyClaim` for why.
    pub fn claim_notification_guard(&self, session_id: &str) -> Option<NotifyClaim<'_>> {
        let mut guard = self.sessions.lock().expect("registry mutex poisoned");
        let t = guard.get_mut(session_id)?;
        if t.notified {
            return None;
        }
        t.notified = true;
        drop(guard);
        Some(NotifyClaim {
            registry: self,
            session_id: session_id.to_string(),
            committed: false,
        })
    }

    pub fn set_result(&self, session_id: &str, status: Status, output: Option<String>) {
        let mut guard = self.sessions.lock().expect("registry mutex poisoned");
        if let Some(t) = guard.get_mut(session_id) {
            t.status = status;
            t.last_text = output;
        }
    }

    pub fn running_session_ids(&self) -> Vec<String> {
        self.sessions
            .lock()
            .expect("registry mutex poisoned")
            .iter()
            .filter(|(_, t)| t.status == Status::Running)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// All tracked sessions, newest first.
    pub fn list(&self) -> Vec<(String, Tracked)> {
        let guard = self.sessions.lock().expect("registry mutex poisoned");
        let mut v: Vec<_> = guard.iter().map(|(k, t)| (k.clone(), t.clone())).collect();
        v.sort_by(|a, b| b.1.created.cmp(&a.1.created));
        v
    }
}

/// RAII guard for a notification claim taken via `claim_notification_guard`.
///
/// A plain `claim_notification()` call leaks if the holder never reaches
/// the point where it would normally release/keep the claim — a `?` on a
/// later fallible step, an early `return Err(..)`, or the whole task being
/// cancelled (CC killing a `tools/call` mid-`/wait`) all skip past
/// hand-written cleanup. This guard makes cleanup unconditional: `Drop`
/// runs on every one of those exits because it's a stack local, not
/// something reached by falling through code.
///
/// - Drop without calling `commit()` ⇒ release the claim (`notified =
///   false`) and force `notify = true`: the holder couldn't report the
///   result itself, so the eventual terminal event must still notify.
/// - Call `commit()` ⇒ keep the claim held: the holder is about to report
///   the result another way (a synchronous tool reply) and the async
///   callback must NOT also fire for this turn.
pub struct NotifyClaim<'a> {
    registry: &'a Registry,
    session_id: String,
    committed: bool,
}

impl NotifyClaim<'_> {
    /// Keeps the claim. Call this only once the result has actually been
    /// reported through this claim's channel (e.g. returned in a
    /// synchronous tool reply) — after this, nothing else will notify for
    /// this turn.
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for NotifyClaim<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        // Never panic in a destructor: a poisoned mutex here means some
        // other thread already panicked while holding it, so there's
        // nothing safe left to do but leave the claim as-is and return.
        let Ok(mut guard) = self.registry.sessions.lock() else {
            return;
        };
        if let Some(t) = guard.get_mut(&self.session_id) {
            t.notified = false;
            t.notify = true;
        }
    }
}

#[cfg(test)]
mod tests {
    //! Locks the contracts SPEC §7/§8 lean on: outcome parsing (wire
    //! format boundary), claim idempotency (missed-event safety),
    //! followup re-arming (per-turn claim reset), and unregister (the
    //! prompt-failure rollback path).
    use super::*;

    #[test]
    fn status_from_outcome_recognises_three_terminal_values() {
        assert_eq!(
            Status::from_outcome("succeeded").unwrap(),
            Status::Succeeded
        );
        assert_eq!(Status::from_outcome("failed").unwrap(), Status::Failed);
        assert_eq!(
            Status::from_outcome("interrupted").unwrap(),
            Status::Interrupted
        );
    }

    #[test]
    fn status_from_outcome_rejects_unknown_values() {
        // SPEC.md §1 enumerates exactly three terminal outcomes. A
        // new value means the API contract changed underneath us —
        // reject loudly rather than guess.
        assert!(Status::from_outcome("pending").is_err());
        assert!(Status::from_outcome("").is_err());
        assert!(Status::from_outcome("Succeeded").is_err()); // case-sensitive
    }

    #[test]
    fn status_as_str_is_lowercase_wire_format() {
        assert_eq!(Status::Running.as_str(), "running");
        assert_eq!(Status::Succeeded.as_str(), "succeeded");
        assert_eq!(Status::Failed.as_str(), "failed");
        assert_eq!(Status::Interrupted.as_str(), "interrupted");
    }

    #[test]
    fn claim_notification_is_idempotent_per_turn() {
        // Two claims on the same session for the same turn must yield
        // exactly one notification (the first) — this is the SSE
        // missed-event/reconnect safety net (SPEC §7.2).
        let reg = Registry::new();
        reg.register("ses_a".into(), "p".into(), None, None, true);
        assert!(reg.claim_notification("ses_a").is_some());
        assert!(reg.claim_notification("ses_a").is_none());
    }

    #[test]
    fn reset_for_followup_rearms_the_claim() {
        // After a followup is queued, the next claim must succeed (the
        // terminal event for the new turn must fire a fresh notify).
        let reg = Registry::new();
        reg.register("ses_a".into(), "first".into(), None, None, true);
        assert!(reg.claim_notification("ses_a").is_some());
        reg.reset_for_followup("ses_a", "second".into(), true);
        assert!(reg.claim_notification("ses_a").is_some());
    }

    #[test]
    fn unregister_removes_the_entry() {
        let reg = Registry::new();
        reg.register("ses_a".into(), "p".into(), None, None, true);
        assert!(reg.is_tracked("ses_a"));
        reg.unregister("ses_a");
        assert!(!reg.is_tracked("ses_a"));
        assert!(reg.claim_notification("ses_a").is_none());
    }
}
