//! The 4 MCP tools (SPEC.md §5). `definitions()` feeds `tools/list`;
//! `call()` is the dispatch table for `tools/call`. Each handler validates
//! its own arguments (boundary) and otherwise just composes `opencode.rs`
//! + `registry.rs` calls — no opencode wire-format knowledge lives here.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::error::Result;
use crate::opencode::{AgentInfo, ModelInfo, ModelRef};
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
            "description": "Look up what's available to run tasks with — returns a plain-text table sorted by relevance. **Agents come first**; use them with opencode_task's `agent` parameter. Raw models are fallbacks when no agent fits. Pass query to filter (case-insensitive substring, space-separated terms ANDed). kind=\"all\" (default) merges both; kind=\"agents\" or kind=\"models\" for one section only.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": {"type": "string", "default": "all", "enum": ["models", "agents", "all"], "description": "\"models\" = model catalog. \"agents\" = agent tags you pass as opencode_task's `agent`. \"all\" (default) = both merged, agents first. Agents are preferred over raw models."},
                    "query": {"type": "string", "description": "Case-insensitive substring filter; space-separated terms are ANDed (all must match). Omit to list all."},
                    "include_hidden": {"type": "boolean", "description": "kind=agents only: include agents opencode marks hidden (internal helpers like Title/Summary). Default false."}
                },
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

/// Rank catalog entries with BM25, a standard lexical-search scorer. The
/// existing substring matcher decides which rows are eligible; BM25 only
/// ranks those matches. Tokens are whitespace-delimited so partial queries
/// such as `deepseek` still match model handles like `deepseek-v4`.
fn bm25_scores(query: &str, documents: &[String]) -> Vec<u32> {
    const K1: f64 = 1.2;
    const B: f64 = 0.75;

    let terms: Vec<String> =
        query
            .split_whitespace()
            .map(str::to_lowercase)
            .fold(Vec::new(), |mut terms, term| {
                if !terms.contains(&term) {
                    terms.push(term);
                }
                terms
            });
    if terms.is_empty() || documents.is_empty() {
        return vec![0; documents.len()];
    }

    let tokenized: Vec<Vec<String>> = documents
        .iter()
        .map(|document| document.split_whitespace().map(str::to_lowercase).collect())
        .collect();
    let document_count = tokenized.len() as f64;
    let average_length =
        tokenized.iter().map(|tokens| tokens.len()).sum::<usize>() as f64 / document_count;

    let raw_scores: Vec<f64> = tokenized
        .iter()
        .map(|tokens| {
            let document_length = tokens.len() as f64;
            terms
                .iter()
                .map(|term| {
                    let term_frequency =
                        tokens.iter().filter(|token| token.contains(term)).count() as f64;
                    if term_frequency == 0.0 {
                        return 0.0;
                    }
                    let document_frequency = tokenized
                        .iter()
                        .filter(|other| other.iter().any(|token| token.contains(term)))
                        .count() as f64;
                    let idf = ((document_count - document_frequency + 0.5)
                        / (document_frequency + 0.5)
                        + 1.0)
                        .ln();
                    let normalization =
                        term_frequency + K1 * (1.0 - B + B * document_length / average_length);
                    idf * term_frequency * (K1 + 1.0) / normalization
                })
                .sum()
        })
        .collect();

    let maximum = raw_scores.iter().copied().fold(0.0, f64::max);
    if maximum == 0.0 {
        return vec![0; documents.len()];
    }
    raw_scores
        .into_iter()
        .map(|score| (score / maximum * 50.0).round() as u32)
        .collect()
}

fn catalog_score(relevance: u32, is_agent: bool) -> u32 {
    relevance + u32::from(is_agent) * 50
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}

fn format_agent_row(agent: &AgentInfo, score: u32) -> String {
    let description = agent.description.as_deref().unwrap_or("");
    let model_label = agent
        .model
        .as_ref()
        .map(|m| format!("{}/{}", m.provider_id, m.id))
        .unwrap_or_else(|| "—".to_string());
    format!(
        "{:<16} {:<18} {:<30} {:>3}",
        truncate(&agent.name, 16),
        truncate(&model_label, 18),
        truncate(description, 30),
        score
    )
}

fn format_model_row(model: &ModelInfo, score: u32) -> String {
    let handle = format!("{}/{}", model.provider_id, model.id);
    format!(
        "{:<30} {:<30} {:>3}",
        truncate(&handle, 30),
        truncate(&model.name, 30),
        score
    )
}

