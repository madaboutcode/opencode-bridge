//! Tiny hand-rolled file logger (SPEC.md §... — deliberately no `tracing`
//! crate, matching the codebase's minimal-deps rule). One append-mode file
//! per bridge process at `<dir>/bridge-<pid>.log`, so concurrent bridges
//! never interleave and a past run's log survives for post-hoc debugging.
//!
//! Two levels serve the "cheap 80/20 by default, more on demand" goal:
//!   - INFO (default): lifecycle + every completion/notify DECISION +
//!     terminal `session.execution.*` events + errors. This is the 20% that
//!     explains 80% of incidents — in particular it records, for each
//!     tracked session, exactly when and why the bridge decided a turn was
//!     done, which is what you need to catch a premature completion.
//!   - DEBUG (`OPENCODE_MCP_LOG=debug`): adds the full per-frame SSE event
//!     stream (every `session.*` type + durable seq), so a "completed while
//!     the agent was still going" sequence is visible frame by frame.
//!
//! Logging must never take the bridge down: if the log dir/file can't be
//! opened we fall back to stderr only (FALLBACK-OK — observability is not a
//! correctness dependency).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
        }
    }
}

struct Logger {
    file: Option<Mutex<File>>,
    max_level: Level,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Resolve the log directory: `OPENCODE_MCP_LOG_DIR` override, else
/// `$HOME/.local/share/opencode-mcp/log` (sibling of opencode's own
/// `~/.local/share/opencode/log`, where operators already look).
fn log_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("OPENCODE_MCP_LOG_DIR") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".local/share/opencode-mcp/log"))
}

/// Reads `OPENCODE_MCP_LOG` — `debug` raises the ceiling to DEBUG, anything
/// else (or unset) stays at the INFO default.
fn max_level_from_env() -> Level {
    match std::env::var("OPENCODE_MCP_LOG").ok().as_deref() {
        Some("debug") | Some("DEBUG") => Level::Debug,
        _ => Level::Info,
    }
}

/// Opens the per-pid log file and installs the global logger. Idempotent —
/// a second call is ignored. Call once, early in `main`, before spawning the
/// SSE/sweep tasks so their first lines are captured. Returns the log path
/// (if a file was opened) for a one-line "logging to …" boot message.
pub fn init() -> Option<PathBuf> {
    let max_level = max_level_from_env();
    let mut opened_path = None;
    let file = log_dir().and_then(|dir| {
        if std::fs::create_dir_all(&dir).is_err() {
            return None;
        }
        let path = dir.join(format!("bridge-{}.log", std::process::id()));
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok()?;
        opened_path = Some(path.clone());
        Some(Mutex::new(f))
    });
    let _ = LOGGER.set(Logger { file, max_level });
    opened_path
}

pub fn debug_enabled() -> bool {
    LOGGER
        .get()
        .map(|l| l.max_level >= Level::Debug)
        .unwrap_or(false)
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Write one line. INFO and above are also echoed to stderr (an MCP stdio
/// server's stderr is free — stdout is the protocol channel — and Claude
/// Code surfaces it in its MCP logs); DEBUG goes to the file only, to keep
/// stderr quiet. Before `init()` (or if the file failed to open) everything
/// falls back to stderr so nothing is silently lost.
pub fn write(level: Level, component: &str, msg: &str) {
    let Some(logger) = LOGGER.get() else {
        // Pre-init: stderr fallback so early failures aren't swallowed.
        eprintln!("[bridge] {}: {msg}", component);
        return;
    };
    if level > logger.max_level {
        return;
    }
    let line = format!(
        "{} {} [{}] {}\n",
        now_millis(),
        level.label(),
        component,
        msg
    );
    if let Some(file) = &logger.file {
        if let Ok(mut f) = file.lock() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
    // Echo INFO+ to stderr (keeps the pre-existing stderr visibility);
    // DEBUG is file-only. If no file was opened, echo everything so the
    // operator still sees it.
    if level <= Level::Info || logger.file.is_none() {
        eprint!("[bridge] {}: {}", component, {
            // reuse the same body without the leading timestamp for stderr
            let mut s = msg.to_string();
            s.push('\n');
            s
        });
    }
}

#[macro_export]
macro_rules! linfo {
    ($comp:expr, $($arg:tt)*) => {
        $crate::log::write($crate::log::Level::Info, $comp, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! lwarn {
    ($comp:expr, $($arg:tt)*) => {
        $crate::log::write($crate::log::Level::Warn, $comp, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! lerror {
    ($comp:expr, $($arg:tt)*) => {
        $crate::log::write($crate::log::Level::Error, $comp, &format!($($arg)*))
    };
}

/// DEBUG is gated behind `debug_enabled()` so the `format!` cost is only
/// paid when debug logging is actually on (the default INFO run does no
/// per-frame string building).
#[macro_export]
macro_rules! ldebug {
    ($comp:expr, $($arg:tt)*) => {
        if $crate::log::debug_enabled() {
            $crate::log::write($crate::log::Level::Debug, $comp, &format!($($arg)*))
        }
    };
}
