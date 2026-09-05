//! Pure in-memory Claude session transitions and snapshot construction (see
//! `crates/dashboard/src/claude/DESIGN.md`). Synchronous and free of any
//! socket or channel work on purpose: the adapter loop in [`super::ClaudeAdapter`]
//! owns the async receive/send glue, while this module holds all per-session
//! state the Claude adapter keeps, and turns each validated `hook` envelope
//! into a complete provider-neutral [`SessionSnapshot`] or a `Gone` tombstone.
//!
//! This module maps all fifteen R13 events, per the attention/content table
//! sealed 2026-09-05 and widened in its advisor-reviewed round 2
//! (`tasks/2026-09-05-claude-dashboard-activity-capture.spec-delta.md`;
//! `docs/specs/dashboard/claude.md` R13/R14). Every accepted event produces
//! exactly one complete snapshot for whichever session it targets, or (for
//! `SubagentStop`/`SessionEnd`) one `Gone` tombstone.
//!
//! **Subagent modeling.** An event carrying `agent_id` (`ClaudeEvent::
//! agent_id()`) targets a *subagent* session, not the top-level one: it is
//! tracked as its own entry in the same session map, keyed
//! `"{top_session_id}:{agent_id}"`, with `parent_id` set to the top-level
//! session's identity and the same `project_id` (shared from the already-
//! resolved parent when tracked; resolved independently from the event's own
//! `cwd` when the parent has not been seen yet — event ordering is not
//! guaranteed, R17). `SubagentStart` creates it (or updates it, preserving
//! `created_at`, mirroring `SessionStart`'s pattern); a tool/`Stop` event
//! carrying `agent_id` creates the subagent session defensively if
//! `SubagentStart` was never observed for it. `SubagentStop` tombstones it
//! exactly like `SessionEnd` does the top-level session, including for a
//! subagent this state never saw.
//!
//! **Generic exit-path clearing (R14, advisor review round 2).**
//! `PermissionRequest`/`Elicitation` set a tracked session's
//! `pending_tool_use_id` and put it in `NeedsYou { question: true }`. That
//! pending state clears on **any subsequent accepted event carrying the same
//! `tool_use_id`** — `PermissionDenied`, `ElicitationResult`, `PostToolUse`,
//! or `PostToolUseFailure` all qualify, whichever arrives — never a
//! hardcoded "the next `PreToolUse` clears it" assumption: a tool's own
//! `PreToolUse` for a given `tool_use_id` fires *before* the permission
//! check, never after, so it can never be "the next" event. Clearing sets
//! attention back to `Running` *before* the clearing event's own specific
//! mapping runs, so one event can both clear the pending state and carry its
//! own effect (e.g. a `PreToolUse` that clears a pending permission also
//! updates `current_action`/`recent_actions` in the same step — but only
//! `PreToolUse` touches that pair; `PostToolUse`/`PostToolUseFailure` share
//! the same clearing but never touch `current_action`/`recent_actions`, T02
//! finding 2). The same clear also resets `final_assistant_text` to `None`
//! (T02 finding 4, item 9) — a resolved permission/elicitation's
//! synthesized question text must not survive past the moment its pending
//! state clears, for the same reason `UserPromptSubmit` clears it on a new
//! turn. See `clear_pending_tool_use` below.
//!
//! `PermissionRequest`/`Elicitation` carry no `agent_id` field today, so
//! `pending_tool_use_id` is only ever set on the *top-level* tracked
//! session, never a subagent's. But the tool-event handler routes to a
//! *subagent* record whenever the event itself carries `agent_id` — so a
//! subagent's own permission-gated tool call must also clear the top-level
//! record explicitly, or the top-level session gets stuck in `NeedsYou`
//! forever (a real, confirmed bug in an earlier revision). See the
//! tool-event match arm's comment for the fix and its exact assumption; that
//! assumption breaks if a future Claude Code version adds `agent_id` to
//! `PermissionRequest`/`Elicitation`.
//!
//! **`Notification` attention mapping.** `notification_type` (`Option<
//! String>`) drives attention for the two sub-types Claude documents as
//! attention-worthy (R13): `Some("idle_prompt")` maps to `NeedsYou {
//! question: false, .. }` and also clears `turn_started` to `None`,
//! mirroring `Stop`'s own "turn is over" bookkeeping so a later tool event
//! doesn't resurrect a stale start time. `Some("permission_prompt")` or
//! `Some("agent_needs_input")` map to `NeedsYou { question: true, .. }` but
//! leave `turn_started` untouched — unlike `idle_prompt`, the turn may
//! still be running with a subprocess waiting on the user. Any other value
//! (`None`, or a sub-type not in this list) changes nothing but
//! `last_updated`. A `Notification`-driven `NeedsYou` has no dedicated
//! clearing event — it carries no `tool_use_id` to correlate a clear
//! against, unlike `PermissionRequest`/`Elicitation` — so it relies on the
//! next event that unconditionally sets attention (any `UserPromptSubmit`
//! or tool event); this is intentional, not a gap to fix here. `wire_title`
//! and `files_touched` have no evidence-backed source field (R14) and stay
//! `None`/empty for every event.
//!
//! **`Stop` attention mapping.** A subagent `Stop` (`agent_id: Some(_)`)
//! sets `Idle`, not `NeedsYou`: a finished subagent does not need anyone —
//! it is superseded by its own `SubagentStop` tombstone shortly after, and
//! until then should read as done, not blocking. The top-level case
//! (`agent_id: None`) sets `NeedsYou { question: looks_like_question(
//! last_assistant_message), .. }` — Claude's `Stop` carries only text, the
//! same as OpenCode's own turn-end signal, so the same shared, text-based
//! heuristic (`crate::text::looks_like_question`) applies to both.
//!
//! CONTRACT: ClaudeLifecycleState (`docs/specs/dashboard/claude.md`
//! R13-R14; `crates/dashboard/src/claude/DESIGN.md`)
//!
//! GUARANTEES:
//!   - Every session identity is `HarnessKind("claude")`. Every event that
//!     is not `SubagentStop`/`SessionEnd` emits exactly one complete
//!     snapshot for the session it targets (top-level, or a subagent when
//!     `agent_id` is present); `SubagentStop`/`SessionEnd` each emit exactly
//!     one `Gone` and remove the targeted session — including for a native
//!     id/agent id this state has never seen.
//!   - `created_at` is a tracked session's first accepted event's local
//!     receipt time; later events preserve it. `last_updated` is always the
//!     current event's own local receipt time.
//!   - A tracked session's `pending_tool_use_id` clears on any event whose
//!     own `tool_use_id` matches it, regardless of event kind (R14); the
//!     same match also clears `final_assistant_text` (T02 finding 4).
//!   - `current_action`/`recent_actions` (`snapshot.rs`'s invariants: never
//!     a raw tool name, never duplicated between the two) are updated only
//!     by `PreToolUse`, via `action_line::render_action_line` (T02
//!     finding 2) — `PostToolUse`/`PostToolUseFailure` never touch either.
//!   - Project identity resolves through the shared
//!     [`ProjectIdentityCache`]; an unresolvable cwd degrades to the documented
//!     uncanonicalized identity fallback and processing continues.
//!
//! EXPECTS:
//!   - A validated `hook`-produced envelope (protocol version 1, in receipt
//!     order). The adapter loop guarantees this before calling
//!     [`ClaudeState::process`].
//!
//! FAILURE BEHAVIOR:
//!   - A cwd that cannot be resolved degrades one snapshot to the documented
//!     uncanonicalized project identity (FALLBACK-OK, see
//!     `opencode/reconcile.rs`'s identical call site) and never stops the
//!     adapter or other sessions.
//!
//! DOES NOT:
//!   - Read transcripts, infer unobserved events, log or retain raw wire
//!     values, or implement expiry/removal — final staleness policy is
//!     deferred (see `claude.md` R17).

use std::collections::{HashMap, VecDeque};
use std::path::Path;

use crate::adapter::SessionEvent;
use crate::project_identity::{DirResolver, GitDirResolver, ProjectIdentityCache};
use crate::snapshot::{AttentionState, ProjectId, SessionId, SessionSnapshot, Timestamp};
use crate::text::looks_like_question;

use super::action_line;
use super::hook::{ClaudeEvent, ClaudeIpcEnvelope};
use super::KIND;

