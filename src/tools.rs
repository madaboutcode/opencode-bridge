//! The 4 MCP tools (SPEC.md §5). `definitions()` feeds `tools/list`;
//! `call()` is the dispatch table for `tools/call`. Each handler validates
//! its own arguments (boundary) and otherwise just composes `opencode.rs`
//! + `registry.rs` calls — no opencode wire-format knowledge lives here.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::error::Result;
use crate::opencode::ModelRef;
use crate::registry::Status;
use crate::state::AppState;

/// Bridge-side cap on how long `wait=true` blocks (SPEC.md §7.5). CC
/// enforces its own MCP tool-call timeout; if we blocked past it, CC kills
/// the call and the underlying opencode turn keeps running invisibly.
/// Capping here lets us hand back a "still running" reply instead. This is
/// an application-level deadline via `tokio::time::timeout` — it doesn't
/// touch reqwest's per-request timeout, which is simply never set (see
/// `opencode::Client::new`), so `/wait` itself is never killed by the HTTP
/// layer, only raced against this deadline.
const WAIT_CAP: Duration = Duration::from_secs(240);

pub fn definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "opencode_task",
            "description": "Dispatch a prompt to opencode2 — the one entry point for handing off work. Omit session_id to START a new session; pass session_id to CONTINUE (follow up on) an existing one. PREFER THE ASYNC DEFAULT: it returns {session_id, status:\"running\"} immediately and pushes the result back into this Claude Code session when the turn finishes (like spawning a background agent). You stay free to do other work meanwhile instead of blocking on a slower/cheaper model — and you're notified the moment it's done. Treat wait=true as the rare exception, only when you genuinely cannot proceed without the output in this same turn. Note: the notification claim is per-session, not per-turn; interleaving wait=true with async followups on the same session is not supported (use a fresh session for parallel branches).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "prompt": {"type": "string", "description": "The prompt to send."},
                    "session_id": {"type": "string", "description": "Continue this existing session. Omit to start a new one."},
                    "wait": {"type": "boolean", "description": "Default false (async) — RECOMMENDED: fire the task and get a callback when it finishes, so you don't sit blocked on it. Only set true when you truly need the output inline in this same turn; it blocks up to 240s, then falls back to the async callback anyway if still running. Prefer leaving this off."},
                    "notify": {"type": "boolean", "description": "Push a completion message back into this CC session when the async turn finishes. Default true."},
                    "model": {"type": "string", "description": "NEW SESSIONS ONLY. \"providerID/modelID\", e.g. \"opencode-go/ox-alpha-free\". Omit for the server default. Ignored when continuing (session_id set)."},
                    "agent": {"type": "string", "description": "NEW SESSIONS ONLY. opencode agent name, e.g. \"coder\" (see opencode_catalog kind=agents). Ignored when continuing."},
                    "directory": {"type": "string", "description": "NEW SESSIONS ONLY. Absolute path the agent's tools (edit/read/bash) operate in — the project to touch. Omit to use the directory this bridge launched from (usually the current project). Ignored when continuing (the session keeps its original directory)."},
                    "title": {"type": "string", "description": "NEW SESSIONS ONLY. Human-readable session title. Ignored when continuing."},
                    "delivery": {"type": "string", "enum": ["queue", "steer"], "description": "CONTINUING ONLY. \"queue\" (default) waits for any in-flight turn to finish; \"steer\" interrupts it. Ignored when starting a new session."}
                },
                "required": ["prompt"]
            }
        }),
        json!({
            "name": "opencode_sessions",
            "description": "Inspect opencode2 sessions. Pass session_id to get ONE session's detail (outcome, running, cost, tokens, and the final assistant output text of its last turn) — this works for ANY session id, including ones started elsewhere. Omit session_id to LIST this CC session's own sessions (as `sessions`, newest first) — scoped to work you launched or followed up on here, and still correct across an MCP restart. Pass include_all=true to also dump unrelated server sessions as `other_sessions` (debug).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "The session to detail. Omit to list this bridge's own sessions."},
                    "include_all": {"type": "boolean", "description": "List mode only (no session_id): also include unrelated sessions on the shared server — the TUI, other tools, other CC sessions. Default false. Rarely needed; mainly to grab the id of a session started elsewhere so you can continue it."}
                }
            }
        }),
        json!({
            "name": "opencode_cancel",
            "description": "Interrupt a running opencode2 session's current turn.",
            "inputSchema": {
                "type": "object",
                "properties": {"session_id": {"type": "string"}},
                "required": ["session_id"]
            }
        }),
        json!({
            "name": "opencode_catalog",
            "description": "Look up what's available to run tasks with: the opencode2 server's models or agents. Set kind=\"models\" or kind=\"agents\". Pass query to filter (case-insensitive substring, space-separated terms ANDed) — the server has hundreds of models, so a query is usually what you want. Model results are capped (see `truncated`); agent results exclude hidden internal agents unless include_hidden=true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": {"type": "string", "enum": ["models", "agents"], "description": "\"models\" = model catalog (search over providerID/id/name). \"agents\" = agent tags you pass as opencode_task's `agent` (search over name/description)."},
                    "query": {"type": "string", "description": "Case-insensitive substring filter; space-separated terms are ANDed (all must match). Omit to list all."},
                    "include_hidden": {"type": "boolean", "description": "kind=agents only: include agents opencode marks hidden (internal helpers like Title/Summary). Default false."}
                },
                "required": ["kind"]
            }
        }),
    ]
}

