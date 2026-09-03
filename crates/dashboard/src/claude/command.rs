//! The `dashboard claude-hook` helper command — T04 (see
//! `tasks/2026-09-03-claude-dashboard-t04-runtime.design.md`).
//!
//! A user-configured Claude Code command hook (R11-R12) runs this command
//! with one hook event on stdin. It performs a bounded, deadline-capped
//! read, hands the payload to T02's [`parse_hook_input`] with a local receipt
//! time, and invokes T02's best-effort [`deliver`]. Every expected
//! drop/unavaliable case — malformed, unknown, oversized, invalid UTF-8,
//! absent listener, unavailable listener, empty input — exits successfully
//! and writes nothing to stdout, so a hook can never block or fail Claude
//! (R16).
//!
//! CONTRACT: ClaudeHookCommand (T04; `tasks/2026-09-03-claude-dashboard-
//! support/contracts/T04-claude-runtime.md` §Hook command contract;
//! `docs/specs/dashboard/claude.md` R11/R12/R16)
//!
//! GUARANTEES:
//!   - Reads at most `MAX_HOOK_INPUT_BYTES + 1` bytes from stdin under a
//!     single finite wall-clock deadline; an oversized or stalled payload can
//!     never make the helper read more or hang. The read runs on a dedicated
//!     OS thread and the process exits at the deadline even when stdin stays
//!     open — a pipe held open can never keep the helper process alive past
//!     [`HOOK_READ_TIMEOUT`].
//!   - Parses only valid UTF-8 through T02. Invalid UTF-8 is a category-only
//!     drop that still exits 0.
//!   - Delegates accepted records to T02 `deliver` with `ReceivedAt::now()`;
//!     every delivery outcome (`Sent`, `ListenerAbsent`, `ListenerUnavailable`)
//!     is a success for the hook.
//!   - Never writes stdout; logs only category strings on stderr, never
//!     payload values.
//!   - Never pairs with OpenCode, starts a listener or TUI, reads Claude
//!     configuration, or touches transcripts.
//!
//! EXPECTS:
//!   - T02 to remain authoritative for parsing, path resolution, and
//!     delivery; this command adds no parsing or path logic of its own.
//!
//! FAILURE BEHAVIOR:
//!   - No enumerated input can make the helper fail: every expected
//!     drop/unavailable case is an exit-0 category-only drop. An unbuildable
//!     async runtime or failed stdin read is the same harmless case
//!     (FALLBACK-OK: R16 — a hook must never fail Claude). A read that does
//!     not finish within [`HOOK_READ_TIMEOUT`] — stdin open with no/partial
//!     bytes — returns success immediately, and the still-blocked reader
//!     thread is terminated with the process, so the helper never outlives
//!     the bounded read.
//!
//! DOES NOT:
//!   - Own stdout output, configuration access, transcript access, OpenCode
//!     pairing, or the listener.

use std::io::Read;
use std::process::ExitCode;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use super::hook::{deliver, parse_hook_input, ParseOutcome, ReceivedAt, MAX_HOOK_INPUT_BYTES};

/// The exact first argument that selects helper mode (`claude.md` R11).
pub const HOOK_COMMAND: &str = "claude-hook";

/// Total budget for the stdin read. Claude writes a hook payload
/// immediately, so any longer wait is a stalled runner; the helper must
/// never hang Claude waiting for input that never ends. This is a hard
/// process bound: when it expires the helper process terminates even if
/// stdin is still open (R16).
pub const HOOK_READ_TIMEOUT: Duration = Duration::from_secs(1);

/// The `dashboard claude-hook` command. See the module CONTRACT.
pub struct ClaudeHookCommand;

impl ClaudeHookCommand {
    /// Runs the helper to completion and returns its process exit code.
    /// Always `ExitCode::SUCCESS`: no enumerated input can fail the hook.
    ///
    /// The bounded stdin read runs on a dedicated OS thread while this thread
    /// waits on a channel with a hard [`HOOK_READ_TIMEOUT`]. When the deadline
    /// expires, `run()` returns and the process exits even though the reader
    /// thread is still blocked holding an open stdin — the deadline is a real
    /// process bound, which cancelling an async read alone would not be.
    pub fn run() -> ExitCode {
        // Bounded read: at most MAX_HOOK_INPUT_BYTES + 1 bytes so an oversized
        // payload is observable without reading an unbounded stream, under a
        // single finite wall-clock deadline so a stalled runner cannot hang
        // Claude (R16). The read uses std's blocking stdin on its own thread
        // (not Tokio's stdin, whose cancelled read would keep a runtime thread
        // and therefore the process alive past the deadline): when the main
        // thread times out it simply returns, and the process exits with the
        // blocked reader thread still inside read(2). Unblocking that thread
        // is unnecessary — nothing after the deadline can use its bytes.
        let (tx, rx) = channel::<(std::io::Result<usize>, Vec<u8>)>();
        std::thread::spawn(move || {
            let mut buf = Vec::with_capacity(MAX_HOOK_INPUT_BYTES + 1);
            let mut bounded = std::io::stdin()
                .lock()
                .take((MAX_HOOK_INPUT_BYTES + 1) as u64);
            let result = bounded.read_to_end(&mut buf);
            // The main thread may already have timed out; a failed send is
            // expected in that case and is ignored.
            let _ = tx.send((result, buf));
        });

        let (read_result, buf) = match rx.recv_timeout(HOOK_READ_TIMEOUT) {
            Ok(pair) => pair,
            Err(RecvTimeoutError::Timeout) => {
                // stdin sent no/partial bytes and stayed open: the deadline
                // expired. FALLBACK-OK: R16 — a stalled runner is a drop,
                // never a failure. Returning exits the process immediately;
                // the blocked reader thread dies with it.
                log_category("stdin read timed out");
                return ExitCode::SUCCESS;
            }
            Err(RecvTimeoutError::Disconnected) => {
                log_category("stdin read failed");
                return ExitCode::SUCCESS;
            }
        };
        if read_result.is_err() {
            log_category("stdin read failed");
            // FALLBACK-OK: R16 — a failed read is a drop, never a failure a
            // hook should propagate.
            return ExitCode::SUCCESS;
        }

        let input = match std::str::from_utf8(&buf) {
            Ok(input) => input,
            Err(_) => {
                log_category("dropped (invalid utf-8)");
                return ExitCode::SUCCESS;
            }
        };

        match parse_hook_input(input, ReceivedAt::now()) {
            ParseOutcome::Accepted(record) => {
                // Delivery needs a Tokio runtime; build it only after the
                // bounded read completed, so no runtime ever outlives or
                // waits on a blocked read.
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => {
                        log_category("async runtime unavailable");
                        // FALLBACK-OK: R16 — any inability to reach the
                        // listener is the harmless absent/unavailable case;
                        // the hook never fails.
                        return ExitCode::SUCCESS;
                    }
                };
                // T02 delivery is best-effort and bounded; every outcome is a
                // success from the hook's perspective (R16). Non-`Sent`
                // outcomes already log category-only via T02's own
                // `report_delivery`.
                let _ = rt.block_on(deliver(&record));
            }
            // T02 already logged the drop category; nothing further to emit.
            ParseOutcome::Dropped(_) => {}
        }
        ExitCode::SUCCESS
    }
}

/// Category-only stderr line: never payload values (R14).
fn log_category(message: &str) {
    eprintln!("dashboard claude-hook: {message}");
}
