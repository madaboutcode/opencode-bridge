//! The two-layer claim scheme itself — `docs/specs/dashboard/visuals.md`
//! R6.8, restated for implementation in T10's contract. Pure logic: no I/O,
//! no clock reads (creation time is handed in by the caller, from T09's
//! `SessionSnapshot::created_at`), no TUI. This is the "core-owned
//! allocation state" R6.8 describes — it just doesn't know yet who its
//! caller (T12) will be.
//!
//! # The two layers
//!
//! - **Project → category.** Each live project (>=1 live session) holds
//!   exactly one category, exclusively, for as long as it has a live
//!   session. Preferred category is a deterministic hash of the project's
//!   identity; a conflict probes forward through the fixed category order
//!   for the next unclaimed one.
//! - **Session → word.** Each session holds exactly one word within its
//!   project's category. Preferred word is a deterministic hash of the
//!   session identity tuple; a conflict probes forward through the
//!   category's word list for the next word that is both free and off
//!   cooldown.
//!
//! # Claim order
//!
//! [`NamingClaimMap::claim_batch`] sorts its whole input by creation time
//! before resolving anything, so the same live set produces the same claims
//! regardless of what order the caller's `Vec` happened to be built in
//! (REST pagination order, SSE arrival order, ...). A project's effective
//! position falls out of this for free: the first time a not-yet-claimed
//! project is encountered while walking the time-sorted session list *is*
//! that project's earliest live session, because every later session of the
//! same project sorts after it.
//!
//! # Cooldown — implementation choice, documented here
//!
//! `visuals.md` R6.8 requires *some* cooldown ("enough *other* distinct
//! words in that category must be claimed first") but doesn't pin the
//! count; the contract calls this a real implementation judgment call. This
//! module uses **N = 2**: a freed word becomes reclaimable once 2 other
//! distinct words in the same category have since been newly claimed.
//! Reasoning: categories run as small as 10 words (Norse myth, Detective
//! fiction, ...); a cooldown count much higher than 2-3 risks locking up a
//! small category under realistic churn (`overview.md` R5.8's design center
//! is ~2 sessions/project, so a category rarely has more than a couple of
//! slots turning over at once). N = 2 is enough that a word doesn't visibly
//! jump straight back to a different session the moment the old one ends,
//! without needing much of the list to cycle first.
//!
//! # Capacity edge case — implementation choice, documented here
//!
//! Both hard guarantees assume live project count never exceeds the 10
//! curated categories, and no session count within one project ever exceeds
//! its category's word count. Neither is checked at runtime; the contract
//! requires only that violating them degrades visibly instead of panicking
//! or silently breaking a guarantee. This module's choice:
//!
//! - **Category overflow** (11th live project, no free category): the
//!   project attaches to its preferred category anyway, *sharing* it with
//!   whichever project already holds it. [`CategoryAssignment::shared`] is
//!   `true` when this happens, so a caller can render it distinctly. Word
//!   claims stay category-scoped even when shared, so two sessions from the
//!   two co-holding projects still never receive the same word — sharing
//!   degrades the "no two live projects show the same name" guarantee
//!   visibly and on purpose (that guarantee is the one the spec says can't
//!   hold past the capacity assumption), but it does not touch the other,
//!   per-project guarantee.
//! - **Word overflow** (more live sessions in one project than its
//!   category has words, including the case where every remaining word
//!   happens to be in cooldown): the session gets a numeric-suffixed
//!   variant of its preferred word (`"Apollo-2"`) instead of a bare word.
//!   The suffix counter is scoped to `(category, word)` and always picks
//!   the smallest unused value, so suffixed names never collide with each
//!   other or with a live bare claim — the per-project uniqueness guarantee
//!   holds even in this branch, it just stops being a single plain word.

use std::collections::{HashMap, HashSet};

use crate::naming::wordlist::CATEGORIES;
use crate::snapshot::{ProjectId, SessionId, Timestamp};

