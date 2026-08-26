//! CC callback protocol (SPEC.md §2, "already proven; port verbatim"):
//! posts a completion message into the launching Claude Code session's
//! inbox over an AF_UNIX socket.

use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

use crate::error::Result;

pub struct Notifier {
    socket_path: Option<String>,
    token: Option<String>,
}

impl Notifier {
    pub fn from_env() -> Self {
        // Empty ⇒ callbacks disabled (degrade gracefully) — SPEC.md §2.
        let socket_path = std::env::var("CLAUDE_CODE_MESSAGING_SOCKET")
            .ok()
            .filter(|s| !s.is_empty());
        let token = std::env::var("CLAUDE_CODE_MESSAGING_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        Self { socket_path, token }
    }

    pub fn enabled(&self) -> bool {
        self.socket_path.is_some()
    }

    /// Best-effort: post `text` into the launching CC session. A failure
    /// here must never crash the SSE consumer or lose the underlying
    /// opencode result — the caller has already recorded that in the
    /// registry before calling this. FALLBACK-OK: SPEC.md §2 — the whole
    /// callback channel is documented as optional/degrade-gracefully, so a
    /// delivery failure is logged and dropped, not propagated.
    pub async fn notify(&self, text: &str) {
        let Some(path) = &self.socket_path else {
            return; // no socket configured — callbacks disabled
        };
        if let Err(e) = self.send(path, text).await {
            eprintln!("[bridge] notify: failed to deliver CC callback: {e}");
        }
    }

    async fn send(&self, path: &str, text: &str) -> Result<()> {
        let mut stream = UnixStream::connect(path).await?;

        if let Some(token) = &self.token {
            let auth = serde_json::json!({"type": "auth", "token": token});
            write_line(&mut stream, &auth.to_string()).await?;
        }

        let msg = serde_json::json!({
            "type": "user",
            "message": {"role": "user", "content": text}
        });
        write_line(&mut stream, &msg.to_string()).await?;

        stream.shutdown().await?;
        Ok(())
    }
}

async fn write_line(stream: &mut UnixStream, line: &str) -> Result<()> {
    stream.write_all(line.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    Ok(())
}
