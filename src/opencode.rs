//! HTTP client for the opencode2 REST API (SPEC.md §1). One method per
//! endpoint we use, plus the data shapes needed to interpret responses.
//! This module owns all knowledge of opencode's JSON wire format AND how
//! to obtain/refresh credentials for it (SPEC.md §7.10) — callers
//! (tools.rs, sse.rs) never parse opencode responses or touch `pair`.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionTime {
    pub created: i64,
    pub updated: i64,
    #[serde(default)]
    pub idle: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    #[serde(default)]
    pub outcome: Option<String>,
    pub time: SessionTime,
    pub cost: f64,
    pub tokens: Value,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageTime {
    pub created: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagePart {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub text: Option<String>,
}

/// The error opencode attaches to an assistant message when its turn fails
/// before/while producing output — e.g. a provider `402` credit rejection,
/// which emits `session.execution.failed` with `data.error={type,message,
/// status}` and persists that same shape as a top-level `error` on the
/// assistant message (verified live against `GET /session/{id}/message`).
/// Captured so the bridge can hand the real reason back to the caller
/// instead of an empty output.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageError {
    #[serde(rename = "type")]
    pub kind: String,
    pub message: String,
    #[serde(default)]
    pub status: Option<i64>,
}

impl MessageError {
    /// One-line, human-readable rendering for a tool reply / CC callback,
    /// e.g. `provider.unknown (402): This request requires more credits…`.
    pub fn display(&self) -> String {
        match self.status {
            Some(status) => format!("{} ({}): {}", self.kind, status, self.message),
            None => format!("{}: {}", self.kind, self.message),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    #[serde(rename = "type")]
    pub kind: String, // "user" | "assistant"
    pub time: MessageTime,
    #[serde(default)]
    pub content: Vec<MessagePart>, // present on assistant messages only
    #[serde(default)]
    pub error: Option<MessageError>, // set when this turn failed (assistant only)
}

/// Both halves of a finished turn: the assistant's output `text` (None if
/// the turn produced none) and, if it failed, the `error` reason. A failed
/// turn typically has `text: None, error: Some(..)`; a normal turn has
/// `text: Some(..), error: None`. Both are read from a single `GET
/// /message` so the "how we read opencode's message shape" logic stays in
/// one place (SPEC.md §7).
#[derive(Debug, Clone, Default)]
pub struct FinalTurn {
    pub text: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
}

/// Default per-request HTTP timeout for everything except `/wait` (which is
/// governed by the application-level `WAIT_CAP` in `tools.rs`). A half-open
/// TCP connection or an unresponsive opencode would otherwise stall ordinary
/// calls indefinitely — including the periodic sweep, which iterates
/// tracked sessions serially (SPEC §7.3). 30s is well above any reasonable
/// p99 for an in-process server.
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The model an agent is pinned to (`GET /api/agent` → `.model`). Null for
/// built-in agents (Build/Explore/etc.) that inherit the session's default
/// model instead of pinning one.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentModel {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
}

/// One opencode agent from `GET /api/agent` (SPEC.md §5). The name is the
/// handle callers pass as `opencode_task`'s `agent`.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub model: Option<AgentModel>,
}

/// Final output text of the most recent assistant message: the concat of
/// `text` parts, ignoring `reasoning` parts (SPEC.md §1). `GET /message`
/// returns messages newest-first (observed live), but we don't rely on
/// that ordering — we pick the assistant message with the latest
/// `time.created` explicitly.
pub fn latest_assistant_text(messages: &[Message]) -> Option<String> {
    messages
        .iter()
        .filter(|m| m.kind == "assistant")
        .max_by_key(|m| m.time.created)
        .map(|m| {
            m.content
                .iter()
                .filter(|p| p.kind == "text")
                .filter_map(|p| p.text.as_deref())
                .collect::<Vec<_>>()
                .join("")
        })
}

/// The failure reason on the latest assistant turn, if it carries one.
/// Read from the same "latest assistant message" as `latest_assistant_text`
/// so text and error describe the same turn. `None` for a turn that didn't
/// fail (the normal case).
pub fn latest_assistant_error(messages: &[Message]) -> Option<String> {
    messages
        .iter()
        .filter(|m| m.kind == "assistant")
        .max_by_key(|m| m.time.created)
        .and_then(|m| m.error.as_ref())
        .map(MessageError::display)
}