/// How many other distinct words in a category must be newly claimed
/// before a word freed by an ended session becomes reclaimable again. See
/// the module doc's "Cooldown" section for why 2.
const COOLDOWN_OTHER_WORDS: usize = 2;

/// One live session as input to [`NamingClaimMap::claim_batch`].
#[derive(Debug, Clone)]
pub struct LiveSession {
    pub project_id: ProjectId,
    pub session_id: SessionId,
    pub created_at: Timestamp,
}

/// A project's resolved category. Returned alongside each session's
/// nickname so a caller doesn't have to look it up separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryAssignment {
    pub category_index: usize,
    pub category_name: &'static str,
    /// `true` when more live projects exist than curated categories and
    /// this category is currently held by more than one of them — the
    /// documented capacity-overflow degrade. See the module doc.
    pub shared: bool,
}

/// A session's resolved nickname: its project's category plus its own word
/// within that category, with an optional degrade suffix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionNickname {
    pub category: CategoryAssignment,
    pub word: &'static str,
    /// `Some(n)` only on the word-overflow degrade path (see module doc);
    /// `None` for every ordinary claim.
    pub suffix: Option<u32>,
}

impl SessionNickname {
    /// The word as it should be displayed: the bare word normally, or
    /// `"{word}-{n}"` on the degrade path. Rendering (T11) is expected to
    /// call this rather than reach into `word`/`suffix` separately.
    pub fn display_word(&self) -> String {
        match self.suffix {
            Some(n) => format!("{}-{n}", self.word),
            None => self.word.to_string(),
        }
    }
}

/// Where a session's word claim lives inside its category's bookkeeping —
/// the ordinary word-list slot, or a synthetic overflow slot. Needed so
/// `release_session` knows which table to clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordSlot {
    Normal(usize),
    Overflow(usize, u32),
}

/// One category's live claim state. Lives independently of which
/// project(s) currently hold the category, so cooldown bookkeeping for a
/// word survives a project's claim ending and a different project taking
/// the category over.
#[derive(Default)]
struct CategoryState {
    /// Live projects currently holding this category. Length 1 in the
    /// common case; length >1 only under the documented overflow-sharing
    /// degrade.
    holders: Vec<ProjectId>,
    /// `word_holder[i]` is the session currently holding word `i`, if any.
    word_holder: Vec<Option<SessionId>>,
    /// Freed word index -> distinct word indices newly claimed since it was
    /// freed. A word is off cooldown once this set reaches
    /// `COOLDOWN_OTHER_WORDS`; entries are removed once that happens.
    cooldown: HashMap<usize, HashSet<usize>>,
    /// Overflow (degrade-path) claims: `(word_index, suffix) -> session`.
    overflow_holder: HashMap<(usize, u32), SessionId>,
}

impl CategoryState {
    fn new(word_count: usize) -> Self {
        Self {
            holders: Vec::new(),
            word_holder: vec![None; word_count],
            cooldown: HashMap::new(),
            overflow_holder: HashMap::new(),
        }
    }
}

/// Where a live session's claim is recorded, for `release_session` to find
/// without a linear scan.
struct SessionLocation {
    project_id: ProjectId,
    category_index: usize,
    slot: WordSlot,
}

/// The claim-map itself. Owns all naming state for every currently-live
/// project and session; nothing outside this type reads or writes it.
#[derive(Default)]
pub struct NamingClaimMap {
    categories: Vec<CategoryState>,
    project_category: HashMap<ProjectId, usize>,
    project_live_sessions: HashMap<ProjectId, usize>,
    session_location: HashMap<SessionId, SessionLocation>,
}

impl NamingClaimMap {
    pub fn new() -> Self {
        Self {
            categories: CATEGORIES
                .iter()
                .map(|c| CategoryState::new(c.words.len()))
                .collect(),
            project_category: HashMap::new(),
            project_live_sessions: HashMap::new(),
            session_location: HashMap::new(),
        }
    }

