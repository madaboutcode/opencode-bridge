# T05 — Serde-backed Claude wire model

**Contract version** — 3 (advisor-adjudicated amendment; Review Frame v2
pending fresh implementation review)

**Reviewer binding** — fresh `luna` review

**Context** — Replace the duplicated hand-written envelope serializer and
decoder with serde-backed typed conversion without changing the shipped local
JSON protocol, validation bounds, error categories, privacy boundary, or hook
behavior.

**Boundaries** — owns only `crates/dashboard/Cargo.toml`, `Cargo.lock`,
`crates/dashboard/src/claude/hook.rs`, and
`crates/dashboard/src/claude/wire.rs`, including tests. The serde model may
derive on the existing typed envelope/event types and metadata enums.

The hook-side `parse_hook_input` extraction remains authoritative for R14:
allowlist selection, required-field checks, label checks, UTF-8 truncation,
`tool_input` object-to-compact-string conversion, and category-only dropped
input all remain in `hook.rs`. Serde must not parse raw hook payloads as a
replacement, bypass those checks, or reintroduce rejected fields downstream.

Must not change the fifteen variants, R14 field set, public typed fields,
protocol version, error enum/categories, socket behavior, state logic, or
acceptance of ignored extra keys.

**Delivery profile** — `tasks/2026-09-05-claude-dashboard-fable-fixes/delivery-profile.md`
version 2; task override: none.

**Skills to apply** — `software-design`, `code-quality`, and
`writing-unit-tests`.

## Exact Wire Contract

Serialization is compact JSON, with no spaces, followed by exactly one `\n`.
The key order is part of this compatibility contract:

- root: `protocol_version`, then `record`;
- record: `session_id`, `cwd`, `event`, `received_at`;
- event: `kind` first, then the fields in the variant order below.

Optional fields are omitted when `None`. Decoder accepts them absent or JSON
`null`, and accepts unknown extra keys at root, record, and event levels while
never retaining or exposing them. Strings use normal JSON escaping. The frame
size bound includes the trailing newline and is `MAX_ENVELOPE_BYTES`; a frame
over it is `OutOfBounds` on decode and `Oversized`/`OversizedEnvelope` through
the existing encode/parse APIs. Serialization must preserve the existing
category-only error behavior.

## Fifteen-Variant Compatibility Matrix

The `kind` tag is internally tagged and uses these exact snake-case values.
The listed fields are the complete emitted R14 fields and their order after
`kind`; optional fields are omitted, never emitted as null.

| variant | `kind` | fields in order | optional omission |
|---|---|---|---|
| SessionStart | `session_start` | `source`, `model` | both |
| UserPromptSubmit | `user_prompt_submit` | `prompt` | none |
| PreToolUse | `pre_tool_use` | `tool_name`, `tool_use_id`, `tool_input`, `agent_id`, `agent_type` | last two |
| PostToolUse | `post_tool_use` | `tool_name`, `tool_use_id`, `tool_input`, `tool_response`, `agent_id`, `agent_type` | last two |
| PostToolUseFailure | `post_tool_use_failure` | `tool_name`, `tool_use_id`, `tool_input`, `error`, `error_type`, `agent_id`, `agent_type` | last three |
| PermissionRequest | `permission_request` | `tool_name`, `tool_use_id`, `tool_input` | none |
| PermissionDenied | `permission_denied` | `tool_name`, `tool_use_id`, `denial_reason` | last |
| Elicitation | `elicitation` | `tool_use_id`, `server_name`, `elicitation_request` | none |
| ElicitationResult | `elicitation_result` | `tool_use_id`, `server_name`, `user_response` | none |
| Notification | `notification` | `notification_type`, `notification_message` | first |
| Stop | `stop` | `last_assistant_message`, `agent_id`, `agent_type` | last two |
| StopFailure | `stop_failure` | `error_type` | only field |
| SubagentStart | `subagent_start` | `agent_id`, `agent_type`, `agent_prompt` | middle |
| SubagentStop | `subagent_stop` | `agent_id`, `agent_type`, `last_assistant_message`, `stop_reason` | second and last |
| SessionEnd | `session_end` | `reason` | only field |

