//! The Claude listener for normal dashboard startup — T04 (see
//! `tasks/2026-09-03-claude-dashboard-t04-runtime.design.md`).
//!
//! Owns the user-scoped Unix socket: resolves exactly T02's
//! [`claude_socket_path`] (no shared fallback), prepares the target safely
//! (a stale socket is replaced; a regular file or symlink is refused, never
//! deleted), accepts short-lived independent connections, reads at most one
//! bounded newline-delimited frame per connection under a finite deadline,
//! decodes through T03's [`decode_envelope`], and submits only successful
//! typed [`ClaudeIpcEnvelope`]s to the T03 sender. Runtime shutdown removes
//! the owned socket best-effort. Lifecycle mapping and snapshot construction
//! remain T03's; this module never does either.
//!
//! CONTRACT: ClaudeListener (T04; `tasks/2026-09-03-claude-dashboard-support/
//! contracts/T04-claude-runtime.md` §Listener contract and §Startup contract;
//! `docs/specs/dashboard/claude.md` R15-R16)
//!
//! GUARANTEES:
//!   - Binds only the exact T02 user-scoped path (or an explicit test path);
//!     never a public TCP endpoint or a shared temp fallback.
//!   - Prepares the target safely with `symlink_metadata`: removes a path
//!     only when it is a Unix socket, with the socket's device/inode identity
//!     re-checked immediately before removal so a path swapped underneath is
//!     never deleted; refuses (without deleting) regular files and symlinks.
//!   - Accepts independent connections under a fixed semaphore bound; each
//!     connection reads at most `MAX_ENVELOPE_BYTES + 1` bytes from a fixed-size
//!     buffer under a single finite deadline, carries at most one newline-
//!     delimited frame, and recognizes a second frame even when the first
//!     frame fills the envelope bound exactly. Malformed, unknown-version,
//!     unknown-event, out-of-bounds, multiple-frame, oversized, unterminated,
//!     and silent connections are dropped category-only; later valid
//!     connections continue.
//!   - Sends only a successful T03 `decode_envelope` result to the T03
//!     sender; never maps lifecycle events or constructs snapshots.
//!   - Removes its owned socket file on listener-task shutdown (and on drop
//!     without `run`) only while the path still names the exact socket it
//!     bound — same device/inode identity and still a socket; a path replaced
//!     by a regular file, symlink, or different socket is never deleted.
//!     Cleanup is best-effort and category-only.
//!
//! EXPECTS:
//!   - T02/T03 to remain authoritative for path resolution, parse, delivery,
//!     and envelope decoding; this module only reads bytes, applies the frame
//!     bound, and forwards typed results.
//!   - `run` to be called inside an active Tokio runtime (it spawns the
//!     accept loop).
//!
//! FAILURE BEHAVIOR:
//!   - Path preparation/bind failures yield a category-only [`ListenerError`]
//!     with no path and no OS error text; the dashboard continues without
//!     Claude monitoring (T04 §Startup contract).
//!   - Accept/read/cleanup failures are category-only log lines that never
//!     print paths, payloads, or OS error text; a single bad connection
//!     cannot affect later ones (T04 §Listener contract).
//!   - A closed T03 channel ends the accept loop and triggers socket cleanup
//!     (T04 design, "Bounds And Shutdown").
//!
//! DOES NOT:
//!   - Read or write Claude configuration, access transcripts, enforce
//!     session control, map lifecycle events, or construct snapshots.

use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;

use super::hook::{claude_socket_path, ClaudeIpcEnvelope, MAX_ENVELOPE_BYTES};
use super::wire::decode_envelope;

/// Maximum number of connections processed concurrently. Connections beyond
/// this bound wait for a slot; a silent connection frees its slot at
/// [`FRAME_READ_TIMEOUT`], so valid clients are never starved (T04 design,
/// "Bounds And Shutdown").
pub const MAX_CONCURRENT_CONNECTIONS: usize = 8;

/// The single finite deadline for reading one connection's frame. A client
/// that never completes a frame (silent or trickling) is released after
/// this, freeing its concurrency slot.
pub const FRAME_READ_TIMEOUT: Duration = Duration::from_secs(1);

/// Why listener binding could not complete. Category-only: never carries a
/// path, a payload, or OS error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerError {
    /// No user-scoped socket path could be established (T02's resolver
    /// returned `None`).
    NoUserScopedPath,
    /// The target path exists but is not a Unix socket (regular file,
    /// symlink, or other); it was refused and is never deleted.
    RefusedNonSocket,
    /// Binding failed for any other reason (missing parent, permissions,
    /// address in use, ...). Disables only Claude monitoring.
    BindFailed,
}

impl ListenerError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoUserScopedPath => "no user-scoped socket path",
            Self::RefusedNonSocket => "refused non-socket path",
            Self::BindFailed => "bind failed",
        }
    }
}