    /// Resolves a batch of newly-live sessions, ordered by actual creation
    /// time regardless of the order they appear in `sessions` — see the
    /// module doc's "Claim order" section. Returns each session's assigned
    /// nickname.
    pub fn claim_batch(
        &mut self,
        mut sessions: Vec<LiveSession>,
    ) -> HashMap<SessionId, SessionNickname> {
        sessions.sort_by(|a, b| {
            a.created_at
                .epoch_millis()
                .cmp(&b.created_at.epoch_millis())
                .then_with(|| a.session_id.harness.0.cmp(b.session_id.harness.0))
                .then_with(|| a.session_id.native_id.cmp(&b.session_id.native_id))
                .then_with(|| a.project_id.as_path().cmp(b.project_id.as_path()))
        });

        let mut results = HashMap::with_capacity(sessions.len());
        for live in sessions {
            let nickname = self.claim_one(&live.project_id, &live.session_id);
            results.insert(live.session_id, nickname);
        }
        results
    }

    /// Claims a single newly-live session. Equivalent to calling
    /// `claim_batch` with one element; provided for the steady-state case
    /// (sessions discovered one at a time after startup, where real time
    /// already advances monotonically so no batching is needed).
    pub fn claim_session(
        &mut self,
        project_id: &ProjectId,
        session_id: &SessionId,
        _created_at: Timestamp,
    ) -> SessionNickname {
        self.claim_one(project_id, session_id)
    }

    fn claim_one(&mut self, project_id: &ProjectId, session_id: &SessionId) -> SessionNickname {
        let category_index = self.category_for(project_id);
        *self
            .project_live_sessions
            .entry(project_id.clone())
            .or_insert(0) += 1;

        let (word_index, slot, suffix) = self.claim_word(category_index, project_id, session_id);

        self.session_location.insert(
            session_id.clone(),
            SessionLocation {
                project_id: project_id.clone(),
                category_index,
                slot,
            },
        );

        let category = &CATEGORIES[category_index];
        SessionNickname {
            category: CategoryAssignment {
                category_index,
                category_name: category.name,
                shared: self.categories[category_index].holders.len() > 1,
            },
            word: category.words[word_index],
            suffix,
        }
    }

    /// Resolves (claiming if necessary) `project_id`'s category index.
    fn category_for(&mut self, project_id: &ProjectId) -> usize {
        if let Some(&index) = self.project_category.get(project_id) {
            return index;
        }

        let n = self.categories.len();
        let preferred = preferred_index(project_id.as_path().as_os_str().as_encoded_bytes(), n);

        let mut chosen = None;
        for offset in 0..n {
            let idx = (preferred + offset) % n;
            if self.categories[idx].holders.is_empty() {
                chosen = Some(idx);
                break;
            }
        }
        // Capacity-overflow degrade: every category already has a live
        // holder. Share the preferred one rather than fail — see module
        // doc's "Capacity edge case" section.
        let index = chosen.unwrap_or(preferred);

        self.categories[index].holders.push(project_id.clone());
        self.project_category.insert(project_id.clone(), index);
        index
    }