/// Case-insensitive AND-of-terms substring match, mirroring the skill's
/// `opencode models | grep -i` habit but letting multiple words narrow the
/// result. An empty/whitespace-only query matches everything. `haystack` is
/// the pre-joined searchable text for one row (name + id + description etc).
fn matches_query(query: &str, haystack: &str) -> bool {
    let hay = haystack.to_lowercase();
    query
        .split_whitespace()
        .all(|term| hay.contains(&term.to_lowercase()))
}

pub async fn call(state: &AppState, name: &str, args: Value) -> Result<Value> {
    match name {
        // Broad tools (SPEC.md §5): each dispatches on whether an optional
        // discriminator is present, so 8 narrow verbs collapse to 4 without
        // losing schema clarity.
        "opencode_task" => task(state, args).await, // session_id absent = start, present = continue
        "opencode_sessions" => sessions(state, args).await, // session_id absent = list, present = detail
        "opencode_cancel" => cancel(state, args).await,
        "opencode_catalog" => catalog(state, args).await, // kind = models | agents
        other => Err(format!("unknown tool: {other}").into()),
    }
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing or non-string required argument: {key}").into())
}

fn opt_bool(args: &Value, key: &str, default: bool) -> Result<bool> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        Some(other) => Err(format!("{key} must be a boolean, got {other}").into()),
    }
}

#[allow(dead_code)]
fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

fn opt_str_strict(args: &Value, key: &str) -> Result<Option<String>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(format!("{key} must be a string, got {other}").into()),
    }
}

/// Accepts `model` as either a `"providerID/modelID"` string (split on the
/// first `/`) or a `{id, providerID}` object (SPEC.md §5).
fn parse_model(args: &Value, key: &str) -> Result<Option<ModelRef>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let (provider_id, id) = s
                .split_once('/')
                .ok_or_else(|| format!("model string must be \"providerID/modelID\", got {s:?}"))?;
            Ok(Some(ModelRef {
                id: id.to_string(),
                provider_id: provider_id.to_string(),
            }))
        }
        Some(v @ Value::Object(_)) => {
            let m: ModelRef = serde_json::from_value(v.clone())
                .map_err(|e| format!("invalid model object: {e}"))?;
            Ok(Some(m))
        }
        Some(other) => Err(format!("model must be a string or object, got {other}").into()),
    }
}

