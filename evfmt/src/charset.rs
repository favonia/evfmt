//! Finite character sets for formatter policy.
//!
//! This module owns the typed charset model used by policy configuration.
//! The universe is exactly the set of code points listed in the repository's
//! pinned `emoji-variation-sequences.txt` data.

use std::fmt;
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Sub, SubAssign,
};

use crate::unicode::{self, DefaultSide};

const WORD_BITS: usize = u64::BITS as usize;
const CHARSET_WORDS: usize = unicode::VARIATION_ENTRIES.len().div_ceil(WORD_BITS);

/// Identifier for a named charset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedSetId {
    /// ASCII characters (U+0000-U+007F).
    Ascii,
    /// Variation-sequence characters whose Unicode default side is emoji.
    EmojiDefaults,
    /// ©️ (U+00A9), ®️ (U+00AE), ™️ (U+2122).
    RightsMarks,
    /// Arrow characters used by the formatter policy docs.
    Arrows,
    /// ♠️ (U+2660), ♣️ (U+2663), ♥️ (U+2665), ♦️ (U+2666).
    CardSuits,
}

impl NamedSetId {
    /// Check if a character belongs to this named set.
    #[must_use]
    pub fn matches(&self, ch: char) -> bool {
        match self {
            NamedSetId::Ascii => ch.is_ascii(),
            NamedSetId::EmojiDefaults => unicode::variation_sequence_info(ch)
                .is_some_and(|info| info.default_side == DefaultSide::Emoji),
            NamedSetId::RightsMarks => matches!(ch, '\u{00A9}' | '\u{00AE}' | '\u{2122}'),
            NamedSetId::Arrows => matches!(
                ch,
                '\u{2194}'
                    | '\u{2195}'
                    | '\u{2196}'
                    | '\u{2197}'
                    | '\u{2198}'
                    | '\u{2199}'
                    | '\u{21A9}'
                    | '\u{21AA}'
                    | '\u{27A1}'
                    | '\u{2934}'
                    | '\u{2935}'
                    | '\u{2B05}'
                    | '\u{2B06}'
                    | '\u{2B07}'
            ),
            NamedSetId::CardSuits => {
                matches!(ch, '\u{2660}' | '\u{2663}' | '\u{2665}' | '\u{2666}')
            }
        }
    }
}

impl fmt::Display for NamedSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamedSetId::Ascii => write!(f, "ascii"),
            NamedSetId::EmojiDefaults => write!(f, "emoji-defaults"),
            NamedSetId::RightsMarks => write!(f, "rights-marks"),
            NamedSetId::Arrows => write!(f, "arrows"),
            NamedSetId::CardSuits => write!(f, "card-suits"),
        }
    }
}

/// A finite set of dual-presentation characters.
///
/// The universe is exactly the generated variation-sequence table. Characters
/// outside that universe are never members, including in [`CharSet::all`].
/// The internal representation is private.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharSet {
    bits: [u64; CHARSET_WORDS],
}

impl CharSet {
    /// Construct the set containing every eligible character.
    #[must_use]
    pub fn all() -> Self {
        let mut bits = [u64::MAX; CHARSET_WORDS];
        let used_bits = unicode::VARIATION_ENTRIES.len() % WORD_BITS;
        if used_bits != 0 {
            bits[CHARSET_WORDS - 1] = (1u64 << used_bits) - 1;
        }
        Self { bits }
    }

    /// Construct the empty set.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            bits: [0; CHARSET_WORDS],
        }
    }

    /// Construct a named set.
    #[must_use]
    pub fn named(id: NamedSetId) -> Self {
        let mut set = Self::none();
        for (index, entry) in unicode::VARIATION_ENTRIES.iter().enumerate() {
            if id.matches(entry.code_point) {
                set.set_index(index);
            }
        }
        set
    }

    /// Construct a singleton set containing one code point.
    #[must_use]
    pub fn singleton(ch: char) -> Self {
        let mut set = Self::none();
        if let Some(index) = index_of(ch) {
            set.set_index(index);
        }
        set
    }

    /// Return whether the set contains the given character.
    #[must_use]
    pub fn contains(&self, ch: char) -> bool {
        let Some(index) = index_of(ch) else {
            return false;
        };
        let word = index / WORD_BITS;
        let bit = index % WORD_BITS;
        (self.bits[word] & (1u64 << bit)) != 0
    }

    fn set_index(&mut self, index: usize) {
        let word = index / WORD_BITS;
        let bit = index % WORD_BITS;
        self.bits[word] |= 1u64 << bit;
    }
}