    /// Claims a word for `session_id` within `category_index`, on behalf of
    /// `project_id`. Returns the word index, the slot it was recorded in
    /// (for release bookkeeping), and the degrade suffix if the overflow
    /// path was used.
    fn claim_word(
        &mut self,
        category_index: usize,
        _project_id: &ProjectId,
        session_id: &SessionId,
    ) -> (usize, WordSlot, Option<u32>) {
        let word_count = CATEGORIES[category_index].words.len();
        let preferred = preferred_index(session_id.native_id.as_bytes(), word_count);

        let state = &mut self.categories[category_index];
        let mut chosen = None;
        for offset in 0..word_count {
            let idx = (preferred + offset) % word_count;
            if state.word_holder[idx].is_none() && !state.cooldown.contains_key(&idx) {
                chosen = Some(idx);
                break;
            }
        }

        if let Some(idx) = chosen {
            state.word_holder[idx] = Some(session_id.clone());
            note_new_claim(state, idx);
            return (idx, WordSlot::Normal(idx), None);
        }

        // Word-overflow degrade (real capacity overflow, or every
        // remaining word is transiently in cooldown): smallest unused
        // suffix for the preferred word. See module doc.
        let mut suffix = 2u32;
        while state.overflow_holder.contains_key(&(preferred, suffix)) {
            suffix += 1;
        }
        state
            .overflow_holder
            .insert((preferred, suffix), session_id.clone());
        (
            preferred,
            WordSlot::Overflow(preferred, suffix),
            Some(suffix),
        )
    }

    /// Releases `session_id` per T09's tombstone signal
    /// (`adapter::SessionEvent::Gone`) — wired directly to that signal, not
    /// to any staleness-threshold trigger (contract explicitly forbids
    /// inventing one here). Frees the session's word (into cooldown, for a
    /// normal claim) and, if this was the project's last live session,
    /// releases the project's category too.
    pub fn release_session(&mut self, session_id: &SessionId) {
        let Some(location) = self.session_location.remove(session_id) else {
            return;
        };
        let SessionLocation {
            project_id,
            category_index,
            slot,
        } = location;

        {
            let state = &mut self.categories[category_index];
            match slot {
                WordSlot::Normal(idx) => {
                    state.word_holder[idx] = None;
                    state.cooldown.insert(idx, HashSet::new());
                }
                WordSlot::Overflow(idx, suffix) => {
                    // Synthetic overflow slots aren't part of the curated
                    // pool; they free immediately, no cooldown.
                    state.overflow_holder.remove(&(idx, suffix));
                }
            }
        }

        let remaining = self
            .project_live_sessions
            .get_mut(&project_id)
            .map(|count| {
                *count -= 1;
                *count
            })
            .unwrap_or(0);

        if remaining == 0 {
            self.project_live_sessions.remove(&project_id);
            self.project_category.remove(&project_id);
            let state = &mut self.categories[category_index];
            state.holders.retain(|p| p != &project_id);
            if state.holders.is_empty() {
                // Category fully vacated: reset it clean for whoever
                // claims it next, per R6.8 ("released when the project has
                // no live sessions left") — no cooldown carries over once
                // nobody is drawing from the category at all.
                let word_count = state.word_holder.len();
                *state = CategoryState::new(word_count);
            }
        }
    }

    pub fn nickname_of(&self, session_id: &SessionId) -> Option<SessionNickname> {
        let location = self.session_location.get(session_id)?;
        let category = &CATEGORIES[location.category_index];
        let (word_index, suffix) = match location.slot {
            WordSlot::Normal(idx) => (idx, None),
            WordSlot::Overflow(idx, s) => (idx, Some(s)),
        };
        Some(SessionNickname {
            category: CategoryAssignment {
                category_index: location.category_index,
                category_name: category.name,
                shared: self.categories[location.category_index].holders.len() > 1,
            },
            word: category.words[word_index],
            suffix,
        })
    }

    pub fn category_of(&self, project_id: &ProjectId) -> Option<CategoryAssignment> {
        let &index = self.project_category.get(project_id)?;
        Some(CategoryAssignment {
            category_index: index,
            category_name: CATEGORIES[index].name,
            shared: self.categories[index].holders.len() > 1,
        })
    }
}

/// Records that `claimed_idx` was just newly claimed in `state`, advancing
/// every word currently in cooldown one step closer to release, and
/// releasing any that just crossed `COOLDOWN_OTHER_WORDS`.
fn note_new_claim(state: &mut CategoryState, claimed_idx: usize) {
    let mut ready = Vec::new();
    for (&frozen_idx, others) in state.cooldown.iter_mut() {
        others.insert(claimed_idx);
        if others.len() >= COOLDOWN_OTHER_WORDS {
            ready.push(frozen_idx);
        }
    }
    for idx in ready {
        state.cooldown.remove(&idx);
    }
}