/// Bound on the recent-actions ring (`claude.md` R14; mirrors the pattern in
/// `opencode/session_state.rs`'s `RECENT_ACTIONS_CAP`). Holds rendered
/// action lines (`action_line::render_action_line`), the same as
/// OpenCode's own ring — not bare tool names (T02, finding 2).
const RECENT_ACTIONS_CAP: usize = 5;

/// What the adapter remembers about one live Claude session — top-level or
/// subagent alike. Enough to rebuild a complete [`SessionSnapshot`] on every
/// event without re-deriving anything from a prior wire value this module
/// does not retain.
struct ClaudeTrackedSession {
    project_id: ProjectId,
    created_at: Timestamp,
    attention: AttentionState,
    /// The sticky "current turn started at" basis a tool event reuses when
    /// no fresher value exists (set by `UserPromptSubmit`/`SubagentStart`,
    /// cleared by `Stop`). Distinct from `attention`'s own `Running`
    /// payload, which is recomputed from this field on every tool event.
    turn_started: Option<Timestamp>,
    current_action: Option<String>,
    recent_actions: VecDeque<String>,
    last_user_prompt: Option<String>,
    final_assistant_text: Option<String>,
    /// `None` for a top-level session; `Some(parent)` for a subagent.
    parent_id: Option<SessionId>,
    /// The `tool_use_id` of an outstanding `PermissionRequest`/`Elicitation`
    /// this session is waiting on, if any. Cleared by
    /// [`clear_pending_tool_use`] the moment any event carrying the same
    /// `tool_use_id` arrives (`claude.md` R14's generic exit-path rule).
    pending_tool_use_id: Option<String>,
}

/// The adapter's in-memory Claude session state. Generic over
/// [`DirResolver`] so unit tests stay fixture-only and never spawn `git`;
/// production uses the default [`GitDirResolver`].
pub(crate) struct ClaudeState<R: DirResolver = GitDirResolver> {
    sessions: HashMap<SessionId, ClaudeTrackedSession>,
    project_cache: ProjectIdentityCache<R>,
}

impl<R: DirResolver> ClaudeState<R> {
    pub(crate) fn new(resolver: R) -> Self {
        Self {
            sessions: HashMap::new(),
            project_cache: ProjectIdentityCache::new(resolver),
        }
    }

    /// Processes one validated `hook`-produced envelope in receipt order and
    /// returns the provider-neutral events it produces.
    pub(crate) fn process(&mut self, envelope: &ClaudeIpcEnvelope) -> Vec<SessionEvent> {
        let receipt =
            Timestamp::from_epoch_millis(envelope.record.received_at.epoch_millis() as i64);
        let cwd = envelope.record.cwd.as_str();
        let top_id = SessionId::new(KIND, envelope.record.session_id.clone());

        match &envelope.record.event {
            ClaudeEvent::SessionStart { .. } => {
                self.ensure_tracked(&top_id, cwd, receipt, None);
                let tracked = self.sessions.get_mut(&top_id).expect("just ensured");
                tracked.attention = AttentionState::Idle {
                    last_update: receipt,
                };
                snapshot_event(&top_id, tracked, receipt)
            }
            ClaudeEvent::UserPromptSubmit { prompt } => {
                self.ensure_tracked(&top_id, cwd, receipt, None);
                let tracked = self.sessions.get_mut(&top_id).expect("just ensured");
                tracked.attention = AttentionState::Running {
                    turn_started: receipt,
                };
                tracked.turn_started = Some(receipt);
                tracked.last_user_prompt = Some(prompt.clone());
                // A new turn's stale "previous answer" (finding 4, item 6)
                // must not persist into whatever NeedsYou state this new
                // turn eventually reaches.
                tracked.final_assistant_text = None;
                snapshot_event(&top_id, tracked, receipt)
            }
            // Split from PostToolUse/PostToolUseFailure below (finding 2,
            // item 4): only PreToolUse updates current_action/recent_actions
            // — PostToolUse/PostToolUseFailure share everything else
            // (routing, the generic exit-path clear, forcing attention back
            // to Running) but must not touch either field.
            ClaudeEvent::PreToolUse {
                tool_name,
                tool_input,
                agent_id,
                ..
            } => {
                let target = target_of(&top_id, agent_id.as_deref());
                let parent = parent_of(&top_id, agent_id.as_deref());
                self.ensure_tracked(&target, cwd, receipt, parent);

                // BUG FIX: PermissionRequest/Elicitation carry no agent_id
                // field (see the comment on those arms below), so
                // pending_tool_use_id can only ever be set on the TOP-LEVEL
                // tracked session, never on a subagent's. When this tool
                // event itself carries agent_id, `target` is a *subagent*
                // record, distinct from `top_id` — clearing only there would
                // never match a pending flag that lives on `top_id`, and a
                // subagent's permission-gated tool call would leave the
                // top-level session stuck in NeedsYou forever. Clear the
                // top-level record too whenever it differs from `target`.
                //
                // IMPORTANT: this is only correct because
                // PermissionRequest/Elicitation have no agent_id field
                // *today* — a fact about Claude Code's hook schema, not
                // something the type system enforces. If a future Claude
                // Code version adds agent_id to either event, this fix needs
                // revisiting: pending_tool_use_id could then land on a
                // subagent record instead, and clearing would need to target
                // whichever record it actually landed on, not unconditionally
                // top_id.
                if target != top_id {
                    if let Some(top_tracked) = self.sessions.get_mut(&top_id) {
                        clear_pending_tool_use(top_tracked, &envelope.record.event, receipt);
                    }
                }

                let tracked = self.sessions.get_mut(&target).expect("just ensured");
                clear_pending_tool_use(tracked, &envelope.record.event, receipt);
                tracked.attention = AttentionState::Running {
                    turn_started: tracked.turn_started.unwrap_or(receipt),
                };
                // Only PreToolUse advances the ring/current-action pair, and
                // only with the *previous* call's rendered line, never the
                // bare tool name (finding 2, item 4): pushing the value this
                // call is about to replace, not the value it is setting,
                // is what keeps recent_actions from ever containing the
                // current current_action (snapshot.rs's own invariant).
                let line = action_line::render_action_line(tool_name, tool_input);
                if let Some(previous) = tracked.current_action.take() {
                    push_recent_action(&mut tracked.recent_actions, previous);
                }
                tracked.current_action = Some(line);
                snapshot_event(&target, tracked, receipt)
            }
            ClaudeEvent::PostToolUse { agent_id, .. }
            | ClaudeEvent::PostToolUseFailure { agent_id, .. } => {
                let target = target_of(&top_id, agent_id.as_deref());
                let parent = parent_of(&top_id, agent_id.as_deref());
                self.ensure_tracked(&target, cwd, receipt, parent);

                // Same top-level-pending-clear fix as PreToolUse above — see
                // its comment for the full rationale.
                if target != top_id {
                    if let Some(top_tracked) = self.sessions.get_mut(&top_id) {
                        clear_pending_tool_use(top_tracked, &envelope.record.event, receipt);
                    }
                }

                let tracked = self.sessions.get_mut(&target).expect("just ensured");
                clear_pending_tool_use(tracked, &envelope.record.event, receipt);
                tracked.attention = AttentionState::Running {
                    turn_started: tracked.turn_started.unwrap_or(receipt),
                };
                // Deliberately does not touch current_action/recent_actions
                // (finding 2, item 4) — PreToolUse alone owns that pair.
                snapshot_event(&target, tracked, receipt)
            }
            // NOTE: PermissionRequest/Elicitation have no agent_id field
            // today, so they always ensure/mutate top_id (never a subagent
            // target) and pending_tool_use_id is only ever set here, on the
            // top-level record. The tool-event arm above has a targeted
            // fix that depends on this being true — see its comment. If
            // agent_id is ever added to this event, that fix (and this one)
            // need revisiting together.
            ClaudeEvent::PermissionRequest {
                tool_name,
                tool_use_id,
                tool_input,
            } => {
                self.ensure_tracked(&top_id, cwd, receipt, None);
                let tracked = self.sessions.get_mut(&top_id).expect("just ensured");
                clear_pending_tool_use(tracked, &envelope.record.event, receipt);
                tracked.pending_tool_use_id = Some(tool_use_id.clone());
                tracked.attention = AttentionState::NeedsYou {
                    question: true,
                    turn_ended: receipt,
                };
                // Finding 4, item 7: the Question tile shows what is being
                // asked for, reusing finding 2's own object extraction.
                tracked.final_assistant_text =
                    Some(action_line::render_permission_text(tool_name, tool_input));
                snapshot_event(&top_id, tracked, receipt)
            }
            ClaudeEvent::PermissionDenied { .. } => {
                // Beyond the generic exit-path clear (module doc comment),
                // clear_pending_tool_use itself now also clears
                // final_assistant_text on a matching tool_use_id (finding 4,
                // item 9) — see its own doc comment for why.
                self.ensure_tracked(&top_id, cwd, receipt, None);
                let tracked = self.sessions.get_mut(&top_id).expect("just ensured");
                clear_pending_tool_use(tracked, &envelope.record.event, receipt);
                snapshot_event(&top_id, tracked, receipt)
            }
            // NOTE: same as PermissionRequest above — Elicitation has no
            // agent_id field today, so pending_tool_use_id only ever lands
            // on the top-level record. See the tool-event arm's comment.
            ClaudeEvent::Elicitation {
                tool_use_id,
                elicitation_request,
                ..
            } => {
                self.ensure_tracked(&top_id, cwd, receipt, None);
                let tracked = self.sessions.get_mut(&top_id).expect("just ensured");
                clear_pending_tool_use(tracked, &envelope.record.event, receipt);
                tracked.pending_tool_use_id = Some(tool_use_id.clone());
                tracked.attention = AttentionState::NeedsYou {
                    question: true,
                    turn_ended: receipt,
                };
                // Finding 4, item 8: already natural-language request text,
                // no extraction needed.
                tracked.final_assistant_text = Some(elicitation_request.clone());
                snapshot_event(&top_id, tracked, receipt)
            }
            ClaudeEvent::ElicitationResult { .. } => {
                // Mirrors PermissionDenied: only the generic exit-path clear
                // applies (which also clears final_assistant_text on a
                // match — see clear_pending_tool_use's doc comment).
                self.ensure_tracked(&top_id, cwd, receipt, None);
                let tracked = self.sessions.get_mut(&top_id).expect("just ensured");
                clear_pending_tool_use(tracked, &envelope.record.event, receipt);
                snapshot_event(&top_id, tracked, receipt)
            }
            ClaudeEvent::Notification {
                notification_type, ..
            } => {
                // See the module doc comment's "Notification attention
                // mapping" for the full rationale, incl. why idle_prompt
                // alone clears turn_started.
                self.ensure_tracked(&top_id, cwd, receipt, None);
                let tracked = self.sessions.get_mut(&top_id).expect("just ensured");
                match notification_type.as_deref() {
                    Some("idle_prompt") => {
                        tracked.attention = AttentionState::NeedsYou {
                            question: false,
                            turn_ended: receipt,
                        };
                        tracked.turn_started = None;
                    }
                    Some("permission_prompt") | Some("agent_needs_input") => {
                        tracked.attention = AttentionState::NeedsYou {
                            question: true,
                            turn_ended: receipt,
                        };
                    }
                    _ => {
                        // Any other value (None, or an unrecognized
                        // sub-type): only last_updated (via `receipt`
                        // below) advances.
                    }
                }
                snapshot_event(&top_id, tracked, receipt)
            }
            ClaudeEvent::Stop {
                last_assistant_message,
                agent_id,
                ..
            } => {
                let target = target_of(&top_id, agent_id.as_deref());
                let parent = parent_of(&top_id, agent_id.as_deref());
                self.ensure_tracked(&target, cwd, receipt, parent);
                let tracked = self.sessions.get_mut(&target).expect("just ensured");
                // See the module doc comment's "Stop attention mapping": a
                // subagent Stop settles to Idle (superseded by its own
                // SubagentStop shortly after), only the top-level case gets
                // the question-glyph heuristic.
                tracked.attention = match agent_id {
                    Some(_) => AttentionState::Idle {
                        last_update: receipt,
                    },
                    None => AttentionState::NeedsYou {
                        question: looks_like_question(last_assistant_message),
                        turn_ended: receipt,
                    },
                };
                tracked.final_assistant_text = Some(last_assistant_message.clone());
                tracked.turn_started = None;
                snapshot_event(&target, tracked, receipt)
            }
            ClaudeEvent::StopFailure { .. } => {
                self.ensure_tracked(&top_id, cwd, receipt, None);
                let tracked = self.sessions.get_mut(&top_id).expect("just ensured");
                tracked.attention = AttentionState::NeedsYou {
                    question: false,
                    turn_ended: receipt,
                };
                snapshot_event(&top_id, tracked, receipt)
            }
            ClaudeEvent::SubagentStart {
                agent_id,
                agent_prompt,
                ..
            } => {
                let target = target_of(&top_id, Some(agent_id.as_str()));
                self.ensure_tracked(&target, cwd, receipt, Some(top_id.clone()));
                let tracked = self.sessions.get_mut(&target).expect("just ensured");
                tracked.attention = AttentionState::Running {
                    turn_started: receipt,
                };
                tracked.turn_started = Some(receipt);
                tracked.last_user_prompt = Some(agent_prompt.clone());
                snapshot_event(&target, tracked, receipt)
            }
            ClaudeEvent::SubagentStop { agent_id, .. } => {
                let target = target_of(&top_id, Some(agent_id.as_str()));
                self.sessions.remove(&target);
                vec![SessionEvent::Gone(target)]
            }
            ClaudeEvent::SessionEnd { .. } => {
                self.sessions.remove(&top_id);
                vec![SessionEvent::Gone(top_id)]
            }
        }
    }