/// Connection info for the paired opencode2 server. Swappable at runtime
/// (SPEC.md §7.10: a server restart rotates the port and password).
#[derive(Debug, Clone)]
pub struct Creds {
    pub base_url: String, // e.g. "http://127.0.0.1:PORT/api"
    pub username: String,
    pub password: String,
}

/// Resolves the opencode2 binary path: `OPENCODE2_BIN` env override, else
/// `~/.opencode/bin/opencode2` (SPEC.md §1).
pub fn resolve_bin() -> String {
    std::env::var("OPENCODE2_BIN").unwrap_or_else(|_| {
        let home =
            std::env::var("HOME").expect("HOME must be set to locate the default opencode2 binary");
        format!("{home}/.opencode/bin/opencode2")
    })
}

/// Runs `<bin> pair` and parses its "URLs / Username / Password" lines
/// (SPEC.md §1). Called at startup, and again by `Client::repair` after a
/// connect failure or 401 (SPEC.md §7.10).
pub async fn pair(bin: &str) -> Result<Creds> {
    let output = tokio::process::Command::new(bin)
        .arg("pair")
        .output()
        .await
        .map_err(|e| format!("failed to run `{bin} pair`: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "`{bin} pair` exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let url = extract_field(&text, "URLs").ok_or("`opencode2 pair` output missing URLs line")?;
    let username =
        extract_field(&text, "Username").ok_or("`opencode2 pair` output missing Username line")?;
    let password =
        extract_field(&text, "Password").ok_or("`opencode2 pair` output missing Password line")?;

    Ok(Creds {
        base_url: format!("{url}/api"),
        username,
        password,
    })
}

fn extract_field(text: &str, label: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix(label)
            .map(|rest| rest.trim().to_string())
    })
}

/// Distinguishes a failure worth retrying-after-repair (connection
/// refused, or a 401 that means our creds went stale) from every other
/// failure, which is a normal boundary error (SPEC.md §7.10).
enum SendError {
    Retryable(String),
    Fatal(Box<dyn std::error::Error + Send + Sync>),
}

impl From<String> for SendError {
    fn from(s: String) -> Self {
        SendError::Fatal(s.into())
    }
}

pub struct Client {
    http: reqwest::Client,
    creds: RwLock<Creds>,
    opencode2_bin: String,
}

impl Client {
    pub fn new(opencode2_bin: String, creds: Creds) -> Self {
        // Default request timeout = DEFAULT_REQUEST_TIMEOUT. `/wait` overrides
        // this on a per-request basis (it must, since the whole point of
        // `/wait` is to block until the turn goes idle).
        let http = reqwest::Client::builder()
            .timeout(DEFAULT_REQUEST_TIMEOUT)
            .build()
            .expect("reqwest client builder with static config should not fail");
        Self {
            http,
            creds: RwLock::new(creds),
            opencode2_bin,
        }
    }

    /// Re-runs `pair` and swaps in the fresh URL/creds. Called once,
    /// automatically, when a request hits a connect error or 401
    /// (SPEC.md §7.10 — opencode2 restarting rotates its port and password).
    async fn repair(&self) -> Result<()> {
        let fresh = pair(&self.opencode2_bin).await?;
        let mut creds = self.creds.write().await;
        eprintln!(
            "[bridge] opencode: re-paired ({} -> {})",
            creds.base_url, fresh.base_url
        );
        *creds = fresh;
        Ok(())
    }