pub async fn call(state: &AppState, name: &str, args: Value) -> Result<Value> {
    match name {
        // Broad tools (SPEC.md §5): each dispatches on whether an optional
        // discriminator is present, so 8 narrow verbs collapse to 4 without
        // losing schema clarity.
        "opencode_task" => task(state, args).await, // session_id absent = start, present = continue
        "opencode_sessions" => sessions(state, args).await, // session_id absent = list, present = detail
        "opencode_cancel" => cancel(state, args).await,
        "opencode_catalog" => catalog(state, args).await, // kind = models | agents | all
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

    sessions.sort_by_key(|a| std::cmp::Reverse(a.0)); // newest first
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
/// "models", "agents", or both (the default). Both share the `query`
/// substring filter; agents additionally honor `include_hidden`.
async fn catalog(state: &AppState, args: Value) -> Result<Value> {
    match opt_str_strict(&args, "kind")?.as_deref() {
        None | Some("all") => catalog_all(state, args).await,
        Some("models") => catalog_models(state, args).await,
        Some("agents") => catalog_agents(state, args).await,
        Some(other) => {
            Err(format!("kind must be \"models\", \"agents\", or \"all\", got {other:?}").into())
        }
    }
}

async fn catalog_models(state: &AppState, args: Value) -> Result<Value> {
    const CAP: usize = 200;
    let query = opt_str_strict(&args, "query")?.unwrap_or_default();
    let models: Vec<_> = state
        .client
        .list_models()
        .await?
        .into_iter()
        .filter(|m| m.enabled)
        .collect();
    let documents: Vec<String> = models
        .iter()
        .map(|m| format!("{}/{} {}", m.provider_id, m.id, m.name))
        .collect();
    let scores = bm25_scores(&query, &documents);
    let mut items = Vec::new();
    for ((m, searchable), score) in models.into_iter().zip(documents).zip(scores) {
        if !matches_query(&query, &searchable) {
            continue;
        }
        let handle = format!("{}/{}", m.provider_id, m.id);
        let line = format_model_row(&m, score);
        items.push((line, score, m.name.to_lowercase(), handle));
    }
    items.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));
    let matched = items.len();
    let truncated = matched > CAP;
    items.truncate(CAP);

    let mut lines: Vec<String> = items.into_iter().map(|(line, _, _, _)| line).collect();
    if truncated {
        lines.push(format!(
            "({} of {} shown — refine your query to narrow results)",
            CAP, matched
        ));
    }

    if lines.is_empty() {
        return Ok(Value::String("No models found.".to_string()));
    }

    Ok(Value::String(lines.join("\n")))
}