    /// Creates the tracked session at `session_id` if absent — preserving
    /// `created_at` when it is already tracked (`SessionStart`'s pattern,
    /// reused for every event that can be a session's first observation,
    /// including a subagent tool event that arrives before `SubagentStart`).
    /// A subagent's project identity is shared from its parent when the
    /// parent is already tracked; otherwise it is resolved independently
    /// from `cwd`, since event ordering is not guaranteed (R17).
    fn ensure_tracked(
        &mut self,
        session_id: &SessionId,
        cwd: &str,
        receipt: Timestamp,
        parent_id: Option<SessionId>,
    ) {
        if self.sessions.contains_key(session_id) {
            return;
        }
        let project_id = match parent_id.as_ref().and_then(|parent| self.sessions.get(parent)) {
            Some(parent_tracked) => parent_tracked.project_id.clone(),
            None => resolve_project_id(&mut self.project_cache, session_id, cwd),
        };
        self.sessions.insert(
            session_id.clone(),
            ClaudeTrackedSession {
                project_id,
                created_at: receipt,
                attention: AttentionState::Idle {
                    last_update: receipt,
                },
                turn_started: None,
                current_action: None,
                recent_actions: VecDeque::new(),
                last_user_prompt: None,
                final_assistant_text: None,
                parent_id,
                pending_tool_use_id: None,
            },
        );
    }
}

/// R14's generic exit-path rule: if `event` carries a `tool_use_id`
/// (`ClaudeEvent::tool_use_id`) that matches `tracked`'s outstanding
/// `pending_tool_use_id`, the pending state clears and attention returns to
/// `Running` — *before* `event`'s own specific mapping runs, so the same
/// event can also carry its own effect (e.g. `PostToolUse` also sets
/// `current_action`). Never assumes which specific event kind clears a given
/// pending state: a mismatched or absent `tool_use_id` is a no-op here.
///
/// **Also clears `final_assistant_text` on the same match** (finding 4, item
/// 9, `tasks/2026-09-05-claude-dashboard-fable-fixes/contracts/
/// T02-tile-content-correctness.md`): `PermissionRequest`/`Elicitation` set
/// `final_assistant_text` to a synthesized "allow: X" / request string
/// (items 7-8) so the Question tile shows what's being asked for. Once this
/// function's match fires — whichever event caused it, `PermissionDenied`,
/// `ElicitationResult`, `PostToolUse`, or `PostToolUseFailure` — that text
/// describes a question that has already been answered, and attention has
/// already moved back to `Running` (whose own tile block list never shows
/// `final_assistant_text`, so the stale text is briefly invisible). Left
/// alone, it would survive in tracked state and resurface if this session
/// later reaches an `Idle`/`NeedsYou(plain)` tile with no intervening
/// `Stop`/`UserPromptSubmit` to overwrite it — the same class of bug item 6
/// fixes for `UserPromptSubmit`, just reachable from every event kind this
/// generic rule already covers, not only the two `PermissionDenied`/
/// `ElicitationResult` arms that (before this fix) were the only ones
/// thought to need it. Clearing only on an actual match — never
/// unconditionally in a calling arm — is deliberate: a mismatched
/// `tool_use_id` means an unrelated event arrived while a different
/// permission is still genuinely pending, and that pending question's text
/// must not be wiped by it (see `mismatched_tool_use_id_does_not_clear_the_
/// pending_permission` in this module's tests).
fn clear_pending_tool_use(tracked: &mut ClaudeTrackedSession, event: &ClaudeEvent, receipt: Timestamp) {
    if let Some(tool_use_id) = event.tool_use_id() {
        if tracked.pending_tool_use_id.as_deref() == Some(tool_use_id) {
            tracked.pending_tool_use_id = None;
            tracked.attention = AttentionState::Running {
                turn_started: tracked.turn_started.unwrap_or(receipt),
            };
            tracked.final_assistant_text = None;
        }
    }
}

