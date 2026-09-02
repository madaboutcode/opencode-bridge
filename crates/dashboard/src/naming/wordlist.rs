//! The frozen 10-category/60-word Appendix — `docs/specs/dashboard/
//! visuals.md` R6.8, "Appendix — R6.8 word lists (frozen, 2026-09-02)".
//!
//! Copied verbatim from that Appendix. This data is frozen: T10's contract
//! is explicit that both the category list and each category's word list
//! are frozen once approved, because both claim layers are sensitive to
//! each list's order and length — changing either after V1 ships reshuffles
//! who holds which seat. Do not edit these lists here; if the Appendix
//! itself needs to change, that's a spec change in `visuals.md`, and this
//! file gets re-copied from it, not edited independently.

/// One curated category: a display name and its ordered word list. Order
/// matters — [`crate::naming::claim_map`]'s preferred-word hash indexes into
/// this slice by position, so reordering words changes who's holding what.
pub struct Category {
    pub name: &'static str,
    pub words: &'static [&'static str],
}

/// The 10 categories, in Appendix order. Order matters here too, for the
/// same reason: preferred-category hashing indexes into this slice by
/// position.
pub const CATEGORIES: &[Category] = &[
    Category {
        name: "Greek myth",
        words: &[
            "Zeus",
            "Hera",
            "Apollo",
            "Athena",
            "Hermes",
            "Ares",
            "Artemis",
            "Hades",
            "Persephone",
            "Poseidon",
            "Demeter",
            "Dionysus",
            "Hestia",
            "Nemesis",
        ],
    },
    Category {
        name: "Norse myth",
        words: &[
            "Odin", "Thor", "Loki", "Freya", "Baldur", "Heimdall", "Frigg", "Tyr", "Skadi", "Njord",
        ],
    },
    Category {
        name: "Detective fiction",
        words: &[
            "Holmes", "Watson", "Poirot", "Marple", "Columbo", "Marlowe", "Maigret", "Dupin",
            "Wimsey", "Cadfael",
        ],
    },
    Category {
        name: "Sci-fi",
        words: &[
            "Spock",
            "Kirk",
            "Ripley",
            "Trinity",
            "Picard",
            "Sarek",
            "Neo",
            "Solo",
            "Sarah",
            "Deckard",
            "Rorschach",
        ],
    },
    Category {
        name: "Classical composers",
        words: &[
            "Bach", "Mozart", "Chopin", "Handel", "Vivaldi", "Brahms", "Liszt", "Verdi", "Dvorak",
            "Sibelius",
        ],
    },
    Category {
        name: "Chess",
        words: &[
            "Fischer",
            "Carlsen",
            "Karpov",
            "Tal",
            "Anand",
            "Nakamura",
            "Nepo",
            "Ding",
            "Judit",
            "Botvinnik",
        ],
    },
    Category {
        name: "Mollywood",
        words: &[
            "Ganga",
            "Nagavalli",
            "Dasan",
            "Vijayan",
            "Mannar",
            "Velu",
            "Pazhassi",
            "Meenakshi",
            "Kunjikka",
            "Bhaskaran",
        ],
    },
    Category {
        name: "Supervillains",
        words: &[
            "Thanos",
            "Joker",
            "Venom",
            "Ultron",
            "Magneto",
            "Vader",
            "Bane",
            "Sauron",
            "Voldemort",
            "Cruella",
        ],
    },
    Category {
        name: "Sidekicks",
        words: &[
            "Robin", "Samwise", "Ron", "Pippin", "Gimli", "Alfred", "Sancho", "Tonto", "Donkey",
            "Chewie",
        ],
    },
    Category {
        name: "Cricket legends",
        words: &[
            "Sachin", "Kohli", "Dhoni", "Gavaskar", "Kapil", "Dravid", "Sehwag", "Ganguly",
            "Bumrah", "Ashwin",
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// AC7: the Appendix is reproduced verbatim (10 categories) and no word
    /// repeats across categories — this is one of the two capacity
    /// assumptions the hard guarantees depend on (`visuals.md` R6.8: "no
    /// single word is duplicated across two different category files"),
    /// and it's the word-list curator's responsibility, not something
    /// checked at runtime — so it's proven once, here, at test time.
    #[test]
    fn ten_categories_frozen() {
        assert_eq!(
            CATEGORIES.len(),
            10,
            "Appendix defines exactly 10 categories"
        );
    }

    #[test]
    fn no_word_repeats_across_categories() {
        let mut seen = HashSet::new();
        for category in CATEGORIES {
            for word in category.words {
                assert!(
                    seen.insert(*word),
                    "word {word:?} appears in more than one category (category {:?})",
                    category.name
                );
            }
        }
    }

    #[test]
    fn every_word_is_at_most_ten_chars() {
        for category in CATEGORIES {
            for word in category.words {
                assert!(
                    word.chars().count() <= 10,
                    "word {word:?} in category {:?} exceeds 10 characters",
                    category.name
                );
            }
        }
    }

    #[test]
    fn no_category_is_empty() {
        for category in CATEGORIES {
            assert!(
                !category.words.is_empty(),
                "category {:?} has no words",
                category.name
            );
        }
    }
}