    /// Sends one request and classifies the outcome: success, retryable
    /// (connect error / 401), or fatal. Doesn't hold the creds lock across
    /// the network await. `timeout` overrides the client's default
    /// per-request timeout — `Some(DEFAULT_REQUEST_TIMEOUT)` for ordinary
    /// calls, `Some(Duration::from_secs(300))` for `/wait`, and `None`
    /// for the long-lived SSE stream (`/event`), which must stay open
    /// indefinitely. Per-read idle timeouts live in `sse.rs` for the
    /// SSE case; the HTTP layer doesn't impose one.
    async fn send_raw(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
        timeout: Option<Duration>,
    ) -> std::result::Result<reqwest::Response, SendError> {
        let (url, username, password) = {
            let creds = self.creds.read().await;
            (
                format!("{}{}", creds.base_url, path),
                creds.username.clone(),
                creds.password.clone(),
            )
        };
        let mut req = self
            .http
            .request(method, url)
            .basic_auth(&username, Some(&password));
        if let Some(t) = timeout {
            req = req.timeout(t);
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.map_err(|e| {
            if e.is_connect() {
                SendError::Retryable(format!("connection error: {e}"))
            } else {
                SendError::Fatal(format!("{path}: request failed: {e}").into())
            }
        })?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SendError::Retryable("401 Unauthorized".to_string()));
        }
        Ok(resp)
    }

    async fn send_once(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
        timeout: Option<Duration>,
    ) -> std::result::Result<Value, SendError> {
        let resp = self.send_raw(method, path, body, timeout).await?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| {
            SendError::Fatal(format!("{path}: failed reading response body: {e}").into())
        })?;
        if !status.is_success() {
            return Err(SendError::Fatal(
                format!("{path} -> {status}: {text}").into(),
            ));
        }
        if text.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| {
            SendError::Fatal(format!("{path}: invalid JSON response ({e}): {text}").into())
        })
    }

    /// Sends one HTTP request, basic-authed, and returns the parsed JSON
    /// body (or `Value::Null` for an empty body, e.g. 204 No Content).
    /// On a connect failure or 401, re-pairs once and retries the whole
    /// request before giving up (SPEC.md §7.10).
    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        self.request_with_timeout(method, path, body, Some(DEFAULT_REQUEST_TIMEOUT))
            .await
    }

    /// Same as `request` but with a per-call timeout. `None` = no
    /// HTTP-layer timeout (used by `/event`); `Some(d)` = cap at `d`
    /// (used by `/wait`).
    async fn request_with_timeout(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
        timeout: Option<Duration>,
    ) -> Result<Value> {
        match self
            .send_once(method.clone(), path, body.as_ref(), timeout)
            .await
        {
            Ok(v) => Ok(v),
            Err(SendError::Fatal(e)) => Err(e),
            Err(SendError::Retryable(reason)) => {
                eprintln!("[bridge] opencode: {reason} on {path} — re-pairing once and retrying");
                self.repair()
                    .await
                    .map_err(|e| format!("re-pair after {reason} failed: {e}"))?;
                self.send_once(method, path, body.as_ref(), timeout)
                    .await
                    .map_err(|e| match e {
                        SendError::Fatal(e) => e,
                    SendError::Retryable(reason2) => format!(
                        "opencode server unreachable — re-pair ({path} still {reason2} after re-pairing)"
                    )
                    .into(),
                })
            }
        }
    }

    fn data_field(path: &str, value: Value) -> Result<Value> {
        match value {
            Value::Object(mut obj) => obj
                .remove("data")
                .ok_or_else(|| format!("{path}: response missing \"data\" field").into()),
            other => Err(format!("{path}: expected a JSON object response, got {other}").into()),
        }
    }

    pub async fn health(&self) -> Result<()> {
        self.request(reqwest::Method::GET, "/health", None).await?;
        Ok(())
    }

    pub async fn create_session(
        &self,
        model: Option<&ModelRef>,
        agent: Option<&str>,
        title: Option<&str>,
        directory: Option<&str>,
    ) -> Result<String> {
        let mut body = serde_json::Map::new();
        if let Some(m) = model {
            body.insert("model".into(), serde_json::to_value(m)?);
        }
        if let Some(a) = agent {
            body.insert("agent".into(), json!(a));
        }
        if let Some(t) = title {
            body.insert("title".into(), json!(t));
        }
        // `location.directory` sets the working directory the agent's tools
        // (edit/read/bash) operate in. Verified live: set → session runs
        // there; omitted → opencode defaults it to the SERVER's cwd
        // ($HOME), not the caller's project. So callers that want the agent
        // to touch a specific repo must pass this (SPEC.md §5).
        if let Some(d) = directory {
            body.insert("location".into(), json!({"directory": d}));
        }
        let resp = self
            .request(reqwest::Method::POST, "/session", Some(Value::Object(body)))
            .await?;
        let data = Self::data_field("/session", resp)?;
        let id = data
            .get("id")
            .and_then(Value::as_str)
            .ok_or("/session: response data missing \"id\"")?;
        Ok(id.to_string())
    }

    /// `origin` is stamped into `metadata.origin` on every prompt (SPEC.md
    /// §8 Layer 2): a durable provenance tag that survives the title being
    /// edited in the TUI. It is NEVER used to decide whether to notify —
    /// see the invariant documented on `registry::Registry::claim_notification`.
    pub async fn prompt(
        &self,
        session_id: &str,
        text: &str,
        delivery: Option<&str>,
        origin: &str,
    ) -> Result<()> {
        let mut body = serde_json::Map::new();
        body.insert("text".into(), json!(text));
        if let Some(d) = delivery {
            body.insert("delivery".into(), json!(d));
        }
        body.insert(
            "metadata".into(),
            json!({"origin": format!("cc-bridge:{origin}"), "bridge": true}),
        );
        self.request(
            reqwest::Method::POST,
            &format!("/session/{session_id}/prompt"),
            Some(Value::Object(body)),
        )
        .await?;
        Ok(())
    }

    pub async fn wait(&self, session_id: &str) -> Result<()> {
        // /wait must block until the turn goes idle — its purpose is to
        // outlast the turn. Override the client's default 30s timeout with
        // a longer per-request one (a few minutes) so the HTTP layer doesn't
        // kill the call before the application-level WAIT_CAP kicks in.
        // (tools.rs races /wait against tokio::time::timeout(WAIT_CAP, ...);
        // see tools::wait_and_finish.)
        self.request_with_timeout(
            reqwest::Method::POST,
            &format!("/session/{session_id}/wait"),
            None,
            Some(Duration::from_secs(300)),
        )
        .await?;
        Ok(())
    }

    pub async fn interrupt(&self, session_id: &str) -> Result<()> {
        self.request(
            reqwest::Method::POST,
            &format!("/session/{session_id}/interrupt"),
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<SessionInfo> {
        let path = format!("/session/{session_id}");
        let resp = self.request(reqwest::Method::GET, &path, None).await?;
        let data = Self::data_field(&path, resp)?;
        Ok(serde_json::from_value(data)?)
    }

    pub async fn list_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let path = format!("/session/{session_id}/message");
        let resp = self.request(reqwest::Method::GET, &path, None).await?;
        let data = Self::data_field(&path, resp)?;
        Ok(serde_json::from_value(data)?)
    }

    /// Convenience: fetch messages once and extract the latest assistant
    /// turn's output text AND its failure reason (if any). Used by both the
    /// sync tool paths and the SSE consumer so the "how do we read
    /// opencode's message shape" logic lives in exactly one place.
    pub async fn final_turn(&self, session_id: &str) -> Result<FinalTurn> {
        let messages = self.list_messages(session_id).await?;
        Ok(FinalTurn {
            text: latest_assistant_text(&messages),
            error: latest_assistant_error(&messages),
        })
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let resp = self.request(reqwest::Method::GET, "/session", None).await?;
        let data = Self::data_field("/session", resp)?;
        Ok(serde_json::from_value(data)?)
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let resp = self.request(reqwest::Method::GET, "/model", None).await?;
        let data = Self::data_field("/model", resp)?;
        Ok(serde_json::from_value(data)?)
    }

    pub async fn list_agents(&self) -> Result<Vec<AgentInfo>> {
        let resp = self.request(reqwest::Method::GET, "/agent", None).await?;
        let data = Self::data_field("/agent", resp)?;
        Ok(serde_json::from_value(data)?)
    }

    /// Opens the global SSE stream. The caller (sse.rs) owns reading it.
    /// Uses `None` for the HTTP-layer timeout so a healthy long-lived SSE
    /// stream isn't killed by the client's default per-request timeout —
    /// per-read idle timeouts live in `sse.rs` for the half-open-TCP case.
    pub async fn events(&self) -> Result<reqwest::Response> {
        match self
            .send_raw(reqwest::Method::GET, "/event", None, None)
            .await
        {
            Ok(resp) if resp.status().is_success() => Ok(resp),
            Ok(resp) => Err(format!("/event -> {}", resp.status()).into()),
            Err(SendError::Fatal(e)) => Err(e),
            Err(SendError::Retryable(reason)) => {
                eprintln!("[bridge] opencode: {reason} on /event — re-pairing once and retrying");
                self.repair()
                    .await
                    .map_err(|e| format!("re-pair after {reason} failed: {e}"))?;
                match self
                    .send_raw(reqwest::Method::GET, "/event", None, None)
                    .await
                {
                    Ok(resp) if resp.status().is_success() => Ok(resp),
                    Ok(resp) => Err(format!("/event -> {}", resp.status()).into()),
                    Err(SendError::Fatal(e)) => Err(e),
                    Err(SendError::Retryable(reason2)) => {
                        Err(format!("opencode server unreachable — re-pair (/event still {reason2} after re-pairing)").into())
                    }
                }
            }
        }
    }
}