/// The subagent-or-top-level session id a tool/`Stop` event targets
/// (`claude.md` R14: presence of `agent_id` routes to the subagent).
fn target_of(top_id: &SessionId, agent_id: Option<&str>) -> SessionId {
    match agent_id {
        Some(id) => SessionId::new(KIND, format!("{}:{}", top_id.native_id, id)),
        None => top_id.clone(),
    }
}

/// The `parent_id` a tool/`Stop` event's targeted session should carry:
/// `Some(top)` when it targets a subagent, `None` for the top-level session
/// itself.
fn parent_of(top_id: &SessionId, agent_id: Option<&str>) -> Option<SessionId> {
    agent_id.map(|_| top_id.clone())
}

/// Pushes `action` onto the bounded recent-actions ring, dropping the oldest
/// entry once at capacity (mirrors `opencode/session_state.rs`'s pattern).
fn push_recent_action(ring: &mut VecDeque<String>, action: String) {
    if ring.len() == RECENT_ACTIONS_CAP {
        ring.pop_front();
    }
    ring.push_back(action);
}

/// Resolves an envelope `cwd` to its canonical project identity through the
/// shared per-session cache. On failure, degrades to the documented
/// uncanonicalized identity so one bad directory never stops the adapter.
fn resolve_project_id<R: DirResolver>(
    project_cache: &mut ProjectIdentityCache<R>,
    session_id: &SessionId,
    cwd: &str,
) -> ProjectId {
    match project_cache.resolve(session_id, Path::new(cwd)) {
        Ok(id) => id,
        Err(_e) => {
            // FALLBACK-OK: identical to the opencode adapter's documented
            // degraded path (`opencode/reconcile.rs::resolve_project_id`) and
            // the design's Failure Domains. Canonicalization requires the
            // directory to exist and resolves via `git`; either can fail for a
            // session whose directory vanished or is momentarily unreadable.
            // Crashing the whole adapter — and every other tracked session with
            // it — over one session's bad cwd would violate the release bar's
            // "other sessions must render correctly" concern far worse than
            // showing this one session under a degraded, uncanonicalized
            // project identity until its directory is healthy again.
            ProjectId::from_uncanonicalized(Path::new(cwd))
        }
    }
}