/// `opencode_task`: the single dispatch entry point. `session_id` present ⇒
/// continue an existing session (followup); absent ⇒ start a new one. Both
/// branches share the `wait`/`notify` async-vs-sync tail (`wait_and_finish`).
async fn task(state: &AppState, args: Value) -> Result<Value> {
    let prompt = require_str(&args, "prompt")?.to_string();
    let wait = opt_bool(&args, "wait", false)?;
    let notify = opt_bool(&args, "notify", true)?;

    // For wait=true we take the notification pre-claim BEFORE /prompt so
    // the SSE consumer can never observe a fast-fail terminal event before
    // we hold the slot (otherwise wait=true would both return inline AND
    // fire the async callback — a double report, SPEC §7.5). The claim is
    // an RAII guard: on any non-success exit Drop releases it (notified=
    // false, notify=true); commit() on the success tail keeps it. We
    // create the guard in task() and thread it into wait_and_finish().
    let (session_id, pre_claim) = match opt_str_strict(&args, "session_id")? {
        // CONTINUE: followup on an existing session.
        Some(session_id) => {
            // SPEC.md §5 DECISION: default delivery = queue (let the current
            // turn finish); "steer" interrupts it. Only meaningful here.
            let delivery =
                opt_str_strict(&args, "delivery")?.unwrap_or_else(|| "queue".to_string());
            if delivery != "queue" && delivery != "steer" {
                return Err(
                    format!("delivery must be \"queue\" or \"steer\", got {delivery:?}").into(),
                );
            }
            // Snapshot before mutating, so a failed /prompt can restore the
            // prior turn's state instead of leaving a phantom "running" entry
            // that a later sweep would mis-fire on with stale output.
            let prior_snapshot = if state.registry.is_tracked(&session_id) {
                state.registry.snapshot(&session_id)
            } else {
                None
            };
            let inserted_now = if state.registry.is_tracked(&session_id) {
                // Re-arm: a second followup must be able to notify again.
                state
                    .registry
                    .reset_for_followup(&session_id, prompt.clone(), notify);
                false
            } else {
                // Followup on a session we didn't launch — still register
                // BEFORE prompting so the SSE consumer can find it
                // (SPEC.md §5 race guard).
                state
                    .registry
                    .register(session_id.clone(), prompt.clone(), None, None, notify);
                true
            };
            // Pre-claim BEFORE /prompt (wait=true only) — closes the
            // /prompt → wait_and_finish race window described above.
            let pre_claim = if wait {
                state.registry.claim_notification_guard(&session_id)
            } else {
                None
            };
            if let Err(e) = state
                .client
                .prompt(&session_id, &prompt, Some(&delivery), &state.origin)
                .await
            {
                if inserted_now {
                    state.registry.unregister(&session_id);
                } else if let Some(prior) = prior_snapshot {
                    // Already-tracked session: /prompt failed, so the new
                    // turn never started — restore the prior turn's state
                    // so a later sweep doesn't see a phantom running turn
                    // and emit a stale callback with the prior output.
                    state.registry.restore(&session_id, prior);
                }
                // pre_claim (if any) drops here — on a restored entry this
                // sets notified=false/notify=true on the prior turn, which
                // is dormant (no new turn is running) and harmless; a
                // subsequent reset_for_followup will re-arm correctly.
                drop(pre_claim);
                return Err(e);
            }
            (session_id, pre_claim)
        }
        // START: create a new session, then prompt it. model/agent/directory/
        // title apply only on this branch.
        None => {
            let model = parse_model(&args, "model")?;
            let agent = opt_str_strict(&args, "agent")?;
            let title = opt_str_strict(&args, "title")?;
            // Explicit `directory` wins; else the bridge's startup cwd (the
            // project CC launched it from); else None → opencode's default.
            let directory =
                opt_str_strict(&args, "directory")?.or_else(|| state.default_dir.clone());

            // SPEC.md §8 Layer 2: tag every session we create with
            // "cc-bridge:<origin>:<slug>" so opencode_sessions can rediscover
            // our own sessions across a bridge restart. Slug source is the
            // user-supplied title if given, else the prompt.
            let slug_source = title.as_deref().unwrap_or(&prompt);
            let tagged_title = format!("cc-bridge:{}:{}", state.origin, slugify(slug_source, 40));

            let session_id = state
                .client
                .create_session(
                    model.as_ref(),
                    agent.as_deref(),
                    Some(&tagged_title),
                    directory.as_deref(),
                )
                .await?;
            // Register BEFORE prompting (SPEC.md §5/§8 race guard).
            state
                .registry
                .register(session_id.clone(), prompt.clone(), model, agent, notify);
            // Pre-claim BEFORE /prompt (wait=true only) — same race guard
            // as the CONTINUE branch above.
            let pre_claim = if wait {
                state.registry.claim_notification_guard(&session_id)
            } else {
                None
            };
            if let Err(e) = state
                .client
                .prompt(&session_id, &prompt, None, &state.origin)
                .await
            {
                // /session POST already created the session on opencode;
                // if /prompt then fails, the local registry entry points at
                // work we never started — unregister so the SSE consumer
                // doesn't fire a callback on its (orphan) terminal event.
                // The opencode-side session is left in place; deleting it
                // would need an undocumented endpoint. pre_claim drops
                // after unregister — entry is gone, so no-op.
                state.registry.unregister(&session_id);
                drop(pre_claim);
                return Err(e);
            }
            (session_id, pre_claim)
        }
    };

    if wait {
        wait_and_finish(state, &session_id, pre_claim).await
    } else {
        // wait=false: pre_claim is always None (we only take it for
        // wait=true), so no guard to drop. SSE will claim when the turn
        // goes terminal.
        debug_assert!(pre_claim.is_none());
        Ok(json!({"session_id": session_id, "status": "running"}))
    }
}

