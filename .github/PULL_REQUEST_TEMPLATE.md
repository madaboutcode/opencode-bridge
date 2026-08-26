## What

<!-- One paragraph. What's the change and why? -->

## Why

<!-- Link the issue, or describe the user-visible problem. -->

## How

<!-- Implementation notes worth flagging for review. Anything that touches
SPEC.md §7 (robustness) or §8 (correlation) — race guards, claim semantics,
the in-process registry as the only notify path — call out explicitly. -->

## Checklist

- [ ] `cargo build` is green.
- [ ] `cargo test` is green.
- [ ] `cargo clippy -- -D warnings` is green.
- [ ] `cargo fmt --check` is green.
- [ ] If you changed a tool schema or added a tool, `SPEC.md` §5 reflects it.
- [ ] If you touched any contract called out in `SPEC.md` §7 or §8, you said
      so above and called out the test that proves the new behavior.