/// Builds the complete provider-neutral snapshot event for one tracked
/// session. `wire_title` and `files_touched` have no evidence-backed source
/// field (`claude.md` R14) and stay `None`/empty.
fn snapshot_event(
    session_id: &SessionId,
    tracked: &ClaudeTrackedSession,
    last_updated: Timestamp,
) -> Vec<SessionEvent> {
    let snapshot = SessionSnapshot {
        session_id: session_id.clone(),
        project_id: tracked.project_id.clone(),
        parent_id: tracked.parent_id.clone(),
        attention: tracked.attention,
        current_action: tracked.current_action.clone(),
        wire_title: None,
        final_assistant_text: tracked.final_assistant_text.clone(),
        last_user_prompt: tracked.last_user_prompt.clone(),
        files_touched: Vec::new(),
        recent_actions: tracked.recent_actions.iter().cloned().collect(),
        created_at: tracked.created_at,
        last_updated,
    };
    vec![SessionEvent::Snapshot(Box::new(snapshot))]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::hook::{
        ClaudeHookRecord, ReceivedAt, SessionEndReason, SessionStartSource,
        ENVELOPE_PROTOCOL_VERSION,
    };
    use std::io;
    use std::path::PathBuf;

    /// A resolver that never touches the filesystem or spawns `git`: every
    /// directory maps to itself. Keeps the transition tests fixture-only.
    #[derive(Debug, Default, Clone, Copy)]
    struct IdentityResolver;
    impl DirResolver for IdentityResolver {
        fn resolve(&self, dir: &Path) -> io::Result<PathBuf> {
            Ok(dir.to_path_buf())
        }
    }

    /// A resolver that fails for exactly the directory marked in `failing`.
    #[derive(Debug, Default, Clone)]
    struct SelectiveResolver {
        failing: PathBuf,
    }
    impl DirResolver for SelectiveResolver {
        fn resolve(&self, dir: &Path) -> io::Result<PathBuf> {
            if dir == self.failing {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "fixture dir missing",
                ))
            } else {
                Ok(dir.to_path_buf())
            }
        }
    }

    const R1: u64 = 1_700_000_000_100;
    const R2: u64 = 1_700_000_000_200;
    const R3: u64 = 1_700_000_000_300;
    const R4: u64 = 1_700_000_000_400;

    fn envelope(event: ClaudeEvent, session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        ClaudeIpcEnvelope {
            protocol_version: ENVELOPE_PROTOCOL_VERSION,
            record: ClaudeHookRecord {
                session_id: session.to_string(),
                cwd: cwd.to_string(),
                event,
                received_at: ReceivedAt(received_at),
            },
        }
    }

    fn start(session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::SessionStart {
                source: Some(SessionStartSource::Startup),
                model: None,
            },
            session,
            cwd,
            received_at,
        )
    }

    fn prompt(session: &str, cwd: &str, text: &str, received_at: u64) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::UserPromptSubmit {
                prompt: text.to_string(),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn pre_tool(
        session: &str,
        cwd: &str,
        tool_name: &str,
        agent_id: Option<&str>,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::PreToolUse {
                tool_name: tool_name.to_string(),
                tool_use_id: "call-1".to_string(),
                tool_input: "{}".to_string(),
                agent_id: agent_id.map(str::to_string),
                agent_type: None,
            },
            session,
            cwd,
            received_at,
        )
    }

    /// Like `pre_tool`, but with an explicit `tool_input` string — for tests
    /// that need `action_line::render_action_line` to actually extract an
    /// object (finding 2, items 5(a)-(e)/(g)/(h)/(i)).
    fn pre_tool_with_input(
        session: &str,
        cwd: &str,
        tool_name: &str,
        tool_input: &str,
        agent_id: Option<&str>,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::PreToolUse {
                tool_name: tool_name.to_string(),
                tool_use_id: "call-1".to_string(),
                tool_input: tool_input.to_string(),
                agent_id: agent_id.map(str::to_string),
                agent_type: None,
            },
            session,
            cwd,
            received_at,
        )
    }

    fn stop(session: &str, cwd: &str, agent_id: Option<&str>, received_at: u64) -> ClaudeIpcEnvelope {
        stop_with_message(session, cwd, "done", agent_id, received_at)
    }

    fn stop_with_message(
        session: &str,
        cwd: &str,
        last_assistant_message: &str,
        agent_id: Option<&str>,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::Stop {
                last_assistant_message: last_assistant_message.to_string(),
                agent_id: agent_id.map(str::to_string),
                agent_type: None,
            },
            session,
            cwd,
            received_at,
        )
    }

    fn stop_failure(session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::StopFailure { error_type: None },
            session,
            cwd,
            received_at,
        )
    }

    fn permission_request(
        session: &str,
        cwd: &str,
        tool_use_id: &str,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::PermissionRequest {
                tool_name: "Bash".to_string(),
                tool_use_id: tool_use_id.to_string(),
                tool_input: "{}".to_string(),
            },
            session,
            cwd,
            received_at,
        )
    }

    /// Like `permission_request`, but with an explicit `tool_name`/
    /// `tool_input` — for the finding-4 test that checks the synthesized
    /// `"allow: <command>"` text (item 10(b)).
    fn permission_request_with_input(
        session: &str,
        cwd: &str,
        tool_name: &str,
        tool_input: &str,
        tool_use_id: &str,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::PermissionRequest {
                tool_name: tool_name.to_string(),
                tool_use_id: tool_use_id.to_string(),
                tool_input: tool_input.to_string(),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn permission_denied(
        session: &str,
        cwd: &str,
        tool_use_id: &str,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::PermissionDenied {
                tool_name: "Bash".to_string(),
                tool_use_id: tool_use_id.to_string(),
                denial_reason: Some("policy forbids it".to_string()),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn post_tool(
        session: &str,
        cwd: &str,
        tool_name: &str,
        tool_use_id: &str,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::PostToolUse {
                tool_name: tool_name.to_string(),
                tool_use_id: tool_use_id.to_string(),
                tool_input: "{}".to_string(),
                tool_response: "ok".to_string(),
                agent_id: None,
                agent_type: None,
            },
            session,
            cwd,
            received_at,
        )
    }

    fn elicitation(
        session: &str,
        cwd: &str,
        tool_use_id: &str,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::Elicitation {
                tool_use_id: tool_use_id.to_string(),
                server_name: "my-mcp-server".to_string(),
                elicitation_request: "confirm deletion?".to_string(),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn elicitation_result(
        session: &str,
        cwd: &str,
        tool_use_id: &str,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::ElicitationResult {
                tool_use_id: tool_use_id.to_string(),
                server_name: "my-mcp-server".to_string(),
                user_response: "yes, delete it".to_string(),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn notification(session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        notification_typed(session, cwd, None, received_at)
    }

    fn notification_typed(
        session: &str,
        cwd: &str,
        notification_type: Option<&str>,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::Notification {
                notification_type: notification_type.map(str::to_string),
                notification_message: "idle".to_string(),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn subagent_start(
        session: &str,
        cwd: &str,
        agent_id: &str,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::SubagentStart {
                agent_id: agent_id.to_string(),
                agent_type: Some("general-purpose".to_string()),
                agent_prompt: "investigate the flaky test".to_string(),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn subagent_stop(
        session: &str,
        cwd: &str,
        agent_id: &str,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::SubagentStop {
                agent_id: agent_id.to_string(),
                agent_type: None,
                last_assistant_message: "found it".to_string(),
                stop_reason: Some("completed".to_string()),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn end(session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::SessionEnd {
                reason: Some(SessionEndReason::Other),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn ts(millis: u64) -> Timestamp {
        Timestamp::from_epoch_millis(millis as i64)
    }

    fn single_snapshot(events: Vec<SessionEvent>) -> SessionSnapshot {
        match events.as_slice() {
            [SessionEvent::Snapshot(snapshot)] => (**snapshot).clone(),
            other => panic!("expected one snapshot, got {other:?}"),
        }
    }

    fn assert_no_content(snapshot: &SessionSnapshot) {
        assert_eq!(snapshot.current_action, None);
        assert_eq!(snapshot.wire_title, None);
        assert_eq!(snapshot.final_assistant_text, None);
        assert_eq!(snapshot.last_user_prompt, None);
        assert!(snapshot.files_touched.is_empty());
        assert!(snapshot.recent_actions.is_empty());
    }

    // --- Top-level lifecycle mapping (SessionStart/StopFailure/SessionEnd,
    // unchanged from the original three-event contract) ---

    #[test]
    fn session_start_admits_idle_with_no_content() {
        let mut state = ClaudeState::new(IdentityResolver);
        let snapshot = single_snapshot(state.process(&start("sess-1", "/work/proj", R1)));

        assert_eq!(snapshot.session_id.harness, KIND);
        assert_eq!(snapshot.session_id.native_id, "sess-1");
        assert_eq!(snapshot.parent_id, None);
        assert_eq!(snapshot.project_id.as_path(), Path::new("/work/proj"));
        assert_eq!(
            snapshot.attention,
            AttentionState::Idle {
                last_update: ts(R1)
            }
        );
        assert_eq!(snapshot.created_at, ts(R1));
        assert_eq!(snapshot.last_updated, ts(R1));
        assert_no_content(&snapshot);
    }

    #[test]
    fn stop_failure_as_first_event_admits_with_its_receipt_time() {
        let mut state = ClaudeState::new(IdentityResolver);
        let snapshot = single_snapshot(state.process(&stop_failure("sess-1", "/work/proj", R2)));

        assert_eq!(snapshot.created_at, ts(R2), "first event pins created_at");
        assert_eq!(
            snapshot.attention,
            AttentionState::NeedsYou {
                question: false,
                turn_ended: ts(R2)
            }
        );

        let events = state.process(&end("sess-1", "/work/proj", R3));
        assert!(matches!(events.as_slice(), [SessionEvent::Gone(id)] if id.native_id == "sess-1"));
    }

    #[test]
    fn session_end_removes_state_and_emits_gone() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("sess-1", "/work/proj", R1));

        let events = state.process(&end("sess-1", "/work/proj", R3));
        assert!(matches!(events.as_slice(), [SessionEvent::Gone(id)] if id.native_id == "sess-1"));

        let fresh = single_snapshot(state.process(&start("sess-1", "/work/proj", R3)));
        assert_eq!(fresh.created_at, ts(R3), "re-admitted session restarts its clock");
    }

    #[test]
    fn session_end_for_a_never_seen_native_id_still_emits_gone() {
        let mut state = ClaudeState::new(IdentityResolver);
        let events = state.process(&end("never-seen", "/work/x", R3));
        assert!(matches!(events.as_slice(), [SessionEvent::Gone(id)] if id.native_id == "never-seen"));
    }

    // --- Activity mapping ---

    #[test]
    fn user_prompt_submit_starts_a_turn_and_records_the_prompt() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("sess-1", "/work/proj", R1));
        let snapshot = single_snapshot(state.process(&prompt(
            "sess-1",
            "/work/proj",
            "fix the bug",
            R2,
        )));
        assert_eq!(
            snapshot.attention,
            AttentionState::Running {
                turn_started: ts(R2)
            }
        );
        assert_eq!(snapshot.last_user_prompt.as_deref(), Some("fix the bug"));
    }

    #[test]
    fn tool_events_are_running_and_populate_current_action() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let snapshot = single_snapshot(state.process(&pre_tool_with_input(
            "sess-1",
            "/work/proj",
            "Edit",
            r#"{"file_path":"/work/proj/src/lib.rs"}"#,
            None,
            R2,
        )));
        assert_eq!(
            snapshot.attention,
            AttentionState::Running {
                turn_started: ts(R1)
            },
            "tool event reuses the turn's own start, not its own receipt"
        );
        assert_eq!(snapshot.current_action.as_deref(), Some("editing: lib.rs"));
        assert!(
            snapshot.recent_actions.is_empty(),
            "the first tool call of a turn has no previous current_action to push"
        );
    }

    // --- Finding 2, item 5(h)/5(i): the double-count bug and PostToolUse's
    // non-effect on current_action/recent_actions ---

    #[test]
    fn two_consecutive_pre_tool_use_calls_ring_gains_only_the_first_line() {
        // Item 5(h): the ring gains exactly one entry (the first call's
        // line) and current_action reads the second call's line — proves
        // the double-count bug (pushing the bare tool name unconditionally
        // in addition to setting current_action) is fixed.
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let first = single_snapshot(state.process(&pre_tool_with_input(
            "sess-1",
            "/work/proj",
            "Edit",
            r#"{"file_path":"/work/proj/src/lib.rs"}"#,
            None,
            R2,
        )));
        assert_eq!(first.current_action.as_deref(), Some("editing: lib.rs"));
        assert!(first.recent_actions.is_empty());

        let second = single_snapshot(state.process(&pre_tool_with_input(
            "sess-1",
            "/work/proj",
            "Bash",
            r#"{"command":"cargo test"}"#,
            None,
            R3,
        )));
        assert_eq!(
            second.current_action.as_deref(),
            Some("running: cargo test")
        );
        assert_eq!(
            second.recent_actions,
            vec!["editing: lib.rs".to_string()],
            "the ring holds only the first call's line — never a duplicate \
             of the second call's own current_action"
        );
    }

    #[test]
    fn post_tool_use_following_pre_tool_use_leaves_current_and_recent_actions_untouched() {
        // Item 5(i): PostToolUse for the same call must not change
        // current_action or recent_actions at all — it only runs the
        // shared clear_pending_tool_use/attention-Running/routing logic.
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let after_pre = single_snapshot(state.process(&pre_tool_with_input(
            "sess-1",
            "/work/proj",
            "Edit",
            r#"{"file_path":"/work/proj/src/lib.rs"}"#,
            None,
            R2,
        )));

        let after_post = single_snapshot(state.process(&post_tool(
            "sess-1",
            "/work/proj",
            "Edit",
            "call-1",
            R3,
        )));
        assert_eq!(
            after_post.current_action, after_pre.current_action,
            "PostToolUse must not change current_action"
        );
        assert_eq!(
            after_post.recent_actions, after_pre.recent_actions,
            "PostToolUse must not change recent_actions"
        );
    }

    #[test]
    fn tool_event_without_a_prior_turn_falls_back_to_its_own_receipt() {
        let mut state = ClaudeState::new(IdentityResolver);
        let snapshot = single_snapshot(state.process(&pre_tool(
            "sess-1",
            "/work/proj",
            "Edit",
            None,
            R1,
        )));
        assert_eq!(
            snapshot.attention,
            AttentionState::Running {
                turn_started: ts(R1)
            }
        );
    }

    #[test]
    fn recent_actions_ring_is_bounded_at_five_and_never_contains_current_action() {
        // Nine PreToolUse calls (tool-0..tool-8), each an unknown tool name
        // so every call renders "running: tool-N". Under the fixed
        // semantics, the ring holds only *previous* current_action values
        // (never the current one — snapshot.rs's own invariant) and stays
        // capped at 5.
        let mut state = ClaudeState::new(IdentityResolver);
        for (i, r) in (0..8).zip(R1..) {
            state.process(&pre_tool(
                "sess-1",
                "/work/proj",
                &format!("tool-{i}"),
                None,
                r,
            ));
        }
        let snapshot = single_snapshot(state.process(&pre_tool(
            "sess-1",
            "/work/proj",
            "tool-8",
            None,
            R1 + 8,
        )));
        assert_eq!(snapshot.current_action.as_deref(), Some("running: tool-8"));
        assert_eq!(snapshot.recent_actions.len(), 5);
        assert_eq!(
            snapshot.recent_actions,
            vec![
                "running: tool-3".to_string(),
                "running: tool-4".to_string(),
                "running: tool-5".to_string(),
                "running: tool-6".to_string(),
                "running: tool-7".to_string(),
            ],
            "the ring never includes tool-8, the current current_action"
        );
    }

    #[test]
    fn permission_request_is_needs_you_question() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let snapshot = single_snapshot(state.process(&permission_request(
            "sess-1",
            "/work/proj",
            "call-perm",
            R2,
        )));
        assert_eq!(
            snapshot.attention,
            AttentionState::NeedsYou {
                question: true,
                turn_ended: ts(R2)
            }
        );
    }

    // --- R14 (advisor review round 2): generic tool_use_id exit-path ---

    #[test]
    fn approval_path_permission_request_then_post_tool_use_clears_to_running() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let waiting = single_snapshot(state.process(&permission_request(
            "sess-1",
            "/work/proj",
            "call-approve",
            R2,
        )));
        assert_eq!(
            waiting.attention,
            AttentionState::NeedsYou {
                question: true,
                turn_ended: ts(R2)
            }
        );
        assert_eq!(
            waiting.final_assistant_text.as_deref(),
            Some("allow: Bash"),
            "the fixture's tool_input (\"{{}}\") has no command field, so \
             this falls back to the bare tool name (item 7's fallback rule)"
        );

        // The same tool_use_id's PostToolUse (the tool actually running)
        // clears the pending permission and carries its own effect in the
        // same step — never stuck NeedsYou. Item 4: PostToolUse never sets
        // current_action, so it stays None here (no PreToolUse ever ran).
        let resumed = single_snapshot(state.process(&post_tool(
            "sess-1",
            "/work/proj",
            "Bash",
            "call-approve",
            R3,
        )));
        assert_eq!(
            resumed.attention,
            AttentionState::Running {
                turn_started: ts(R1)
            },
            "clearing reuses the turn's own start, not the clearing event's receipt"
        );
        assert_eq!(
            resumed.current_action, None,
            "PostToolUse never sets current_action (item 4)"
        );
        assert_eq!(
            resumed.final_assistant_text, None,
            "the approval path also clears the stale \"allow: X\" text \
             (item 9's rationale applies here too, not just \
             PermissionDenied/ElicitationResult — clear_pending_tool_use \
             clears it on any matching tool_use_id, whichever event carries \
             it)"
        );
    }

    #[test]
    fn subagent_tool_call_clears_a_pending_permission_on_the_top_level_session() {
        // Regression: PermissionRequest/Elicitation carry no agent_id, so
        // pending_tool_use_id always lands on the TOP-level record — but a
        // tool event carrying agent_id routes to a *subagent* record. A
        // subagent's own permission-gated tool call must still clear the
        // top-level session's pending flag, or it gets stuck in NeedsYou
        // forever (the bug this test guards against).
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("top", "/work/proj", "go", R1));
        state.process(&subagent_start("top", "/work/proj", "agent-1", R1));
        let waiting = single_snapshot(state.process(&permission_request(
            "top",
            "/work/proj",
            "call-1",
            R2,
        )));
        assert_eq!(
            waiting.attention,
            AttentionState::NeedsYou {
                question: true,
                turn_ended: ts(R2)
            }
        );

        // The subagent's own tool call (same tool_use_id — `pre_tool`'s
        // fixture hardcodes "call-1") routes to the SUBAGENT record, not
        // the top-level one that actually holds the pending flag.
        let subagent_snapshot = single_snapshot(state.process(&pre_tool(
            "top",
            "/work/proj",
            "Edit",
            Some("agent-1"),
            R3,
        )));
        assert_eq!(subagent_snapshot.session_id.native_id, "top:agent-1");
        assert_eq!(
            subagent_snapshot.attention,
            AttentionState::Running {
                turn_started: ts(R1)
            }
        );

        // The TOP-level session's own pending permission must have cleared
        // too. Observed via a Notification — which never changes attention
        // on its own — so the value below can only come from the clear
        // already applied by the subagent tool event above, not from this
        // event's own mapping.
        let top_after = single_snapshot(state.process(&notification("top", "/work/proj", R4)));
        assert_eq!(
            top_after.attention,
            AttentionState::Running {
                turn_started: ts(R1)
            },
            "top-level session must not be stuck in NeedsYou after a \
             subagent's permission-gated tool call"
        );
    }

    #[test]
    fn denial_path_permission_request_then_permission_denied_clears_to_running() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        state.process(&permission_request("sess-1", "/work/proj", "call-deny", R2));

        let cleared = single_snapshot(state.process(&permission_denied(
            "sess-1",
            "/work/proj",
            "call-deny",
            R3,
        )));
        assert_eq!(
            cleared.attention,
            AttentionState::Running {
                turn_started: ts(R1)
            },
            "denial clears the pending permission back to Running, never stuck NeedsYou"
        );
    }

    #[test]
    fn mismatched_tool_use_id_does_not_clear_the_pending_permission() {
        // PermissionDenied's only defined effect is the generic clear, so a
        // mismatched id must leave attention untouched — a clean way to
        // prove the wrong id has no effect at all, independent of any other
        // event kind's own attention mapping.
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        state.process(&permission_request(
            "sess-1",
            "/work/proj",
            "call-real",
            R2,
        ));

        let still_waiting = single_snapshot(state.process(&permission_denied(
            "sess-1",
            "/work/proj",
            "call-other",
            R3,
        )));
        assert_eq!(
            still_waiting.attention,
            AttentionState::NeedsYou {
                question: true,
                turn_ended: ts(R2)
            },
            "a denial for an unrelated tool_use_id must not clear this pending permission"
        );

        // The real matching id still clears afterward.
        let cleared = single_snapshot(state.process(&permission_denied(
            "sess-1",
            "/work/proj",
            "call-real",
            R4,
        )));
        assert_eq!(
            cleared.attention,
            AttentionState::Running {
                turn_started: ts(R1)
            }
        );
    }

    #[test]
    fn elicitation_then_elicitation_result_clears_to_running() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let waiting = single_snapshot(state.process(&elicitation(
            "sess-1",
            "/work/proj",
            "call-elicit",
            R2,
        )));
        assert_eq!(
            waiting.attention,
            AttentionState::NeedsYou {
                question: true,
                turn_ended: ts(R2)
            }
        );

        let cleared = single_snapshot(state.process(&elicitation_result(
            "sess-1",
            "/work/proj",
            "call-elicit",
            R3,
        )));
        assert_eq!(
            cleared.attention,
            AttentionState::Running {
                turn_started: ts(R1)
            }
        );
    }

    // --- Finding 4 (T02): Question tile content correctness, items 10(a)-(e) ---

    #[test]
    fn user_prompt_submit_clears_a_prior_final_assistant_text() {
        // Item 10(a): a new turn's stale "previous answer" must not persist
        // into whatever NeedsYou state this new turn eventually reaches.
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let stopped = single_snapshot(state.process(&stop_with_message(
            "sess-1",
            "/work/proj",
            "Which file would you like me to delete?",
            None,
            R2,
        )));
        assert_eq!(
            stopped.final_assistant_text.as_deref(),
            Some("Which file would you like me to delete?")
        );

        let next_turn = single_snapshot(state.process(&prompt(
            "sess-1",
            "/work/proj",
            "delete a.txt",
            R3,
        )));
        assert_eq!(
            next_turn.final_assistant_text, None,
            "the previous turn's answer must not survive into the new turn"
        );
    }

    #[test]
    fn permission_request_sets_final_assistant_text_to_allow_command() {
        // Item 10(b).
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let waiting = single_snapshot(state.process(&permission_request_with_input(
            "sess-1",
            "/work/proj",
            "Bash",
            r#"{"command":"rm -rf build"}"#,
            "call-1",
            R2,
        )));
        assert_eq!(
            waiting.final_assistant_text.as_deref(),
            Some("allow: rm -rf build")
        );
    }

    #[test]
    fn elicitation_sets_final_assistant_text_to_the_raw_request_text() {
        // Item 10(c): already natural-language text, no extraction.
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let waiting = single_snapshot(state.process(&elicitation(
            "sess-1",
            "/work/proj",
            "call-elicit",
            R2,
        )));
        assert_eq!(waiting.final_assistant_text.as_deref(), Some("confirm deletion?"));
    }

    #[test]
    fn permission_denied_clears_final_assistant_text() {
        // Item 10(d).
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        state.process(&permission_request_with_input(
            "sess-1",
            "/work/proj",
            "Bash",
            r#"{"command":"rm -rf build"}"#,
            "call-deny",
            R2,
        ));
        let cleared = single_snapshot(state.process(&permission_denied(
            "sess-1",
            "/work/proj",
            "call-deny",
            R3,
        )));
        assert_eq!(cleared.final_assistant_text, None);
    }

    #[test]
    fn elicitation_result_clears_final_assistant_text() {
        // Item 10(e).
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        state.process(&elicitation("sess-1", "/work/proj", "call-elicit", R2));
        let cleared = single_snapshot(state.process(&elicitation_result(
            "sess-1",
            "/work/proj",
            "call-elicit",
            R3,
        )));
        assert_eq!(cleared.final_assistant_text, None);
    }

    #[test]
    fn mismatched_permission_denied_does_not_clear_an_unrelated_pending_question() {
        // Guards the conditional design of clear_pending_tool_use's new
        // final_assistant_text clear: an unrelated denial (wrong
        // tool_use_id) must not wipe a *different*, still-genuinely-pending
        // permission's question text.
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        state.process(&permission_request_with_input(
            "sess-1",
            "/work/proj",
            "Bash",
            r#"{"command":"rm -rf build"}"#,
            "call-real",
            R2,
        ));

        let still_waiting = single_snapshot(state.process(&permission_denied(
            "sess-1",
            "/work/proj",
            "call-other",
            R3,
        )));
        assert_eq!(
            still_waiting.final_assistant_text.as_deref(),
            Some("allow: rm -rf build"),
            "a denial for an unrelated tool_use_id must not clear this \
             session's actually-pending question text"
        );
    }

    #[test]
    fn notification_only_refreshes_last_updated() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let before = single_snapshot(state.process(&pre_tool(
            "sess-1",
            "/work/proj",
            "Edit",
            None,
            R2,
        )));
        let after = single_snapshot(state.process(&notification("sess-1", "/work/proj", R3)));
        assert_eq!(after.attention, before.attention, "attention is untouched");
        assert_eq!(after.current_action, before.current_action);
        assert_eq!(after.last_updated, ts(R3), "only last_updated advances");
    }

    // --- Item 1: Notification's notification_type drives attention ---

    #[test]
    fn notification_idle_prompt_needs_you_and_clears_turn_started() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        state.process(&pre_tool("sess-1", "/work/proj", "Edit", None, R2));

        let notified = single_snapshot(state.process(&notification_typed(
            "sess-1",
            "/work/proj",
            Some("idle_prompt"),
            R3,
        )));
        assert_eq!(
            notified.attention,
            AttentionState::NeedsYou {
                question: false,
                turn_ended: ts(R3)
            }
        );

        // turn_started was cleared: a later tool event (no new
        // UserPromptSubmit) falls back to its own receipt, exactly like
        // Stop's own bookkeeping.
        let resumed = single_snapshot(state.process(&pre_tool(
            "sess-1",
            "/work/proj",
            "Bash",
            None,
            R4,
        )));
        assert_eq!(
            resumed.attention,
            AttentionState::Running {
                turn_started: ts(R4)
            },
            "idle_prompt clears turn_started so a later tool event doesn't \
             resurrect the stale start time"
        );
    }

    #[test]
    fn notification_permission_prompt_and_agent_needs_input_are_question_and_keep_turn_started() {
        for sub_type in ["permission_prompt", "agent_needs_input"] {
            let mut state = ClaudeState::new(IdentityResolver);
            state.process(&prompt("sess-1", "/work/proj", "go", R1));
            state.process(&pre_tool("sess-1", "/work/proj", "Edit", None, R2));

            let notified = single_snapshot(state.process(&notification_typed(
                "sess-1",
                "/work/proj",
                Some(sub_type),
                R3,
            )));
            assert_eq!(
                notified.attention,
                AttentionState::NeedsYou {
                    question: true,
                    turn_ended: ts(R3)
                },
                "notification_type {sub_type:?} must set question: true"
            );

            // Unlike idle_prompt, turn_started is untouched — the turn may
            // still be running with a subprocess waiting on the user. A
            // later tool event reuses the original turn start (R1), not
            // its own receipt.
            let resumed = single_snapshot(state.process(&pre_tool(
                "sess-1",
                "/work/proj",
                "Bash",
                None,
                R4,
            )));
            assert_eq!(
                resumed.attention,
                AttentionState::Running {
                    turn_started: ts(R1)
                },
                "notification_type {sub_type:?} must not clear turn_started"
            );
        }
    }

    #[test]
    fn notification_unrecognized_type_leaves_attention_unchanged() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let before = single_snapshot(state.process(&pre_tool(
            "sess-1",
            "/work/proj",
            "Edit",
            None,
            R2,
        )));

        let after = single_snapshot(state.process(&notification_typed(
            "sess-1",
            "/work/proj",
            Some("some_future_sub_type"),
            R3,
        )));
        assert_eq!(
            after.attention, before.attention,
            "an unrecognized notification_type changes nothing but last_updated"
        );
        assert_eq!(after.last_updated, ts(R3));
    }

    #[test]
    fn stop_ends_the_turn_and_clears_turn_started() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        state.process(&pre_tool("sess-1", "/work/proj", "Edit", None, R2));
        let stopped = single_snapshot(state.process(&stop("sess-1", "/work/proj", None, R3)));
        assert_eq!(
            stopped.attention,
            AttentionState::NeedsYou {
                question: false,
                turn_ended: ts(R3)
            }
        );
        assert_eq!(stopped.final_assistant_text.as_deref(), Some("done"));

        // turn_started was cleared: a later tool event (defensive, no new
        // UserPromptSubmit) falls back to its own receipt again.
        let resumed = single_snapshot(state.process(&pre_tool(
            "sess-1",
            "/work/proj",
            "Bash",
            None,
            R4,
        )));
        assert_eq!(
            resumed.attention,
            AttentionState::Running {
                turn_started: ts(R4)
            }
        );
    }

    // --- Item 3: top-level Stop reuses the shared question heuristic ---

    #[test]
    fn top_level_stop_ending_in_a_question_sets_question_true() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&prompt("sess-1", "/work/proj", "go", R1));
        let stopped = single_snapshot(state.process(&stop_with_message(
            "sess-1",
            "/work/proj",
            "Which file would you like me to delete?",
            None,
            R2,
        )));
        assert_eq!(
            stopped.attention,
            AttentionState::NeedsYou {
                question: true,
                turn_ended: ts(R2)
            },
            "a top-level Stop whose text looks like a question sets question: true"
        );
    }

    // --- Subagent modeling ---

    #[test]
    fn subagent_start_creates_a_distinct_child_session() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("top", "/work/proj", R1));
        let snapshot = single_snapshot(state.process(&subagent_start(
            "top",
            "/work/proj",
            "agent-1",
            R2,
        )));

        assert_eq!(snapshot.session_id.harness, KIND);
        assert_eq!(snapshot.session_id.native_id, "top:agent-1");
        assert_eq!(
            snapshot.parent_id,
            Some(SessionId::new(KIND, "top".to_string()))
        );
        assert_eq!(
            snapshot.attention,
            AttentionState::Running {
                turn_started: ts(R2)
            }
        );
        assert_eq!(
            snapshot.last_user_prompt.as_deref(),
            Some("investigate the flaky test")
        );
        assert_eq!(
            snapshot.project_id.as_path(),
            Path::new("/work/proj"),
            "subagent shares the already-tracked parent's project identity"
        );
    }

    #[test]
    fn subagent_tool_events_target_the_child_not_the_parent() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("top", "/work/proj", R1));
        state.process(&subagent_start("top", "/work/proj", "agent-1", R2));
        let snapshot = single_snapshot(state.process(&pre_tool(
            "top",
            "/work/proj",
            "Edit",
            Some("agent-1"),
            R3,
        )));

        assert_eq!(snapshot.session_id.native_id, "top:agent-1");
        assert_eq!(
            snapshot.current_action.as_deref(),
            Some("running: Edit"),
            "the fixture's tool_input (\"{{}}\") has no file_path field, so \
             this falls back to the bare tool name"
        );
        // The parent (top-level) session is untouched by the subagent's tool
        // event: a subsequent SessionEnd for it still cleanly tombstones,
        // independent of subagent state.
        let gone = state.process(&end("top", "/work/proj", R4));
        assert!(matches!(gone.as_slice(), [SessionEvent::Gone(id)] if id.native_id == "top"));
    }

    #[test]
    fn subagent_created_defensively_before_subagent_start_is_observed() {
        // Event ordering isn't guaranteed (R17): a tool event with agent_id
        // can arrive before any SubagentStart for that agent_id.
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("top", "/work/proj", R1));
        let snapshot = single_snapshot(state.process(&pre_tool(
            "top",
            "/work/proj",
            "Bash",
            Some("agent-2"),
            R2,
        )));
        assert_eq!(snapshot.session_id.native_id, "top:agent-2");
        assert_eq!(
            snapshot.parent_id,
            Some(SessionId::new(KIND, "top".to_string()))
        );
        assert_eq!(snapshot.created_at, ts(R2));
    }

    #[test]
    fn subagent_stop_tombstones_the_child_and_leaves_the_parent_tracked() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("top", "/work/proj", R1));
        state.process(&subagent_start("top", "/work/proj", "agent-1", R2));

        let events = state.process(&subagent_stop("top", "/work/proj", "agent-1", R3));
        assert_eq!(
            events,
            vec![SessionEvent::Gone(SessionId::new(
                KIND,
                "top:agent-1".to_string()
            ))]
        );

        // The parent is still tracked and its own SessionEnd still tombstones.
        let gone = state.process(&end("top", "/work/proj", R4));
        assert!(matches!(gone.as_slice(), [SessionEvent::Gone(id)] if id.native_id == "top"));
    }

    #[test]
    fn subagent_stop_for_a_never_seen_agent_id_still_emits_gone() {
        let mut state = ClaudeState::new(IdentityResolver);
        let events = state.process(&subagent_stop("top", "/work/proj", "ghost", R1));
        assert_eq!(
            events,
            vec![SessionEvent::Gone(SessionId::new(
                KIND,
                "top:ghost".to_string()
            ))]
        );
    }

    #[test]
    fn stop_with_agent_id_targets_the_subagent() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("top", "/work/proj", R1));
        state.process(&subagent_start("top", "/work/proj", "agent-1", R2));
        let snapshot = single_snapshot(state.process(&stop(
            "top",
            "/work/proj",
            Some("agent-1"),
            R3,
        )));
        assert_eq!(snapshot.session_id.native_id, "top:agent-1");
        assert_eq!(
            snapshot.attention,
            AttentionState::Idle {
                last_update: ts(R3)
            },
            "a finished subagent reads as done, not as blocking (item 2)"
        );
        assert_eq!(snapshot.final_assistant_text.as_deref(), Some("done"));
    }

    #[test]
    fn subagent_stop_ending_in_a_question_still_goes_idle_not_needs_you() {
        // Item 3's question heuristic must not leak into item 2's path: a
        // subagent Stop always settles to Idle, even when its text would
        // trip looks_like_question for a top-level Stop.
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("top", "/work/proj", R1));
        state.process(&subagent_start("top", "/work/proj", "agent-1", R2));
        let snapshot = single_snapshot(state.process(&stop_with_message(
            "top",
            "/work/proj",
            "Which approach should I take?",
            Some("agent-1"),
            R3,
        )));
        assert_eq!(snapshot.session_id.native_id, "top:agent-1");
        assert_eq!(
            snapshot.attention,
            AttentionState::Idle {
                last_update: ts(R3)
            },
            "a subagent Stop is Idle regardless of whether its text looks \
             like a question"
        );
    }

    #[test]
    fn duplicate_subagent_start_preserves_creation_time() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("top", "/work/proj", R1));
        state.process(&subagent_start("top", "/work/proj", "agent-1", R2));
        let second = single_snapshot(state.process(&subagent_start(
            "top",
            "/work/proj",
            "agent-1",
            R3,
        )));
        assert_eq!(second.created_at, ts(R2));
        assert_eq!(second.last_updated, ts(R3));
    }

    #[test]
    fn same_agent_id_under_different_top_sessions_is_distinct() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("top-a", "/work/a", R1));
        state.process(&start("top-b", "/work/b", R1));
        state.process(&subagent_start("top-a", "/work/a", "agent-1", R2));
        state.process(&subagent_start("top-b", "/work/b", "agent-1", R2));

        let gone_a = state.process(&subagent_stop("top-a", "/work/a", "agent-1", R3));
        assert_eq!(
            gone_a,
            vec![SessionEvent::Gone(SessionId::new(
                KIND,
                "top-a:agent-1".to_string()
            ))]
        );
        // top-b's identically-named subagent is unaffected.
        let gone_b = state.process(&subagent_stop("top-b", "/work/b", "agent-1", R3));
        assert_eq!(
            gone_b,
            vec![SessionEvent::Gone(SessionId::new(
                KIND,
                "top-b:agent-1".to_string()
            ))]
        );
    }

    #[test]
    fn project_resolution_failure_degrades_one_snapshot_and_continues() {
        let missing = PathBuf::from("/nowhere/project");
        let mut state = ClaudeState::new(SelectiveResolver {
            failing: missing.clone(),
        });

        let degraded = single_snapshot(state.process(&start("bad-cwd", "/nowhere/project", R1)));
        assert_eq!(
            degraded.project_id,
            ProjectId::from_uncanonicalized(&missing),
            "degraded project identity is the raw cwd, uncanonicalized"
        );

        let ok = single_snapshot(state.process(&start("good-cwd", "/work/proj", R2)));
        assert_eq!(ok.project_id.as_path(), Path::new("/work/proj"));
    }

    #[test]
    fn sessions_are_keyed_under_the_claude_harness_kind() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("123", "/work/proj", R1));
        let tracked: Vec<&SessionId> = state.sessions.keys().collect();
        assert_eq!(tracked.len(), 1);
        assert_eq!(tracked[0].harness, KIND);
        assert_eq!(tracked[0].native_id, "123");
        assert_ne!(
            tracked[0].clone(),
            SessionId::new(crate::snapshot::HarnessKind("opencode"), "123")
        );
    }
}