fn index_of(ch: char) -> Option<usize> {
    unicode::VARIATION_ENTRIES
        .binary_search_by_key(&ch, |entry| entry.code_point)
        .ok()
}

impl Default for CharSet {
    fn default() -> Self {
        Self::none()
    }
}

impl Not for CharSet {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::all() - self
    }
}

impl BitOr for CharSet {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        for index in 0..CHARSET_WORDS {
            self.bits[index] |= rhs.bits[index];
        }
        self
    }
}

impl BitOrAssign for CharSet {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitAnd for CharSet {
    type Output = Self;

    fn bitand(mut self, rhs: Self) -> Self::Output {
        for index in 0..CHARSET_WORDS {
            self.bits[index] &= rhs.bits[index];
        }
        self
    }
}

impl BitAndAssign for CharSet {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitXor for CharSet {
    type Output = Self;

    fn bitxor(mut self, rhs: Self) -> Self::Output {
        for index in 0..CHARSET_WORDS {
            self.bits[index] ^= rhs.bits[index];
        }
        self
    }
}

impl BitXorAssign for CharSet {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl Sub for CharSet {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        for index in 0..CHARSET_WORDS {
            self.bits[index] &= !rhs.bits[index];
        }
        self
    }
}

impl SubAssign for CharSet {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl fmt::Display for CharSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::none() {
            return write!(f, "none");
        }
        if *self == Self::all() {
            return write!(f, "all");
        }