/// Stable filesystem identity of the socket actually bound: device + inode.
/// Drop removes the owned path only while it still names exactly this
/// identity, so a replacement regular file, symlink, or different socket is
/// never deleted (cleanup ownership).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SocketIdentity {
    dev: u64,
    ino: u64,
}

/// The device/inode pair from Unix `Metadata` — the standard stable per-object
/// identity that the path-based `remove_file` API cannot express.
fn socket_identity(metadata: &fs::Metadata) -> SocketIdentity {
    SocketIdentity {
        dev: metadata.dev(),
        ino: metadata.ino(),
    }
}

/// True when `path` currently names exactly the Unix socket with `identity`:
/// `symlink_metadata` (which never follows a final symlink) must report a
/// socket whose device/inode match. A missing path, a symlink, a regular
/// file, or a different socket is never this socket.
fn is_same_socket(path: &Path, identity: SocketIdentity) -> bool {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() => socket_identity(&metadata) == identity,
        _ => false,
    }
}

/// A bound Claude listener. Owns the socket file at `path` until dropped —
/// whether dropped by the caller before `run`, or by the accept task when
/// the T03 channel closes or the task is aborted. The captured `identity` is
/// the socket actually bound: at drop the path is removed only while it still
/// names that exact socket.
pub struct ClaudeListener {
    listener: UnixListener,
    path: PathBuf,
    identity: Option<SocketIdentity>,
}

impl ClaudeListener {
    /// Binds the exact T02 user-scoped socket path (see
    /// [`claude_socket_path`]). No shared fallback: when T02 resolves no
    /// user-scoped location, binding is unavailable.
    pub fn bind() -> Result<ClaudeListener, ListenerError> {
        match claude_socket_path() {
            Some(path) => Self::bind_at(&path),
            None => Err(ListenerError::NoUserScopedPath),
        }
    }

    /// Binds an explicit path (normal-startup composition and test seam).
    /// Prepares the target safely: a stale Unix socket is removed first; a
    /// non-socket path (regular file, symlink, ...) is refused and never
    /// deleted. The stale replacement is hardened against races: the socket's
    /// device/inode identity is captured and re-checked immediately before
    /// removal, so a path that changed underneath is refused, never deleted.
    pub fn bind_at(path: &Path) -> Result<ClaudeListener, ListenerError> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_socket() => {
                // A stale socket file from a previous listener: replace it.
                // Re-check its identity right before removal — if the path
                // was swapped for a different socket or a non-socket in the
                // interim (unlink + recreate is not atomic), refuse rather
                // than delete the replacement.
                let identity = socket_identity(&metadata);
                match fs::symlink_metadata(path) {
                    // Still the same socket: safe to remove. If removal
                    // fails, the bind below fails and surfaces the
                    // category; the error is never silent.
                    Ok(current)
                        if current.file_type().is_socket()
                            && socket_identity(&current) == identity =>
                    {
                        let _ = fs::remove_file(path);
                    }
                    // The stale socket vanished before we could remove it:
                    // nothing left to replace, proceed to bind.
                    Err(_) => {}
                    // Replaced by a non-socket or a different socket:
                    // refuse, never delete.
                    Ok(_) => return Err(ListenerError::RefusedNonSocket),
                }
            }
            Ok(_) => return Err(ListenerError::RefusedNonSocket),
            Err(_) => {
                // The path is absent (or metadata is unreadable, in which
                // case the bind below fails and surfaces it).
            }
        }
        let listener = match UnixListener::bind(path) {
            Ok(listener) => listener,
            Err(_) => return Err(ListenerError::BindFailed),
        };
        // Capture the identity of the socket we actually bound: Drop later
        // removes the path only while it still names exactly this socket.
        // If the identity cannot be read right after a successful bind (an
        // exotic filesystem), cleanup is conservatively skipped on drop.
        let identity = fs::symlink_metadata(path)
            .ok()
            .filter(|metadata| metadata.file_type().is_socket())
            .map(|metadata| socket_identity(&metadata));
        Ok(ClaudeListener {
            listener,
            path: path.to_path_buf(),
            identity,
        })
    }

    /// Starts the accept loop: accepts connections, bounds concurrency with
    /// a semaphore, reads one bounded frame per connection, decodes through
    /// T03, and forwards only successful typed envelopes to `sender`. Ends
    /// when `sender`'s channel closes (the T03 adapter is gone) or the
    /// handle is aborted; either way the owned socket is removed on
    /// shutdown.
    pub fn run(self, sender: UnboundedSender<ClaudeIpcEnvelope>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));
            loop {
                tokio::select! {
                    // A closed T03 channel ends forwarding: the adapter is
                    // gone, so there is no sink to feed (design, "Bounds And
                    // Shutdown"). `self` drops here — socket cleanup runs.
                    _ = sender.closed() => break,
                    accepted = self.listener.accept() => {
                        let (stream, _) = match accepted {
                            Ok(accepted) => accepted,
                            Err(_) => {
                                // FALLBACK-OK: design "Validation Scenarios" —
                                // a failed accept is a category-only drop and
                                // the loop continues for later clients.
                                log_category("accept failed");
                                continue;
                            }
                        };
                        // Bound concurrent processing. A connection beyond
                        // the bound waits for a slot rather than starving
                        // valid clients; silent clients free a slot at
                        // FRAME_READ_TIMEOUT (design, "Validation Scenarios").
                        let permit = match semaphore.clone().acquire_owned().await {
                            Ok(permit) => permit,
                            Err(_) => {
                                // Unreachable: the semaphore is created here
                                // and never closed, so acquisition cannot fail.
                                unreachable!("listener semaphore is never closed");
                            }
                        };
                        tokio::spawn(handle_connection(stream, permit, sender.clone()));
                    }
                }
            }
        })
    }
}

