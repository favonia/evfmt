//! Finite character sets for formatter policy.
//!
//! This module owns the typed charset model used by policy configuration.
//! The universe is exactly the set of code points listed in the repository's
//! pinned `emoji-variation-sequences.txt` data.

use std::fmt;
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Sub, SubAssign,
};

use crate::unicode::{self, has_variation_sequence};

const WORD_BITS: usize = u64::BITS as usize;
const CHARSET_WORDS: usize = unicode::VARIATION_ENTRY_COUNT.div_ceil(WORD_BITS);
const ALL: CharSet = CharSet { bits: all_bits() };

/// ASCII characters (U+0000-U+007F).
pub const ASCII: CharSet = CharSet {
    bits: named_bits(NamedSet::Ascii),
};
/// Variation-sequence characters whose Unicode default side is text.
pub const TEXT_DEFAULTS: CharSet = CharSet {
    bits: named_bits(NamedSet::TextDefaults),
};
/// Variation-sequence characters whose Unicode default side is emoji.
pub const EMOJI_DEFAULTS: CharSet = CharSet {
    bits: named_bits(NamedSet::EmojiDefaults),
};
/// ©️ (U+00A9), ®️ (U+00AE), ™️ (U+2122).
pub const RIGHTS_MARKS: CharSet = CharSet {
    bits: named_bits(NamedSet::RightsMarks),
};
/// Arrow characters used by the formatter policy docs.
pub const ARROWS: CharSet = CharSet {
    bits: named_bits(NamedSet::Arrows),
};
/// ♠️ (U+2660), ♣️ (U+2663), ♥️ (U+2665), ♦️ (U+2666).
pub const CARD_SUITS: CharSet = CharSet {
    bits: named_bits(NamedSet::CardSuits),
};

#[derive(Clone, Copy)]
enum NamedSet {
    Ascii,
    TextDefaults,
    EmojiDefaults,
    RightsMarks,
    Arrows,
    CardSuits,
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

/// Return whether `ch` is an eligible variation-sequence base character.
///
/// This checks for a base code point in the crate's pinned
/// `emoji-variation-sequences.txt` data, not for a complete base-plus-selector
/// sequence.
#[must_use]
pub fn is_variation_sequence_character(ch: char) -> bool {
    has_variation_sequence(ch)
}

impl CharSet {
    /// Construct the set containing every eligible character.
    #[must_use]
    pub const fn all() -> Self {
        ALL
    }

    /// Construct the empty set.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            bits: [0; CHARSET_WORDS],
        }
    }

    /// Construct a singleton set containing one eligible code point.
    ///
    /// Returns the empty set when `ch` is outside the variation-sequence
    /// character universe checked by [`is_variation_sequence_character`].
    #[must_use]
    pub fn singleton(ch: char) -> Self {
        let mut set = Self::none();
        if let Some(index) = unicode::variation_sequence_index(ch) {
            set.set_index(index);
        }
        set
    }

    /// Return whether the set contains the given character.
    #[must_use]
    pub fn contains(&self, ch: char) -> bool {
        let Some(index) = unicode::variation_sequence_index(ch) else {
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

const fn all_bits() -> [u64; CHARSET_WORDS] {
    let mut bits = [u64::MAX; CHARSET_WORDS];
    let used_bits = unicode::VARIATION_ENTRY_COUNT % WORD_BITS;
    if used_bits != 0 {
        bits[CHARSET_WORDS - 1] = (1u64 << used_bits) - 1;
    }
    bits
}

const fn named_bits(id: NamedSet) -> [u64; CHARSET_WORDS] {
    let mut bits = [0; CHARSET_WORDS];
    let mut index = 0;
    while index < unicode::VARIATION_ENTRY_COUNT {
        let ch = unicode::variation_entry(index);
        if named_entry_matches(id, ch) {
            let word = index / WORD_BITS;
            let bit = index % WORD_BITS;
            bits[word] |= 1u64 << bit;
        }
        index += 1;
    }
    bits
}

const fn named_entry_matches(id: NamedSet, ch: char) -> bool {
    match id {
        NamedSet::Ascii => ch.is_ascii(),
        NamedSet::TextDefaults => unicode::is_text_default(ch),
        NamedSet::EmojiDefaults => unicode::is_emoji_default(ch),
        NamedSet::RightsMarks => matches!(ch, '\u{00A9}' | '\u{00AE}' | '\u{2122}'),
        NamedSet::Arrows => matches!(
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
        NamedSet::CardSuits => {
            matches!(ch, '\u{2660}' | '\u{2663}' | '\u{2665}' | '\u{2666}')
        }
    }
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
        for index in 0..unicode::VARIATION_ENTRY_COUNT {
            if self.contains(unicode::variation_entry(index)) {
                if !first {
                    write!(f, ",")?;
                }
                write!(f, "u({:04X})", unicode::variation_entry(index) as u32)?;
                first = false;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