/// Shared sync-wait tail for `opencode_task`: block until idle
/// (capped at `WAIT_CAP`), fetch the output, record it, and return the
/// reply shape SPEC.md §5 documents for `wait=true`.
///
/// `pre_claim` is the RAII guard taken in `task()` BEFORE /prompt (see
/// that function's comment for why). It already holds the notification
/// slot for this turn, so the SSE consumer can never win a race against
/// us — commit() on the success tail is the only way to keep the claim
/// (no async dup); every other exit drops it (notified=false, notify=
/// true) so the eventual terminal event still notifies via SSE/sweep.
async fn wait_and_finish(
    state: &AppState,
    session_id: &str,
    pre_claim: Option<crate::registry::NotifyClaim<'_>>,
) -> Result<Value> {
    // Reuse the pre-claim from task(). In the wait=true path task()
    // always took one, so this is Some in practice; handle None anyway
    // since a None here isn't a state worth crashing over (e.g. the
    // session was somehow unregistered between task() and here).
    let claim = pre_claim;

    match tokio::time::timeout(WAIT_CAP, state.client.wait(session_id)).await {
        Ok(Ok(())) => {}             // finished within the cap — fall through
        Ok(Err(e)) => return Err(e), // /wait itself failed — claim released by Drop
        Err(_elapsed) => {
            // Hit the cap before the turn finished. Claim released by Drop
            // (forces notify=true) — the caller can no longer get the
            // result synchronously, so an async report is the only way
            // left. If the turn actually finished right around this
            // deadline, the periodic sweep (SPEC.md §7.3) is the backstop
            // that picks it up within ~60s even if this exact race is lost.
            return Ok(json!({
                "session_id": session_id,
                "status": "running",
                "note": format!("still running after {}s; will notify on completion", WAIT_CAP.as_secs()),
            }));
        }
    }

    let output = state.client.final_output(session_id).await?;
    let info = state.client.get_session(session_id).await?;
    let outcome = info
        .outcome
        .ok_or("get_session returned no outcome after /wait completed — API invariant violated")?;
    let status = Status::from_outcome(&outcome)?;
    state
        .registry
        .set_result(session_id, status, output.clone());
    if let Some(claim) = claim {
        claim.commit(); // reporting synchronously below — no async dup
    }
    Ok(json!({"session_id": session_id, "output": output, "outcome": outcome}))
}

