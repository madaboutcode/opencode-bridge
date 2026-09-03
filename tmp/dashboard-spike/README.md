# Dashboard Spike

A throwaway TUI spike: 2-level squarified treemap of dummy sessions using ratatui + crossterm.

## Run

```bash
cargo run
```

From the `tmp/dashboard-spike/` directory.

## Keys

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit |
| `j` / `k` / arrows | Move selection |
| `[` | Window −5m |
| `]` | Window +5m |
| `w` | Reset window to 10m |
| `a` | Toggle show-all (W=∞, shows idle-only projects) |
| `Enter` | Zoom stub (not implemented) |

## Dummy Data

- **opencode-mcp**: 4 sessions (1 stalled, 1 thinking child, 1 doing, 1 idle)
- **web**: 1 thinking session
- **infra**: 5 sessions (2 doing, 2 waiting, 1 stalled)
- **idle-only**: 1 waiting session (only visible with `a` show-all)

Default window W=10m. Active sessions (updated within W) get weight=3, idle get weight=1.

## Build

```bash
cargo build
cargo test
```