        let mut first = true;
        for entry in unicode::VARIATION_ENTRIES {
            if self.contains(entry.code_point) {
                if !first {
                    write!(f, ",")?;
                }
                write!(f, "u({:04X})", entry.code_point as u32)?;
                first = false;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_all() {
        let set = CharSet::all();
        assert!(set.contains('#'));
        assert!(set.contains('\u{00A9}'));
        assert!(!set.contains('A'));
    }

    #[test]
    fn test_contains_none() {
        let set = CharSet::none();
        assert!(!set.contains('#'));
        assert!(!set.contains('\u{00A9}'));
    }

    #[test]
    fn test_named_ascii() {
        let set = CharSet::named(NamedSetId::Ascii);
        assert!(set.contains('#'));
        assert!(!set.contains('\u{00A9}'));
        assert!(!set.contains('A'));
    }

    #[test]
    fn test_named_emoji_defaults() {
        let set = CharSet::named(NamedSetId::EmojiDefaults);
        assert!(set.contains('\u{2728}'));
        assert!(!set.contains('\u{00A9}'));
        assert!(!set.contains('#'));
        assert!(!set.contains('A'));
    }

    #[test]
    fn test_named_rights_marks() {
        let set = CharSet::named(NamedSetId::RightsMarks);
        assert!(set.contains('\u{00A9}'));
        assert!(set.contains('\u{00AE}'));
        assert!(set.contains('\u{2122}'));
        assert!(!set.contains('\u{2660}'));
    }

    #[test]
    fn test_named_arrows() {
        let set = CharSet::named(NamedSetId::Arrows);
        assert!(set.contains('\u{2194}'));
        assert!(set.contains('\u{27A1}'));
        assert!(set.contains('\u{2B05}'));
        assert!(!set.contains('\u{2660}'));
    }

    #[test]
    fn test_named_card_suits() {
        let set = CharSet::named(NamedSetId::CardSuits);
        assert!(set.contains('\u{2660}'));
        assert!(set.contains('\u{2663}'));
        assert!(set.contains('\u{2665}'));
        assert!(set.contains('\u{2666}'));
        assert!(!set.contains('\u{00A9}'));
    }

    #[test]
    fn test_remove_ascii_from_all() {
        let set = CharSet::all() - CharSet::named(NamedSetId::Ascii);
        assert!(!set.contains('#'));
        assert!(set.contains('\u{00A9}'));
    }

    #[test]
    fn test_remove_multiple_named_sets() {
        let set = CharSet::all()
            - CharSet::named(NamedSetId::Ascii)
            - CharSet::named(NamedSetId::EmojiDefaults);
        assert!(!set.contains('#'));
        assert!(!set.contains('\u{2728}'));
        assert!(set.contains('\u{00A9}'));
    }

    #[test]
    fn test_add_singletons() {
        let set = CharSet::singleton('#') | CharSet::singleton('*');
        assert!(set.contains('#'));
        assert!(set.contains('*'));
        assert!(!set.contains('\u{00A9}'));
    }

    #[test]
    fn test_singleton_ignores_non_universe_chars() {
        assert!(CharSet::singleton('#').contains('#'));
        assert!(!CharSet::singleton('A').contains('A'));
    }

    #[test]
    fn test_add_none_is_identity() {
        let set = CharSet::none() | CharSet::named(NamedSetId::Ascii);
        assert!(set.contains('#'));
        assert!(!set.contains('\u{00A9}'));
    }

    #[test]
    fn test_remove_all_clears_set() {
        let set = CharSet::named(NamedSetId::Ascii) - CharSet::all();
        assert!(!set.contains('#'));
        assert!(!set.contains('\u{00A9}'));
    }

    #[test]
    fn test_operator_not_complements_within_universe() {
        let set = !CharSet::named(NamedSetId::Ascii);

        assert!(!set.contains('#'));
        assert!(set.contains('\u{00A9}'));
        assert!(!set.contains('A'));
    }

    #[test]
    fn test_operator_union() {
        let set = CharSet::singleton('#') | CharSet::singleton('*');

        assert!(set.contains('#'));
        assert!(set.contains('*'));
        assert!(!set.contains('\u{00A9}'));
    }

    #[test]
    fn test_operator_intersection() {
        let set = CharSet::named(NamedSetId::Ascii) & CharSet::singleton('#');

        assert!(set.contains('#'));
        assert!(!set.contains('*'));
        assert!(!set.contains('\u{00A9}'));
    }

    #[test]
    fn test_operator_symmetric_difference() {
        let set = (CharSet::singleton('#') | CharSet::singleton('*'))
            ^ (CharSet::singleton('*') | CharSet::singleton('\u{00A9}'));

        assert!(set.contains('#'));
        assert!(!set.contains('*'));
        assert!(set.contains('\u{00A9}'));
    }

    #[test]
    fn test_operator_difference() {
        let set = CharSet::all() - CharSet::named(NamedSetId::Ascii);

        assert!(!set.contains('#'));
        assert!(set.contains('\u{00A9}'));
    }

    #[test]
    fn test_operator_assignments() {
        let mut set = CharSet::singleton('#');
        set |= CharSet::singleton('*');
        set &= CharSet::named(NamedSetId::Ascii);
        set ^= CharSet::singleton('#');
        set -= CharSet::singleton('\u{00A9}');

        assert!(!set.contains('#'));
        assert!(set.contains('*'));
        assert!(!set.contains('\u{00A9}'));
    }

    #[test]
    fn test_display_examples() {
        assert_eq!(CharSet::none().to_string(), "none");
        assert_eq!(CharSet::all().to_string(), "all");
        assert_eq!(CharSet::singleton('#').to_string(), "u(0023)");
    }

    #[test]
    fn test_named_set_display_names() {
        assert_eq!(NamedSetId::Ascii.to_string(), "ascii");
        assert_eq!(NamedSetId::EmojiDefaults.to_string(), "emoji-defaults");
        assert_eq!(NamedSetId::RightsMarks.to_string(), "rights-marks");
        assert_eq!(NamedSetId::Arrows.to_string(), "arrows");
        assert_eq!(NamedSetId::CardSuits.to_string(), "card-suits");
    }

    #[test]
    fn test_named_set_matches_reject_nonmembers() {
        assert!(!NamedSetId::Ascii.matches('\u{00A9}'));
        assert!(!NamedSetId::EmojiDefaults.matches('\u{00A9}'));
        assert!(!NamedSetId::RightsMarks.matches('#'));
        assert!(!NamedSetId::Arrows.matches('\u{2660}'));
        assert!(!NamedSetId::CardSuits.matches('\u{2194}'));
    }

    #[test]
    fn test_default_is_empty() {
        assert_eq!(CharSet::default(), CharSet::none());
    }

    #[test]
    fn test_display_multiple_code_points_in_table_order() {
        let set = CharSet::singleton('*') | CharSet::singleton('#');

        assert_eq!(set.to_string(), "u(0023),u(002A)");
    }
}