All R14 fields in the matrix must survive a serialize/decode round trip,
including `source`/`reason` closed-enum values and every optional combination.
`ReceivedAt` remains the unsigned epoch-millisecond number in the record.

## Rejection and Boundary Matrix

- malformed JSON, non-object root/record/event, missing required keys, wrong
  JSON types, invalid closed-enum values, and malformed known variants ->
  `DecodeError::Malformed`; duplicate keys retain serde_json::Value's current
  last-key-wins behavior at root, record, event, and `kind` objects. A valid
  last value is accepted; the resulting last value determines its category.
- present protocol version other than `1` -> `UnknownVersion`; missing or
  mistyped version -> `Malformed`;
- missing or non-string final `kind` -> `Malformed`; unknown or empty string
  final `kind` -> `UnknownEvent`, not `Malformed`;
- empty or over-bound session id/cwd, over-bound labels, invalid `received_at`
  outside the shared timestamp range, embedded additional newlines, or frame
  length over `MAX_ENVELOPE_BYTES` -> `OutOfBounds`;
- unknown root/record/event keys -> ignored, including values that look like
  transcript or raw hook payload fields;
- every rejection returns only the existing category. No raw JSON, rejected
  value, sensitive field, or serde error text may appear in `DecodeError`,
  hook drop logs, or typed state.

The bounded raw `kind` preflight is mandatory. It reads the final `kind` value
after serde_json's duplicate-key last-wins parsing, classifies missing or
non-string as `Malformed` and unknown/empty strings as `UnknownEvent`, then
allows only a known kind through typed serde and typed bounds validation. It
must not become a second per-event field mapping or cross the boundary into
typed state. Every serde failure maps to an existing category-only
`DecodeError`; raw serde error text never escapes.

## Acceptance — executable

1. Add only the direct serde dependency needed by the existing model and lock
   it. `cargo test -p dashboard` compiles with the declared dependency.
2. A raw JSON assertion verifies exact root/record/event key order, snake-case
   tags, compact formatting, omitted options, and exactly one trailing newline
   for representative events with present and absent options.
3. A table-driven test round-trips all fifteen variants with every R14 field
   and optional combination from the matrix, asserting typed equality.
4. Tests cover malformed JSON/shape/type, unknown-version, missing-version,
   missing/non-string kind, unknown/empty kind, bounds, embedded/multiple
   newline, oversized frame, and extra-key behavior with the exact category in
   the rejection matrix.
5. Duplicate-key tests prove serde_json's last-key-wins behavior at root,
   record, event, and kind objects: known last kind decodes, unknown last kind
   returns `UnknownEvent`, and duplicate typed fields use the final value.
6. Tests construct nested JSON at serde_json's unchanged default recursion
    boundary: 127 nested containers are accepted, while 128 are rejected as a
    category-only malformed failure. Do not raise, lower, or otherwise change
    this parser policy.
7. Tests prove hook-side extraction remains authoritative: raw hook
   `tool_input` is converted to compact text and bounded before serialization;
   truncated bounded fields, missing required fields, invalid labels, and
   unknown hook events retain existing `DropReason` outcomes. The serde wire
   path never accepts raw hook-only fields as typed R14 fields.
8. Tests prove rejected payload content and raw serde errors do not appear in
   error/debug output or logs, and no raw JSON reaches a typed envelope.
9. Existing hook ingress, wire, adapter, and state tests remain green; the
   newline and size checks remain inclusive of the newline.
10. `cargo test -p dashboard` and `cargo clippy -p dashboard --all-targets`
   pass.

## Review Frame

**Status** — advisor-adjudicated amendment · v2 · fresh implementation review
pending

**Context** — Replace hand-written Claude wire conversion with serde-backed conversion while preserving protocol bytes, validation, and privacy boundaries.

**Expectations** — Verify all fifteen variants and R14 fields, exact declaration-order compact JSON, tags/enums, omission/null, newline/inclusive bounds, last-wins duplicates, mandatory final-kind preflight, 127/128 recursion, extras, malformed/unknown categories, and hook-side extraction/truncation.

**Depth** — Fresh `luna`; use byte fixtures and the rejection matrix, including duplicate kind/typed fields and normalized last-wins decode. Run ingress/wire tests plus full dashboard test/clippy; no socket or state changes.