/// Deterministic preferred-index hash: FNV-1a over the identity bytes,
/// modulo `len`. Not `std`'s `DefaultHasher` on purpose — that hasher's
/// algorithm is explicitly documented as an implementation detail that can
/// change between Rust releases, which would break the "same live set
/// produces the same names across two restarts" guarantee the moment the
/// toolchain changes. FNV-1a is fixed, simple, and scatters small inputs
/// well enough to satisfy R6.8's "scattered across the whole list" note.
fn preferred_index(bytes: &[u8], len: usize) -> usize {
    debug_assert!(len > 0, "category/word lists are never empty");
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash % len as u64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::HarnessKind;
    use std::path::PathBuf;

    const KIND: HarnessKind = HarnessKind("test");

    fn project(path: &str) -> ProjectId {
        ProjectId::from_canonical(PathBuf::from(path))
    }

    fn session(native_id: &str) -> SessionId {
        SessionId::new(KIND, native_id)
    }

    fn live(project_path: &str, session_native_id: &str, created_at_ms: i64) -> LiveSession {
        LiveSession {
            project_id: project(project_path),
            session_id: session(session_native_id),
            created_at: Timestamp::from_epoch_millis(created_at_ms),
        }
    }

    // --- AC1: both layers implemented, consuming T09's types directly ---

    #[test]
    fn a_session_gets_a_category_and_a_word() {
        let mut map = NamingClaimMap::new();
        let nickname = map.claim_session(
            &project("/tmp/proj-a"),
            &session("ses-1"),
            Timestamp::from_epoch_millis(0),
        );
        assert!(CATEGORIES
            .iter()
            .any(|c| c.name == nickname.category.category_name));
        assert!(CATEGORIES[nickname.category.category_index]
            .words
            .contains(&nickname.word));
        assert_eq!(nickname.suffix, None);
    }

    #[test]
    fn same_project_identity_always_prefers_same_category() {
        let mut map_a = NamingClaimMap::new();
        let mut map_b = NamingClaimMap::new();
        let n_a = map_a.claim_session(
            &project("/tmp/proj-x"),
            &session("s1"),
            Timestamp::from_epoch_millis(0),
        );
        let n_b = map_b.claim_session(
            &project("/tmp/proj-x"),
            &session("s1"),
            Timestamp::from_epoch_millis(0),
        );
        assert_eq!(n_a.category.category_index, n_b.category.category_index);
    }

    // --- AC2: claim order is creation-time-ordered, not arrival-ordered ---

    #[test]
    fn claim_order_is_creation_time_not_arrival_order() {
        let sessions_forward = vec![
            live("/tmp/proj-a", "a1", 100),
            live("/tmp/proj-a", "a2", 300),
            live("/tmp/proj-b", "b1", 200),
            live("/tmp/proj-c", "c1", 50),
            live("/tmp/proj-c", "c2", 400),
        ];
        // Same five live sessions, shuffled arrival order (simulating a
        // different REST pagination / SSE delivery order).
        let sessions_reversed = vec![
            live("/tmp/proj-c", "c2", 400),
            live("/tmp/proj-b", "b1", 200),
            live("/tmp/proj-c", "c1", 50),
            live("/tmp/proj-a", "a2", 300),
            live("/tmp/proj-a", "a1", 100),
        ];

        let mut map_forward = NamingClaimMap::new();
        let result_forward = map_forward.claim_batch(sessions_forward);

        let mut map_reversed = NamingClaimMap::new();
        let result_reversed = map_reversed.claim_batch(sessions_reversed);

        for id in ["a1", "a2", "b1", "c1", "c2"] {
            let key = session(id);
            assert_eq!(
                result_forward.get(&key),
                result_reversed.get(&key),
                "session {id} got different claims depending on arrival order"
            );
        }
    }

    // --- AC3: both hard guarantees, design-center scale and capacity boundary ---

    fn assert_guarantees_hold(map: &NamingClaimMap, live_sessions: &[LiveSession]) {
        // Guarantee 1: no two sessions in the same live project share a name.
        let mut per_project: HashMap<ProjectId, Vec<String>> = HashMap::new();
        for s in live_sessions {
            let nickname = map.nickname_of(&s.session_id).expect("session was claimed");
            per_project
                .entry(s.project_id.clone())
                .or_default()
                .push(nickname.display_word());
        }
        for (project_id, names) in &per_project {
            let unique: HashSet<&String> = names.iter().collect();
            assert_eq!(
                unique.len(),
                names.len(),
                "project {project_id:?} has duplicate session names: {names:?}"
            );
        }

        // Guarantee 2: no two live projects show the same category, unless
        // the documented overflow-sharing degrade is active.
        let mut project_ids: Vec<ProjectId> = per_project.keys().cloned().collect();
        project_ids.sort_by_key(|p| p.as_path().to_path_buf());
        let mut by_category: HashMap<usize, Vec<ProjectId>> = HashMap::new();
        for project_id in &project_ids {
            let assignment = map.category_of(project_id).expect("project was claimed");
            by_category
                .entry(assignment.category_index)
                .or_default()
                .push(project_id.clone());
        }
        for (idx, projects) in &by_category {
            if projects.len() > 1 {
                let assignment = map.category_of(&projects[0]).unwrap();
                assert!(
                    assignment.shared,
                    "category {idx} held by {} live projects but not marked shared",
                    projects.len()
                );
            }
        }
    }

    #[test]
    fn guarantees_hold_at_design_center_scale() {
        // ~4 projects, ~2 sessions each (overview.md R5.8).
        let mut sessions = Vec::new();
        for p in 0..4 {
            for s in 0..2 {
                sessions.push(live(
                    &format!("/tmp/design-center-proj-{p}"),
                    &format!("ses-{p}-{s}"),
                    (p * 10 + s) as i64,
                ));
            }
        }
        let mut map = NamingClaimMap::new();
        map.claim_batch(sessions.clone());
        assert_guarantees_hold(&map, &sessions);
    }

    #[test]
    fn guarantees_hold_at_capacity_boundary() {
        // 10 projects, 10 categories (exactly at the assumption's edge),
        // each project's sessions filling its *actually-assigned* category
        // to that category's own word count (10-14 words depending on
        // which one it lands on) — still within the capacity assumption,
        // not past it. A project's assigned category is a hash of its
        // identity, not simply `CATEGORIES[p]`, so an anchor session claims
        // first to discover which category this project actually got.
        let mut map = NamingClaimMap::new();
        let mut sessions: Vec<LiveSession> = Vec::new();

        for p in 0..10i64 {
            let proj = project(&format!("/tmp/capacity-proj-{p}"));

            let anchor_id = session(&format!("ses-{p}-anchor"));
            let anchor_time = Timestamp::from_epoch_millis(p * 1000);
            map.claim_session(&proj, &anchor_id, anchor_time);
            sessions.push(LiveSession {
                project_id: proj.clone(),
                session_id: anchor_id,
                created_at: anchor_time,
            });

            let category_index = map.category_of(&proj).unwrap().category_index;
            let word_count = CATEGORIES[category_index].words.len();

            for w in 1..word_count {
                let sid = session(&format!("ses-{p}-{w}"));
                let created_at = Timestamp::from_epoch_millis(p * 1000 + w as i64);
                map.claim_session(&proj, &sid, created_at);
                sessions.push(LiveSession {
                    project_id: proj.clone(),
                    session_id: sid,
                    created_at,
                });
            }
        }

        assert_guarantees_hold(&map, &sessions);

        // Every project landed on a distinct category (10 projects, 10
        // categories, none shared) and every session got a bare word (no
        // overflow suffix needed at exactly-full occupancy).
        for s in &sessions {
            let nickname = map.nickname_of(&s.session_id).unwrap();
            assert!(
                !nickname.category.shared,
                "no sharing expected at exactly 10/10 capacity"
            );
            assert_eq!(
                nickname.suffix, None,
                "no overflow expected at exactly-full occupancy"
            );
        }
    }

    // --- AC4: cooldown ---

    #[test]
    fn freed_word_is_not_immediately_reclaimed() {
        let mut map = NamingClaimMap::new();
        let proj = project("/tmp/cooldown-proj");

        // A sentinel session that stays alive for the rest of the test, so
        // the project (and its category) never fully vacates when `s1`
        // ends below — a fully-vacated category resets its cooldown
        // bookkeeping (R6.8's cooldown only applies while the category
        // keeps being drawn from; see `release_session`'s doc comment).
        let sentinel = session("sentinel");
        map.claim_session(&proj, &sentinel, Timestamp::from_epoch_millis(0));

        let s1 = session("s1");
        let n1 = map.claim_session(&proj, &s1, Timestamp::from_epoch_millis(1));
        let freed_word = n1.word;

        // s1 ends — its word goes into cooldown.
        map.release_session(&s1);

        // A brand-new session in the same project that happens to prefer
        // the just-freed word must not receive it immediately. We can't
        // force a specific preferred-word hash, so instead assert the
        // invariant directly against the claim-map's own state: the freed
        // word is still recorded in that category's cooldown table.
        let category_index = map.category_of(&proj).unwrap().category_index;
        assert!(
            map.categories[category_index]
                .cooldown
                .keys()
                .any(|&idx| CATEGORIES[category_index].words[idx] == freed_word),
            "word freed by an ended session must enter cooldown, not return to the pool immediately"
        );
    }

    #[test]
    fn freed_word_becomes_reclaimable_once_cooldown_condition_met() {
        let mut map = NamingClaimMap::new();
        let proj = project("/tmp/cooldown-proj-2");

        // Sentinel keeps the project (and its category) alive throughout —
        // same reasoning as `freed_word_is_not_immediately_reclaimed`.
        let sentinel = session("sentinel");
        map.claim_session(&proj, &sentinel, Timestamp::from_epoch_millis(0));

        let s1 = session("s1");
        let n1 = map.claim_session(&proj, &s1, Timestamp::from_epoch_millis(1));
        map.release_session(&s1);

        let category_index = map.category_of(&proj).unwrap().category_index;
        let freed_idx = CATEGORIES[category_index]
            .words
            .iter()
            .position(|&w| w == n1.word)
            .unwrap();
        assert!(map.categories[category_index]
            .cooldown
            .contains_key(&freed_idx));

        // Churn filler sessions through the same project/category until
        // COOLDOWN_OTHER_WORDS distinct words have been claimed and
        // released. Which filler lands on which word index is a hash
        // implementation detail this test deliberately doesn't assume —
        // it loops generously (well past what COOLDOWN_OTHER_WORDS distinct
        // claims could plausibly need) and breaks as soon as the freed word
        // clears, rather than hard-coding an iteration count.
        let word_count = CATEGORIES[category_index].words.len();
        for i in 0..word_count * 3 {
            let sid = session(&format!("filler-{i}"));
            map.claim_session(&proj, &sid, Timestamp::from_epoch_millis((i + 2) as i64));
            map.release_session(&sid);
            if !map.categories[category_index]
                .cooldown
                .contains_key(&freed_idx)
            {
                break;
            }
        }

        assert!(
            !map.categories[category_index].cooldown.contains_key(&freed_idx),
            "word must become reclaimable once {COOLDOWN_OTHER_WORDS} other distinct words have been claimed"
        );
    }

    // --- AC5: release on tombstone frees the claim ---

    #[test]
    fn releasing_last_session_frees_the_project_category() {
        let mut map = NamingClaimMap::new();
        let proj = project("/tmp/release-proj");
        let s1 = session("only-session");
        map.claim_session(&proj, &s1, Timestamp::from_epoch_millis(0));
        assert!(map.category_of(&proj).is_some());

        map.release_session(&s1); // T09's tombstone signal for this session

        assert!(
            map.category_of(&proj).is_none(),
            "project with no live sessions left must release its category"
        );
    }

    #[test]
    fn released_category_is_claimable_by_another_project() {
        let mut map = NamingClaimMap::new();
        let proj_a = project("/tmp/release-proj-a");
        let s1 = session("s1");
        let n1 = map.claim_session(&proj_a, &s1, Timestamp::from_epoch_millis(0));
        map.release_session(&s1);

        // A different project that prefers the same now-vacated category
        // should be able to claim it (not permanently locked out).
        let proj_b = project("/tmp/release-proj-b-different-identity");
        let s2 = session("s2");
        map.claim_session(&proj_b, &s2, Timestamp::from_epoch_millis(1));
        // No assertion on which category proj_b lands on (that's a
        // property of its own hash) — this test only needs the earlier
        // release to not have left the map in a broken state.
        assert!(map.category_of(&proj_a).is_none());
        let _ = n1;
    }

    // --- AC6: capacity edge case degrades visibly, never panics ---

    #[test]
    fn eleventh_live_project_shares_a_category_instead_of_panicking() {
        let mut map = NamingClaimMap::new();
        for p in 0..10 {
            map.claim_session(
                &project(&format!("/tmp/overflow-proj-{p}")),
                &session(&format!("s-{p}")),
                Timestamp::from_epoch_millis(p as i64),
            );
        }
        // 11th project: all 10 categories already have a live holder.
        let overflow_proj = project("/tmp/overflow-proj-10");
        let nickname = map.claim_session(
            &overflow_proj,
            &session("s-10"),
            Timestamp::from_epoch_millis(10),
        );

        assert!(
            nickname.category.shared,
            "11th project must degrade by sharing a category"
        );
        let assignment = map.category_of(&overflow_proj).unwrap();
        assert!(assignment.shared);
    }

    #[test]
    fn word_overflow_degrades_to_numeric_suffix_instead_of_panicking() {
        let mut map = NamingClaimMap::new();
        let proj = project("/tmp/word-overflow-proj");
        // Norse myth has 10 words; find a project identity that actually
        // lands on a 10-word category, then fill it plus one more to force
        // overflow. Simpler: just claim (word_count + 1) sessions in one
        // project and check the last one degrades, whichever category it
        // landed on.
        let category_index = {
            let mut probe = NamingClaimMap::new();
            let n = probe.claim_session(&proj, &session("probe"), Timestamp::from_epoch_millis(0));
            n.category.category_index
        };
        let word_count = CATEGORIES[category_index].words.len();

        let mut last_nickname = None;
        for i in 0..=word_count {
            let sid = session(&format!("s-{i}"));
            last_nickname =
                Some(map.claim_session(&proj, &sid, Timestamp::from_epoch_millis(i as i64)));
        }
        let last_nickname = last_nickname.unwrap();

        assert!(
            last_nickname.suffix.is_some(),
            "session beyond the category's word count must degrade to a suffixed name, not panic"
        );

        // Guarantee 1 still holds even under overflow: every live session
        // in this project has a distinct display name.
        let mut names = HashSet::new();
        for i in 0..=word_count {
            let sid = session(&format!("s-{i}"));
            let nickname = map.nickname_of(&sid).unwrap();
            assert!(
                names.insert(nickname.display_word()),
                "duplicate display name under word overflow: {}",
                nickname.display_word()
            );
        }
    }
}
