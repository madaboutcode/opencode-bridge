# Dashboard — Visuals

What a session card shows, the three-state attention model, the still-open
question of chrome (borders vs. color), and the project/session naming
scheme. Source: `tasks/2026-09-01-opencode-dashboard.requirements.md`,
section 4 ("Visuals"), R6-R6.8, plus its Appendix (word lists) and three
Open Questions items carried forward below as `[REVIEW: ...]`.

## Look and chrome

- **R6** — The dashboard is styled as a dark-themed TUI: the Tokyo Night
  color palette, set assuming a monospace terminal font (JetBrains Mono is
  the reference font used when picking colors/spacing, though the dashboard
  cannot force which font the user's terminal actually renders with).
  Projects and sessions are shown as nested, padded, rounded-corner boxes —
  not flush/flat tiles packed edge to edge. The attention model is 3 states,
  not 4 (see R6.7 below; a fourth state, `stalled`, was considered and
  dropped — no wire signal exists to detect it).

  **Superseded in part:** the original wording of this requirement also
  specified a border color per project on every card. That default is
  superseded by `layout.md` R5.11 (forward reference): a project's accent
  color is applied to one small element only (the project name/tag), never
  to a whole card or a whole project border. This number (R6) is kept and
  the border-per-project clause is void; see R5.11 for the current rule and
  R6.2 below for what (if anything) uses a border at all.

  Scenario: Given the dashboard is rendered in a terminal with 24-bit color
  support, when you look at its projects and sessions, then they render as
  padded, rounded-corner nested boxes in the Tokyo Night palette — not flat
  tiles butted against each other, and no card carries a full border in its
  own project's accent color (see R5.11).

- **R6.1** — Attention state (running / needs-you / idle, R6.7) is shown
  through color, glyph, and sort order — never through how much screen area
  a tile gets. Tile size is driven purely by content demand (`layout.md`
  R5.1/R5.2, forward reference); a `needs-you` tile is not drawn bigger than
  a `running` one just because it's more urgent.

  Scenario: Given one `running` session and one `needs-you` session with
  identical content demand, when the layout is computed, then both tiles
  are the same size; the `needs-you` tile is distinguished only by its
  color/glyph/badge and by sorting earlier (`layout.md` R5.6, forward
  reference), never by being drawn larger.

- **R6.2** — [REVIEW: OPEN — this is the live prototype axis, not decided]
  Whether cards/projects use borders at all, and if so which ones, is
  undecided. Three options are under live test in a real terminal render,
  not a mockup:

  - **(A)** Border on both the project box and each session card.
  - **(B)** No borders anywhere; grouping is shown by a color stripe plus a
    header line only.
  - **(C)** Border on the project box only; cards inside it are plain
    (unbordered).

  Whichever wins must still avoid empty container waste (no half-empty box
  around a single-session project) and must still make grouping
  unambiguous at narrow terminal widths. This spec does not pick one — do
  not read anything above as having already decided in favor of any option.
  R6.8 below explains how nickname placement depends on whichever option is
  eventually chosen.

  Scenario: Given the chrome prototype has not yet been decided, when you
  read this spec, then it names three candidate options (A/B/C) and asserts
  none of them as final; whichever the running prototype settles on gets
  written back into this line, replacing this OPEN marker, without changing
  the requirement number.

## Card content

- **R6.3** — Each session card (a card is drawn for a tile with `running`,
  `needs-you`, or `idle`-shown-as-context status; overflow/idle-summary
  chips are a separate, single-line compact form — `layout.md` R5.2/R5.6,
  forward reference — and don't get a full card) shows exactly 3 lines:

  1. **Title/handle line** — the session's nickname (R6.8 below) plus its
     wire title (the harness's own session title/description). Whether
     these sit in a border title or share a plain text line depends on
     which chrome option R6.2 above settles on; that arrangement is not
     decided yet, but the content — nickname + wire title, both present —
     is.
  2. **Status + elapsed line** — the attention state word plus how long
     it's been in that state, per R6.7 below, e.g. `needs-you · question`,
     `needs-you · 14m`, `running · 2m ago`.
  3. **Current action line** — a single, truncated line describing the
     session's most recent tool activity, e.g. `editing: foo.rs` or
     `npm test`. This line holds its last value until new activity
     replaces it. The exact rule for turning a given tool call into this
     rendered text is not a card-visual concern — it's specified in
     `client.md` R6.5 (forward reference); this file does not repeat that
     mapping.

  No 4th line (cost/tokens were considered and cut — see `overview.md` R10,
  forward reference, for the related non-goal).

  Scenario: Given a session named "Apollo" with wire title "fix flaky
  test", status `running` for 2 minutes, and its last tool call editing
  `src/foo.rs`, when its card renders, then line 1 shows the nickname and
  wire title, line 2 reads `running · 2m ago`, and line 3 reads
  `editing: foo.rs`; when the session's next tool call is a shell command
  instead, then line 3 updates to reflect that new action, replacing the
  old line, while lines 1 and 2 update independently per their own rules.

## Attention model

- **R6.7** — **[CONFIRMED FINAL — the 3-state model itself is not open;
  only the question-badge phrase list below is.]** Every session is in
  exactly one of three states, named for who is waiting on whom:

  - **running** — a turn is in progress. The card shows its live current
    action line (R6.3 line 3). Not urgent; sorts after `needs-you`.
  - **needs-you** — the harness's turn has ended and it's waiting on the
    user. Two visual sub-states share this one underlying status:
    - **question badge** — the ended turn's final assistant message looks
      like a question. [REVIEW: OPEN — the exact detection rule/phrase
      list is unspecified. The only settled part is that it's checked once,
      at the moment a session transitions into `needs-you`, not on every
      poll — not the literal wording, which the source material calls
      "ends in `?`, or matches a short phrase list — 'which', 'should I',
      'do you want', etc." and explicitly leaves as a placeholder, not a
      spec.] Shown with a distinct badge/color and sorted first among all
      sessions (`layout.md` R5.6, forward reference).
    - **needs-you `Nm`** — ended turn, question heuristic didn't match.
      Shows elapsed time since the turn ended; the number itself carries
      the urgency ordering (longest-waiting sorts earlier), with no
      separate "stalled" tier.
  - **idle** — outside the active window `W` (`overview.md` R3.2, forward
    reference). Shown only as context inside a project that has at least
    one `running`/`needs-you` session, or collapsed into an overflow chip.

  Scenario: Given a session's turn just ended and its final assistant
  message ends in "?", when the dashboard evaluates its state, then it
  shows `needs-you` with the question badge and sorts it ahead of every
  other non-question session; given instead a session whose turn ended 14
  minutes ago with no question match, when the dashboard evaluates it, then
  it shows `needs-you · 14m` with no badge, ordered by that elapsed time
  among other plain `needs-you` sessions.

## Session and project naming

- **R6.8** — Every session card's identity is a single word, drawn from a
  categorized, curated wordlist (Greek myth, detective fiction, Norse myth,
  etc. — full list in the Appendix below). Words are never combined — no
  adjective+noun pairing, one plain word per card. This reverses an earlier
  scheme; see "Reversal," below.

  **Two-layer claim scheme.** Both layers work the same way: prefer a
  hash-derived slot, and if that slot is taken, move deterministically
  (never randomly) to another one.

  1. **Project → category.** Each currently-live project (a project with at
     least one live session) claims exactly one category, exclusively, for
     as long as it has at least one live session.
     - Preferred category = `hash(project identity) mod (number of
       categories)`. Project identity is `client.md` R1.6's definition
       (forward reference): canonical git-repository toplevel path, or the
       working directory itself if there's no repo.
     - If the preferred category is already claimed by a different
       currently-live project, scan forward in a fixed, deterministic order
       (e.g. next category index, wrapping around) to the next unclaimed
       one.
     - Released when the project has no live sessions left.

  2. **Session → word.** Within its project's claimed category, each
     session claims exactly one word (its "seat").
     - Preferred word = `hash(harness kind, harness-native session id) mod
       (word count in that category)` — the same session-identity pair
       defined in `client.md` R1.5 (forward reference). The hash scatters
       picks across the whole category list rather than walking it in
       order (`word[0]`, `word[1]`, ...), so real usage doesn't keep
       landing on the same first couple of names for every project.
     - If the preferred word is already held by a live sibling session in
       the same project, or is in cooldown (below), probe forward using a
       second, deterministic hash-derived stride (not true randomness)
       until landing on a word that is both free and off cooldown.

  **Recycling (cooldown).** When a session ends, its word does not return
  to the available pool immediately. It enters cooldown: it isn't claimable
  again until enough *other* distinct words in that category have been
  claimed since it was freed (tracked as a small per-word "last freed"
  counter compared against a per-category "claims made since" counter —
  bounded state, no full history log kept). This exists so a name doesn't
  start pointing at a different session moments after the session that
  used to hold it ends.

  **Guarantees — both hard, not just low-probability:**
  - No two sessions in the same live project ever show the same name.
  - No two live projects ever show the same name.

  Both guarantees hold only under two capacity/curation assumptions, and
  neither is checked at runtime:
  (a) the number of live projects never exceeds the number of curated
  categories;
  (b) no single word is duplicated across two different category files.
  Both are the responsibility of whoever curates the wordlists and category
  count, not something the running dashboard verifies — see "Capacity edge
  case" below for what happens if either is violated.

  **Reversal of the 2026-09-01 decision.** This scheme replaces an earlier
  one, and the replacement is a reversal, not an incremental tweak. The
  2026-09-01 rule was: a deterministic adjective+noun nickname (both words
  ≤6 characters), hashed from the session identity alone, picking
  determinism over on-screen uniqueness — collisions were accepted (two
  cards could show the same name, still distinguishable by project/title/
  status/action) and render-time collision-fixing (e.g. appending hash
  characters) was explicitly forbidden. That rule is retracted as of
  2026-09-02, for two reasons:
  1. **Birthday-paradox math.** Adjective+noun gave a combinatorially large
     name space, so collisions were genuinely rare. A single word drawn
     from one category's list (10-14 words, see Appendix) is a much
     smaller space; at real session counts (`overview.md` R5.8, forward
     reference: ~8 sessions across ~4 projects), same-project collisions
     become common, not rare, at that scale.
  2. **A collision-free scheme can't be a pure function of a session's own
     id.** If two sessions prefer the same word, whichever one doesn't get
     it must show something not explained by its own id alone — it has to
     depend on arrival history (who else was live, and when). The old
     rule's other stated virtue — "pure function of session ID alone, no
     storage, no cache" — is dropped along with it; the two are the same
     tradeoff seen from two sides, and neither survives once uniqueness is
     required.

  **Coupling to R1.7 (not yet resolved there).** Both claim layers
  (project→category, session→word) must release their claim according to
  `client.md` R1.7's eventual staleness rule (forward reference) — not only
  when an explicit "this session/project is gone" message arrives.
  Otherwise a session or project that goes silently stale without a
  tombstone keeps its claim (and its seat) forever, leaking it out of the
  pool permanently. R1.7's exact threshold and treatment aren't decided
  yet; this coupling is recorded here so it's visible once R1.7 is
  designed, rather than discovered later.

  [REVIEW: OPEN — capacity edge case] What happens when a capacity
  assumption above breaks — more live projects than curated categories, or
  more live sessions in one project than that category has words — is not
  specified. A numeric-suffix fallback (e.g. appending a number to a reused
  word) is noted as acceptable, since this is a genuine capacity condition
  rather than the common case the scheme is built around, but the exact
  behavior isn't decided.

  **Word lists.** 10 categories are curated and frozen as of 2026-09-02
  (full content in the Appendix below). Each category is meant to become
  its own file (`wordlists/<category>.txt` or equivalent) once M3 scaffolds
  the dashboard crate — the Appendix holds all ten together here only until
  that happens, per this requirement's "one category per file" rule. Both
  the category list and each category's own word list are frozen once
  approved: changing either after V1 ships reshuffles who holds which seat,
  because both claim layers depend on list order and length through the
  hash-mod-count math above.

  **Placement.** Where the nickname appears on the card is a consequence of
  R6.2's chrome outcome, not an independent choice: if R6.2 settles on
  option A or C (a border exists somewhere), the nickname goes in that
  border's title; if it settles on option B (no borders), the nickname
  shares card line 1 as `nickname · title` (R6.3 above). The wire title
  itself is shown as descriptive content either way.

  Scenario: Given two projects whose identities hash to the same preferred
  category index, and project P1 is already live holding that category,
  when project P2 becomes live and computes the same preferred category,
  then P2's claim scans forward to the next unclaimed category instead, and
  P1's category is unaffected.

  Scenario: Given a category whose word list includes "Apollo", and a
  session holding "Apollo" in project P1 just ended, when a new session in
  P1 starts and its preferred-word hash also lands on "Apollo", then it
  does not receive "Apollo" — that word is in cooldown — and instead probes
  forward to the next word that is both free and off cooldown; "Apollo"
  only becomes claimable again once enough other distinct words in that
  category have since been claimed and freed.

  Scenario: Given a project currently has two live sessions with different
  harness-native ids, when both have completed their word claims, then
  they hold two different words within that project's category — never the
  same word, per the hard per-project guarantee above.

## Appendix — R6.8 word lists (frozen, 2026-09-02)

Copied verbatim from the requirements doc. 10 categories, ≤10 chars/word,
no word repeated across categories. Held here as one file until M3
scaffolds the dashboard crate and promotes each list to its own file
(`wordlists/<category>.txt` or equivalent), per R6.8's "one category per
file" rule above.

- **Greek myth:** Zeus, Hera, Apollo, Athena, Hermes, Ares, Artemis, Hades, Persephone, Poseidon, Demeter, Dionysus, Hestia, Nemesis
- **Norse myth:** Odin, Thor, Loki, Freya, Baldur, Heimdall, Frigg, Tyr, Skadi, Njord
- **Detective fiction:** Holmes, Watson, Poirot, Marple, Columbo, Marlowe, Maigret, Dupin, Wimsey, Cadfael
- **Sci-fi:** Spock, Kirk, Ripley, Trinity, Picard, Sarek, Neo, Solo, Sarah, Deckard, Rorschach
- **Classical composers:** Bach, Mozart, Chopin, Handel, Vivaldi, Brahms, Liszt, Verdi, Dvorak, Sibelius
- **Chess:** Fischer, Carlsen, Karpov, Tal, Anand, Nakamura, Nepo, Ding, Judit, Botvinnik
- **Mollywood:** Ganga, Nagavalli, Dasan, Vijayan, Mannar, Velu, Pazhassi, Meenakshi, Kunjikka, Bhaskaran
- **Supervillains:** Thanos, Joker, Venom, Ultron, Magneto, Vader, Bane, Sauron, Voldemort, Cruella
- **Sidekicks:** Robin, Samwise, Ron, Pippin, Gimli, Alfred, Sancho, Tonto, Donkey, Chewie
- **Cricket legends:** Sachin, Kohli, Dhoni, Gavaskar, Kapil, Dravid, Sehwag, Ganguly, Bumrah, Ashwin