async fn catalog_agents(state: &AppState, args: Value) -> Result<Value> {
    let query = opt_str_strict(&args, "query")?.unwrap_or_default();
    let include_hidden = opt_bool(&args, "include_hidden", false)?;
    let agents: Vec<_> = state
        .client
        .list_agents()
        .await?
        .into_iter()
        .filter(|a| include_hidden || !a.hidden)
        .collect();
    let documents: Vec<String> = agents
        .iter()
        .map(|a| format!("{} {}", a.name, a.description.as_deref().unwrap_or("")))
        .collect();
    let scores = bm25_scores(&query, &documents);
    let mut items = Vec::new();
    for ((a, searchable), relevance) in agents.into_iter().zip(documents).zip(scores) {
        if !matches_query(&query, &searchable) {
            continue;
        }
        let score = catalog_score(relevance, true);
        let name = a.name.clone();
        let line = format_agent_row(&a, score);
        items.push((line, score, name.to_lowercase()));
    }
    items.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(a.0.cmp(&b.0)));

    if items.is_empty() {
        return Ok(Value::String("No agents found.".to_string()));
    }

    Ok(Value::String(
        items
            .into_iter()
            .map(|(line, _, _)| line)
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

fn render_catalog(
    agents: Vec<(String, u32, String)>,
    models: Vec<(String, u32, String, String)>,
) -> String {
    const CAP: usize = 200;
    let matched = agents.len() + models.len();
    let truncated = matched > CAP;
    let agent_count = agents.len().min(CAP);
    let model_count = (CAP - agent_count).min(models.len());

    let mut lines: Vec<String> = Vec::new();
    if agent_count > 0 {
        lines.push("── Agents ──".to_string());
        lines.extend(
            agents
                .into_iter()
                .take(agent_count)
                .map(|(line, _, _)| line),
        );
    }
    if !models.is_empty() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push("── Models ──".to_string());
        lines.extend(
            models
                .into_iter()
                .take(model_count)
                .map(|(line, _, _, _)| line),
        );
    }

    if truncated {
        lines.push(format!(
            "({} of {} shown — refine your query to narrow results)",
            CAP, matched
        ));
    }

    if lines.is_empty() {
        "No results.".to_string()
    } else {
        lines.join("\n")
    }
}

async fn catalog_all(state: &AppState, args: Value) -> Result<Value> {
    let query = opt_str_strict(&args, "query")?.unwrap_or_default();
    let include_hidden = opt_bool(&args, "include_hidden", false)?;

    let all_agents: Vec<_> = state
        .client
        .list_agents()
        .await?
        .into_iter()
        .filter(|a| include_hidden || !a.hidden)
        .collect();
    let all_models: Vec<_> = state
        .client
        .list_models()
        .await?
        .into_iter()
        .filter(|m| m.enabled)
        .collect();
    let agent_documents: Vec<String> = all_agents
        .iter()
        .map(|a| format!("{} {}", a.name, a.description.as_deref().unwrap_or("")))
        .collect();
    let model_documents: Vec<String> = all_models
        .iter()
        .map(|m| format!("{}/{} {}", m.provider_id, m.id, m.name))
        .collect();
    let mut documents = agent_documents.clone();
    documents.extend(model_documents.iter().cloned());
    let scores = bm25_scores(&query, &documents);
    let (agent_scores, model_scores) = scores.split_at(all_agents.len());

    let mut agents = Vec::new();
    for ((a, searchable), relevance) in all_agents
        .into_iter()
        .zip(agent_documents)
        .zip(agent_scores.iter().copied())
    {
        if !matches_query(&query, &searchable) {
            continue;
        }
        let score = catalog_score(relevance, true);
        let name = a.name.clone();
        let line = format_agent_row(&a, score);
        agents.push((line, score, name.to_lowercase()));
    }
    agents.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(a.0.cmp(&b.0)));

    let mut models = Vec::new();
    for ((m, searchable), score) in all_models
        .into_iter()
        .zip(model_documents)
        .zip(model_scores.iter().copied())
    {
        if !matches_query(&query, &searchable) {
            continue;
        }
        let handle = format!("{}/{}", m.provider_id, m.id);
        let line = format_model_row(&m, score);
        models.push((line, score, m.name.to_lowercase(), handle));
    }
    models.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));

    Ok(Value::String(render_catalog(agents, models)))
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
    fn strict_optional_string_rejects_wrong_types() {
        assert_eq!(opt_str_strict(&json!({}), "kind").unwrap(), None);
        assert_eq!(
            opt_str_strict(&json!({"kind": "agents"}), "kind").unwrap(),
            Some("agents".to_string())
        );
        assert!(opt_str_strict(&json!({"kind": true}), "kind").is_err());
    }

    #[test]
    fn bm25_empty_query_has_no_relevance() {
        let documents = vec!["anything".to_string(), "something else".to_string()];
        assert_eq!(bm25_scores("", &documents), vec![0, 0]);
    }

    #[test]
    fn bm25_prefers_shorter_matching_documents() {
        let documents = vec!["deepseek pro fast".to_string(), "deepseek pro".to_string()];
        let scores = bm25_scores("deepseek pro", &documents);
        assert!(scores[1] > scores[0]);
        assert_eq!(scores[1], 50);
    }

    #[test]
    fn bm25_scores_partial_and_nonmatching_documents_lower() {
        let documents = vec![
            "deepseek pro".to_string(),
            "deepseek".to_string(),
            "other".to_string(),
        ];
        let scores = bm25_scores("deepseek pro", &documents);
        assert_eq!(scores[0], 50);
        assert!(scores[1] < scores[0]);
        assert_eq!(scores[2], 0);
    }

    #[test]
    fn catalog_score_reserves_top_half_for_agents() {
        assert_eq!(catalog_score(0, false), 0);
        assert_eq!(catalog_score(50, false), 50);
        assert_eq!(catalog_score(0, true), 50);
        assert_eq!(catalog_score(50, true), 100);
    }

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let s = "a".repeat(50);
        assert_eq!(truncate(&s, 10), "aaaaaaaaa…");
        assert_eq!(truncate(&s, 10).chars().count(), 10);
    }

    #[test]
    fn catalog_rows_are_plain_text_and_truncated_to_columns() {
        let agent = AgentInfo {
            name: "abcdefghijklmnopq".into(),
            description: Some(
                "a description that is deliberately longer than thirty characters".into(),
            ),
            hidden: false,
            model: Some(AgentModel {
                id: "a-very-long-model-id".into(),
                provider_id: "provider".into(),
            }),
        };
        let agent_row = format_agent_row(&agent, 100);
        assert!(agent_row.starts_with("abcdefghijklmno…"));
        assert!(agent_row.contains("provider/a-very-l…"));
        assert!(agent_row.ends_with("100"));

        let model = ModelInfo {
            id: "model".into(),
            provider_id: "provider".into(),
            name: "Display name".into(),
            enabled: true,
        };
        let model_row = format_model_row(&model, 50);
        assert_eq!(&model_row[..30], "provider/model                ");
        assert_eq!(&model_row[31..61], "Display name                  ");
        assert_eq!(&model_row[62..], " 50");
    }

    #[test]
    fn capped_combined_catalog_keeps_header_for_hidden_matches() {
        let agents = (0..201)
            .map(|i| (format!("agent-{i}"), 100, format!("agent-{i}")))
            .collect();
        let models = vec![(
            "provider/model".into(),
            50,
            "model".into(),
            "provider/model".into(),
        )];

        let rendered = render_catalog(agents, models);
        assert!(rendered.contains("── Agents ──"));
        assert!(rendered.contains("── Models ──"));
        assert!(rendered.contains("(200 of 202 shown"));
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
                hidden: false,
                model: None,
            },
            AgentInfo {
                name: "Title".into(),
                description: Some("internal helper".into()),
                hidden: true,
                model: Some(AgentModel {
                    id: "x".into(),
                    provider_id: "y".into(),
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