impl Drop for ClaudeListener {
    fn drop(&mut self) {
        // Remove only while the path still names the exact socket we bound:
        // same device/inode identity and still a socket (`symlink_metadata`
        // never follows a symlink). A replacement regular file, symlink, or
        // different socket is never deleted; a missing path has nothing to
        // remove. Leftover paths are harmless anyway — T02 delivery and the
        // next bind both handle a stale socket.
        // FALLBACK-OK: design "Bounds And Errors" — cleanup failure is
        // explicitly category-only, never an error the dashboard propagates.
        if let Some(identity) = self.identity {
            if is_same_socket(&self.path, identity) && fs::remove_file(&self.path).is_err() {
                log_category("socket cleanup failed");
            }
        }
    }
}

/// Result of the one-frame read for a connection.
enum FrameRead {
    /// Exactly one newline-delimited frame (bytes up to the first newline).
    Frame(Vec<u8>),
    /// The connection closed before sending any byte.
    Empty,
    /// The connection closed mid-frame without a terminating newline.
    Unterminated,
    /// The fixed-size buffer filled without a newline: over
    /// `MAX_ENVELOPE_BYTES`.
    Oversized,
    /// Bytes followed the first newline before the connection closed: the
    /// connection carried more than one frame and is dropped whole.
    Multiple,
}

/// Reads at most `MAX_ENVELOPE_BYTES + 1` bytes from `stream` until the first
/// newline, EOF, the buffer filling past the bound, or the caller's deadline.
/// The single extra byte (or the block that carries it) is read only to tell
/// an exact-boundary single frame — a frame that fills `MAX_ENVELOPE_BYTES`
/// exactly, newline last — apart from a two-frame connection where a second
/// frame follows it; it never forwards more than one frame and the buffer
/// stays `MAX_ENVELOPE_BYTES + 1` bytes. Reading past the first newline (up
/// to the bound) lets the reader tell a single-frame connection from a
/// multiple-frame one deterministically, independent of how the kernel chunks
/// the bytes.
async fn read_one_frame(stream: &mut UnixStream) -> Result<FrameRead, ()> {
    let mut buf = [0u8; MAX_ENVELOPE_BYTES + 1];
    let mut len = 0usize;
    let mut newline_at: Option<usize> = None;
    loop {
        if len == MAX_ENVELOPE_BYTES + 1 {
            // The bound plus the one classification byte is full. `classify`
            // reports Oversized when the first newline is at or past the
            // bound, Multiple when bytes followed the first complete frame,
            // and never a Frame here (a frame ending at byte MAX_ENVELOPE_BYTES
            // would itself be MAX_ENVELOPE_BYTES + 1 bytes long).
            return Ok(classify_frame(&buf[..len], newline_at));
        }
        let n = match stream.read(&mut buf[len..]).await {
            Ok(n) => n,
            Err(_) => return Err(()),
        };
        if n == 0 {
            // EOF: classify exactly what arrived.
            return Ok(classify_frame(&buf[..len], newline_at));
        }
        len += n;
        if newline_at.is_none() {
            newline_at = buf[..len].iter().position(|&b| b == b'\n');
        }
    }
}

fn classify_frame(buf: &[u8], newline_at: Option<usize>) -> FrameRead {
    match newline_at {
        None => {
            if buf.is_empty() {
                FrameRead::Empty
            } else if buf.len() >= MAX_ENVELOPE_BYTES {
                FrameRead::Oversized
            } else {
                FrameRead::Unterminated
            }
        }
        Some(i) => {
            // A first newline at or past the bound means the first frame
            // (body + newline) itself exceeds MAX_ENVELOPE_BYTES.
            if i >= MAX_ENVELOPE_BYTES {
                FrameRead::Oversized
            } else if i + 1 < buf.len() {
                FrameRead::Multiple
            } else {
                FrameRead::Frame(buf[..i].to_vec())
            }
        }
    }
}

