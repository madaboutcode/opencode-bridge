# Implementation Plan: Catalog returns scored plain-text with agents-first ordering

## Purpose

Change `opencode_catalog` from returning JSON (`{"models":[...], "matched": N}` / `{"agents":[...], "count": N}`) to returning plain text. The text output has agents sorted above models, each entry scored by query relevance. The tool description is updated to guide the consumer toward using agents over raw models.

## File Tree

```
src/tools.rs
  ── catalog_models()       → rewritten: returns plain text, scored, sorted
  ── catalog_agents()       → rewritten: returns plain text, scored, sorted
  ── catalog()              → updated: new kind=all (no kind = agents+models merged)
  ── definitions()          → catalog description updated to encourage agents-first
  ── compute_score()        → new: relevance scorer for a searchable text + query
  ── tests                  → existing tests updated; new tests for scoring, text format
SPEC.md
  ── opencode_catalog section → updated to reflect text output shape
```

## Data / Scoring Model

### Score formula

```
score = (is_agent ? 50 : 0) + relevance
relevance = normalized BM25 score in the range 0..50
```

- If query is empty/whitespace-only: relevance = 0 (so agents all get 50, models get 0)
- BM25 uses case-insensitive whitespace tokens; a token containing a query term counts as a match, preserving substring search for handles such as `deepseek-v4`.
- Candidates still require every space-separated query term to match.
- Agent bonus of 50 ensures agents always sort above models at equal relevance

### Searchable text per entry

- **Agents**: `"{name} {description}"`
- **Models**: `"{providerID}/{id} {name}"`

### Sorting

Primary: score descending. Secondary: name alphabetically (case-insensitive) for ties.

## Output format

### kind=agents

```
<name>  <model-handle or "—">  <truncated description>  <score>
```

The `providerID/id` model column is truncated to 18 characters and the
description column to 30 characters, each with `…` if truncated. Names and
columns are right-padded to fixed widths.

Example:

```
coder         opencode-go/gpt-5.6-luna   Write production code               92
deepseek      opencode-go/deepseek-v4-pro Strong reasoning, coding            88
deepseek-flash opencode-go/deepseek-v4-flash Fast and capable                  85
clerk         opencode-go/deepseek-v4-flash Fast cheap reliable                70
```

### kind=models

```
<providerID/id>  <name>  <score>
```

Example:

```
deepseek/deepseek-v4-flash    DeepSeek V4 Flash       50
deepseek/deepseek-v4-pro      DeepSeek V4 Pro         50
deepinfra/deepseek-ai/...     DeepSeek V4 Flash       40
```

### kind=all (or omitted)

Both sections merged with a header line:

```
── Agents ──
coder         opencode-go/gpt-5.6-luna   Write production code               92
deepseek      opencode-go/deepseek-v4-pro Strong reasoning, coding            88

── Models ──
deepseek/deepseek-v4-flash    DeepSeek V4 Flash       50
```

If either section is empty after filtering, omit that header.

### Metadata

Instead of JSON `matched`/`returned`/`truncated` fields, append a one-line footer when truncated:

```
(200 of 312 shown — refine your query to narrow results)
```

## Kind behavior

| `kind` | Returns |
|--------|---------|
| `"agents"` | Agents only, scored, sorted |
| `"models"` | Models only, scored, sorted |
| `"all"` | Both sections merged, agents first |
| omitted | Same as `"all"` |
| unrecognized | Error message |

## Pre-decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| kind=all vs separate call | kind=all with kind-optional | Backward-compatible; kind omitted = merged view is the new default |
| Cap behavior | 200 total entries; agents get priority if capped | Same cap as before but shared across both sections |
| Column truncation | Name 16, model 18, description 30, append `…` if truncated | Keeps columns aligned and readable |
| Score display | Right-aligned 3-digit number | Wastes fewer tokens than left-aligned or padded |
| Empty results | "No agents found." / "No models found." | Clean, informative |
| Error on unrecognized kind | Return error string | Act like a strict CLI — catch typos early |

## Testing strategy

- Update existing test `definitions_lists_four_tools_with_expected_names` (no change needed — names same)
- Update `matches_query` tests (no change — function unchanged)
- Add BM25 unit tests: empty query, shorter-match preference, partial/non-match ranking, agent bonus
- Add test for text format: verify output contains expected headers, agents before models
- Integration via existing pipe harness (README.md) — manual

## Acceptance criteria

- [x] `opencode_catalog kind=agents` returns plain text, one agent per line, scored, sorted desc
- [x] `opencode_catalog kind=models` returns plain text, one model per line, scored, sorted desc
- [x] `opencode_catalog kind=all` returns both sections with "── Agents ──" / "── Models ──" headers, agents on top
- [x] `opencode_catalog` (no kind) same as `kind=all`
- [x] Agents always score >= 50; models get no bonus; equal-score agents sort alpha
- [x] Truncation footer appears when results exceed cap
- [x] Empty result message when nothing matches
- [x] Description updated in `definitions()` to encourage agents-first usage
- [x] Existing tests still pass
- [x] SPEC.md updated