/// Short, readable slug for a session title tag (SPEC.md §8): lowercase,
/// non-alphanumeric runs collapsed to '-', capped length. Cosmetic only —
/// never parsed back.
fn slugify(text: &str, max_len: usize) -> String {
    let mut slug = String::new();
    let mut last_was_dash = true; // suppress a leading dash
    for c in text.chars() {
        if slug.len() >= max_len {
            break;
        }
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_end_matches('-').to_string()
}

/// SystemTime -> epoch milliseconds for JSON output. FALLBACK-OK: a clock
/// before the UNIX epoch is not a state this bridge can meaningfully
/// recover from and `created` is purely informational (SPEC.md §8's
/// "mitigate cosmetically" timestamp), so 0 is a harmless display fallback
/// rather than a correctness-affecting one.
fn epoch_millis(t: SystemTime) -> u128 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// `opencode_sessions`: `session_id` present ⇒ detail one session (the old
/// status + result merged into a single reply — metadata AND the last
/// turn's output text); absent ⇒ list all sessions (the old list).
async fn sessions(state: &AppState, args: Value) -> Result<Value> {
    match opt_str_strict(&args, "session_id")? {
        Some(session_id) => detail_session(state, &session_id).await,
        None => list_sessions(state, opt_bool(&args, "include_all", false)?).await,
    }
}

async fn detail_session(state: &AppState, session_id: &str) -> Result<Value> {
    let info = state.client.get_session(session_id).await?;
    let output = state.client.final_output(session_id).await?;
    // SPEC.md §5: running = outcome absent OR time.idle older than time.updated
    // (idle absent counts as "older" — no idle timestamp recorded yet).
    let running = info.outcome.is_none()
        || info
            .time
            .idle
            .map(|idle| idle < info.time.updated)
            .unwrap_or(true);
    Ok(json!({
        "session_id": session_id,
        "outcome": info.outcome,
        "running": running,
        "idle": info.time.idle,
        "cost": info.cost,
        "tokens": info.tokens,
        "output": output,
    }))
}

/// Lists THIS CC session's opencode work as one flat `sessions` array,
/// newest first. Entries come from two places the caller shouldn't have to
/// care about: the live in-memory registry, and — after an MCP restart wiped
/// that registry — the server, re-matched by our durable origin tag (title
/// `cc-bridge:<origin>:…`; origin is the CC socket pid, stable for the life
/// of the CC session, so this is genuinely "ours", not server-wide noise).
/// Registry entries carry richer fields (model/agent/notify); re-found ones
/// only have what the server kept. Re-found entries are display/reattach
/// only — listing never registers or notifies (SPEC.md §8 invariant: a tag
/// match is a label, never a trigger); continue one via opencode_task to get
/// callbacks again. `include_all` additionally returns `other_sessions` —
/// every OTHER (foreign) server session — for debugging.
async fn list_sessions(state: &AppState, include_all: bool) -> Result<Value> {
    let tracked = state.registry.list(); // newest first
    let tracked_ids: std::collections::HashSet<&str> =
        tracked.iter().map(|(id, _)| id.as_str()).collect();

    // (created_ms, entry) so we can merge registry + server and sort as one.
    let mut sessions: Vec<(i128, Value)> = tracked
        .iter()
        .map(|(id, t)| {
            let created = epoch_millis(t.created);
            (
                created as i128,
                json!({
                    "session_id": id,
                    "prompt": t.prompt,
                    "model": t.model.as_ref().map(|m| format!("{}/{}", m.provider_id, m.id)),
                    "agent": t.agent,
                    "outcome": t.status.as_str(),
                    "notify": t.notify,
                    "created": created,
                }),
            )
        })
        .collect();

    // One server fetch serves both rediscovery (our tag) and, if asked, the
    // foreign dump. Partition by our stable origin-tag prefix.
    let same_origin_prefix = format!("cc-bridge:{}:", state.origin);
    let all = state.client.list_sessions().await?;

    let mut other_sessions: Vec<Value> = Vec::new();
    for s in all {
        if tracked_ids.contains(s.id.as_str()) {
            continue; // already represented from the registry
        }
        let title = s.title.as_deref().unwrap_or("");
        if let Some(label) = title.strip_prefix(&same_origin_prefix) {
            // Ours, re-found after a restart. Strip the tag → plain label so
            // the correlation scheme doesn't leak into the caller's view.
            sessions.push((
                s.time.created as i128,
                json!({
                    "session_id": s.id,
                    "prompt": label,
                    "outcome": s.outcome,
                    "created": s.time.created,
                }),
            ));
        } else if include_all {
            other_sessions.push(json!({
                "session_id": s.id,
                "title": s.title,
                "outcome": s.outcome,
                "created": s.time.created,
                "cc_bridge_owned": title.starts_with("cc-bridge:"),
            }));
        }
    }

    sessions.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
    let sessions: Vec<Value> = sessions.into_iter().map(|(_, v)| v).collect();

    let mut out = json!({"sessions": sessions});
    if include_all {
        out["other_sessions"] = json!(other_sessions);
    }
    Ok(out)
}

async fn cancel(state: &AppState, args: Value) -> Result<Value> {
    let session_id = require_str(&args, "session_id")?;
    state.client.interrupt(session_id).await?;
    Ok(json!({"session_id": session_id, "cancelled": true}))
}

/// `opencode_catalog`: `kind` selects which reference list to return —
/// "models" (the model catalog) or "agents" (the agent tags). Both share
/// the `query` substring filter; agents additionally honor `include_hidden`.
async fn catalog(state: &AppState, args: Value) -> Result<Value> {
    match require_str(&args, "kind")? {
        "models" => catalog_models(state, args).await,
        "agents" => catalog_agents(state, args).await,
        other => Err(format!("kind must be \"models\" or \"agents\", got {other:?}").into()),
    }
}

async fn catalog_models(state: &AppState, args: Value) -> Result<Value> {
    const CAP: usize = 200; // SPEC.md §5: "cap the response size" — server has 685 models
    let query = opt_str_strict(&args, "query")?.unwrap_or_default();
    let all = state.client.list_models().await?;
    let mut enabled: Vec<_> = all
        .into_iter()
        .filter(|m| m.enabled)
        // Search over the "providerID/id" handle plus the display name, so a
        // query like "deepseek v4" narrows to the right model (SPEC.md §5).
        .filter(|m| matches_query(&query, &format!("{}/{} {}", m.provider_id, m.id, m.name)))
        .collect();
    enabled.sort_by(|a, b| (&a.provider_id, &a.id).cmp(&(&b.provider_id, &b.id)));
    let matched = enabled.len();
    let truncated = matched > CAP;
    enabled.truncate(CAP);
    let items: Vec<Value> = enabled
        .iter()
        .map(|m| json!({"id": m.id, "providerID": m.provider_id, "name": m.name}))
        .collect();
    Ok(
        json!({"models": items, "matched": matched, "returned": items.len(), "truncated": truncated}),
    )
}

async fn catalog_agents(state: &AppState, args: Value) -> Result<Value> {
    let query = opt_str_strict(&args, "query")?.unwrap_or_default();
    let include_hidden = opt_bool(&args, "include_hidden", false)?;
    let mut all = state.client.list_agents().await?;
    all.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let items: Vec<Value> = all
        .into_iter()
        .filter(|a| include_hidden || !a.hidden)
        .filter(|a| {
            let desc = a.description.as_deref().unwrap_or("");
            matches_query(&query, &format!("{} {}", a.name, desc))
        })
        .map(|a| {
            // "providerID/id" handle, plus the variant (effort level) so
            // e.g. luna vs luna-high are distinguishable. null model = the
            // agent inherits the session default (built-ins like Build).
            let model = a
                .model
                .as_ref()
                .map(|m| format!("{}/{}", m.provider_id, m.id));
            let variant = a.model.as_ref().and_then(|m| m.variant.clone());
            json!({
                "name": a.name,
                "mode": a.mode,
                "description": a.description,
                "model": model,
                "variant": variant,
                "hidden": a.hidden,
            })
        })
        .collect();
    Ok(json!({"agents": items, "count": items.len()}))
}

#[cfg(test)]
mod tests {
    //! Pure-function unit tests for the parts that don't need opencode
    //! running. The full MCP-handshake + live-southbound smoke tests
    //! described in SPEC.md §6 require an opencode2 service; run them
    //! manually with the pipe harness in README.md.
    use super::*;
    use crate::opencode::{
        latest_assistant_text, AgentInfo, AgentModel, Message, MessagePart, MessageTime,
    };
    use serde_json::json;

    #[test]
    fn definitions_lists_four_tools_with_expected_names() {
        // Cross-check against src/tools.rs::definitions(). SPEC §5 lists
        // exactly four tools; if this test fails, either the schema
        // drifted or a tool was added/removed without updating SPEC.md.
        let names: Vec<String> = definitions()
            .into_iter()
            .map(|d| d.get("name").and_then(Value::as_str).unwrap().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "opencode_task".to_string(),
                "opencode_sessions".to_string(),
                "opencode_cancel".to_string(),
                "opencode_catalog".to_string(),
            ]
        );
    }

    #[test]
    fn definitions_each_tool_has_input_schema() {
        // Each tool must carry a JSON Schema object — the MCP client uses
        // it to validate arguments before calling.
        for def in definitions() {
            let name = def.get("name").and_then(Value::as_str).unwrap();
            let schema = def
                .get("inputSchema")
                .unwrap_or_else(|| panic!("{name}: missing inputSchema"));
            assert_eq!(
                schema.get("type").and_then(Value::as_str),
                Some("object"),
                "{name}: inputSchema.type must be object"
            );
        }
    }

    #[test]
    fn parse_model_accepts_provider_slash_id_string() {
        let v = json!({"model": "opencode-go/ox-alpha-free"});
        let parsed = parse_model(&v, "model").unwrap().unwrap();
        assert_eq!(parsed.provider_id, "opencode-go");
        assert_eq!(parsed.id, "ox-alpha-free");
    }

    #[test]
    fn parse_model_accepts_object_form() {
        let v = json!({"model": {"id": "ox-alpha-free", "providerID": "opencode-go"}});
        let parsed = parse_model(&v, "model").unwrap().unwrap();
        assert_eq!(parsed.provider_id, "opencode-go");
        assert_eq!(parsed.id, "ox-alpha-free");
    }

    #[test]
    fn parse_model_rejects_string_without_slash() {
        let v = json!({"model": "nope"});
        assert!(parse_model(&v, "model").is_err());
    }

    #[test]
    fn matches_query_is_case_insensitive_and_anded() {
        let hay = "opencode-go ox-alpha-free Cheap fast model";
        assert!(matches_query("", hay));
        assert!(matches_query("alpha", hay));
        assert!(matches_query("ALPHA", hay));
        assert!(matches_query("cheap fast", hay)); // AND of two terms
        assert!(!matches_query("expensive", hay));
    }

    #[test]
    fn slugify_lowercases_and_collapses_runs() {
        // Cosmetic only — these strings are written to opencode session
        // titles and never parsed back. Lock the shape so an accidental
        // rewrite doesn't silently break rediscovery heuristics.
        assert_eq!(slugify("Hello, World!", 40), "hello-world");
        assert_eq!(slugify("foo___bar", 40), "foo-bar");
        assert_eq!(slugify("---leading-dashes", 40), "leading-dashes");
        assert_eq!(slugify("a".repeat(60).as_str(), 10), "aaaaaaaaaa");
    }

    #[test]
    fn latest_assistant_text_concats_text_parts_skips_reasoning() {
        // Mirrors opencode's GET /message shape: assistant messages carry
        // `content` parts where `type` discriminates `text` from
        // `reasoning`. The final output is the concat of the text parts
        // on the assistant message with the latest `time.created`.
        let messages = vec![
            Message {
                kind: "user".into(),
                time: MessageTime { created: 1 },
                content: vec![],
            },
            Message {
                kind: "assistant".into(),
                time: MessageTime { created: 2 },
                content: vec![
                    MessagePart {
                        kind: "reasoning".into(),
                        text: Some("thinky".into()),
                    },
                    MessagePart {
                        kind: "text".into(),
                        text: Some("hello ".into()),
                    },
                ],
            },
            Message {
                kind: "assistant".into(),
                time: MessageTime { created: 3 },
                content: vec![MessagePart {
                    kind: "text".into(),
                    text: Some("world".into()),
                }],
            },
        ];
        assert_eq!(latest_assistant_text(&messages).as_deref(), Some("world"));
    }

    #[test]
    fn latest_assistant_text_returns_none_with_no_assistant_messages() {
        let messages = vec![Message {
            kind: "user".into(),
            time: MessageTime { created: 1 },
            content: vec![],
        }];
        assert_eq!(latest_assistant_text(&messages), None);
    }

    #[test]
    fn catalog_agents_filter_excludes_hidden_by_default() {
        // We don't run the async handler here (it needs a Client); the
        // filtering logic is `include_hidden || !a.hidden`. Encode that
        // contract by checking the filtered set against a sample.
        let agents = [
            AgentInfo {
                name: "Build".into(),
                description: Some("solid build agent".into()),
                mode: Some("primary".into()),
                hidden: false,
                model: None,
            },
            AgentInfo {
                name: "Title".into(),
                description: Some("internal helper".into()),
                mode: Some("subagent".into()),
                hidden: true,
                model: Some(AgentModel {
                    id: "x".into(),
                    provider_id: "y".into(),
                    variant: None,
                }),
            },
        ];
        let visible: Vec<&str> = agents
            .iter()
            .filter(|a| !a.hidden)
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(visible, vec!["Build"]);
    }
}