async fn handle_connection(
    mut stream: UnixStream,
    _permit: OwnedSemaphorePermit,
    sender: UnboundedSender<ClaudeIpcEnvelope>,
) {
    // One finite deadline for the entire frame read: a silent or trickling
    // client is released after this, freeing its concurrency slot.
    let frame = match tokio::time::timeout(FRAME_READ_TIMEOUT, read_one_frame(&mut stream)).await {
        Ok(Ok(frame)) => frame,
        Ok(Err(())) => {
            // FALLBACK-OK: design "Validation Scenarios" — a failed read is
            // a category-only connection drop; later clients are unaffected.
            log_category("connection read failed");
            return;
        }
        Err(_) => {
            log_category("dropped (read deadline)");
            return;
        }
    };

    let bytes = match frame {
        FrameRead::Frame(bytes) => bytes,
        FrameRead::Empty => {
            log_category("dropped (empty connection)");
            return;
        }
        FrameRead::Unterminated => {
            log_category("dropped (unterminated frame)");
            return;
        }
        FrameRead::Oversized => {
            log_category("dropped (oversized frame)");
            return;
        }
        FrameRead::Multiple => {
            log_category("dropped (multiple frames)");
            return;
        }
    };

    // Bytes up to the first newline must be valid UTF-8 before T03 decodes.
    let line = match std::str::from_utf8(&bytes) {
        Ok(line) => line,
        Err(_) => {
            log_category("dropped (invalid utf-8)");
            return;
        }
    };

    match decode_envelope(line) {
        Ok(envelope) => {
            if sender.send(envelope).is_err() {
                // FALLBACK-OK: design "Bounds And Shutdown" — a closed T03
                // channel ends forwarding; the accept loop's `closed()` select
                // stops the listener promptly, and this connection has no
                // sink.
                log_category("channel closed");
            }
        }
        Err(error) => {
            // Only the category is logged, never the rejected payload.
            log_category(&format!("dropped ({})", error.as_str()));
        }
    }
}

/// Category-only stderr line: never paths, payloads, or OS error text.
fn log_category(message: &str) {
    eprintln!("[dashboard] claude listener: {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `classify_frame` decides purely from `newline_at` vs `len`; the
    /// buffer contents are irrelevant to the category, only the length and
    /// newline position matter.
    fn classify(len: usize, newline_at: Option<usize>) -> FrameRead {
        let mut buf = [0u8; MAX_ENVELOPE_BYTES + 1];
        if let Some(i) = newline_at {
            if i < len {
                buf[i] = b'\n';
            }
        }
        classify_frame(&buf[..len], newline_at)
    }

    #[test]
    fn a_single_frame_exactly_fills_the_buffer_is_accepted() {
        // One line whose newline is the final byte and the buffer is exactly
        // full: a complete single frame with no room for more (EOF follows).
        let len = MAX_ENVELOPE_BYTES;
        assert!(matches!(classify(len, Some(len - 1)), FrameRead::Frame(_)));
    }

    #[test]
    fn an_exact_boundary_frame_followed_by_more_bytes_is_multiple() {
        // The first frame fills the envelope bound exactly (newline last) —
        // the case a reader that stops at the bound would accept — and one
        // more byte (a second frame) follows: the connection carried two
        // frames and must be dropped whole.
        assert!(matches!(
            classify(MAX_ENVELOPE_BYTES + 1, Some(MAX_ENVELOPE_BYTES - 1)),
            FrameRead::Multiple
        ));
        // A first newline landing on the extra byte itself makes the first
        // frame MAX_ENVELOPE_BYTES + 1 bytes long: oversized, never a frame.
        assert!(matches!(
            classify(MAX_ENVELOPE_BYTES + 1, Some(MAX_ENVELOPE_BYTES)),
            FrameRead::Oversized
        ));
        // No newline anywhere in the bound plus classification byte:
        // oversized.
        assert!(matches!(
            classify(MAX_ENVELOPE_BYTES + 1, None),
            FrameRead::Oversized
        ));
    }

    #[test]
    fn empty_eof_unterminated_oversized_and_multiple_are_distinguished() {
        assert!(matches!(classify(0, None), FrameRead::Empty));
        assert!(matches!(classify(3, None), FrameRead::Unterminated));
        assert!(matches!(
            classify(MAX_ENVELOPE_BYTES, None),
            FrameRead::Oversized
        ));
        assert!(matches!(classify(5, Some(3)), FrameRead::Multiple));
        assert!(matches!(classify(3, Some(2)), FrameRead::Frame(_)));
    }
}
