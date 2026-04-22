//! Finite variation-position sets for formatter policy.
//!
//! This module owns the typed variation-set model used by policy configuration.
//! The public universe has two domains, both indexed by the repository's
//! pinned `emoji-variation-sequences.txt` base-character table:
//!
//! - ordinary non-keycap variation positions
//! - keycap-character positions, where the same base is followed by
//!   `U+20E3 COMBINING ENCLOSING KEYCAP`
//!
//! # Examples
//!
//! ```rust
//! use evfmt::{FormatResult, Policy, format_text, variation_set};
//!
//! let policy = Policy::default()
//!     .with_prefer_bare(variation_set::ASCII | variation_set::RIGHTS_MARKS)
//!     .with_bare_as_text(variation_set::ASCII | variation_set::RIGHTS_MARKS);
//!
//! assert_eq!(format_text("\u{00A9}", &policy), FormatResult::Unchanged);
//! ```

use std::fmt;
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Sub, SubAssign,
};

use crate::unicode::{self, has_variation_sequence};

const WORD_BITS: usize = u64::BITS as usize;
const CHARSET_WORDS: usize = unicode::VARIATION_ENTRY_COUNT.div_ceil(WORD_BITS);
const ALL_CHARS: CharSet = CharSet { bits: all_bits() };

/// ASCII characters (U+0000-U+007F).
pub const ASCII: VariationSet = VariationSet {
    chars: CharSet {
        bits: named_bits(NamedSet::Ascii),
    },
    keycap_chars: CharSet::none(),
};
/// Variation-sequence characters whose Unicode default side is text.
pub const TEXT_DEFAULTS: VariationSet = VariationSet {
    chars: CharSet {
        bits: named_bits(NamedSet::TextDefaults),
    },
    keycap_chars: CharSet::none(),
};
/// Variation-sequence characters whose Unicode default side is emoji.
pub const EMOJI_DEFAULTS: VariationSet = VariationSet {
    chars: CharSet {
        bits: named_bits(NamedSet::EmojiDefaults),
    },
    keycap_chars: CharSet::none(),
};
/// ©️ (U+00A9), ®️ (U+00AE), ™️ (U+2122).
pub const RIGHTS_MARKS: VariationSet = VariationSet {
    chars: CharSet {
        bits: named_bits(NamedSet::RightsMarks),
    },
    keycap_chars: CharSet::none(),
};
/// Arrow characters used by the formatter policy docs.
pub const ARROWS: VariationSet = VariationSet {
    chars: CharSet {
        bits: named_bits(NamedSet::Arrows),
    },
    keycap_chars: CharSet::none(),
};
/// ♠️ (U+2660), ♣️ (U+2663), ♥️ (U+2665), ♦️ (U+2666).
pub const CARD_SUITS: VariationSet = VariationSet {
    chars: CharSet {
        bits: named_bits(NamedSet::CardSuits),
    },
    keycap_chars: CharSet::none(),
};
/// Every ordinary non-keycap variation-sequence base position.
pub const NON_KEYCAP_CHARS: VariationSet = VariationSet {
    chars: ALL_CHARS,
    keycap_chars: CharSet::none(),
};
/// Every keycap-character position for a variation-sequence base.
pub const KEYCAP_CHARS: VariationSet = VariationSet {
    chars: CharSet::none(),
    keycap_chars: ALL_CHARS,
};
/// RGI emoji keycap bases (`#`, `*`, `0`-`9`) in keycap-character positions.
pub const KEYCAP_EMOJIS: VariationSet = VariationSet {
    chars: CharSet::none(),
    keycap_chars: CharSet {
        bits: named_bits(NamedSet::KeycapEmojis),
    },
};

#[derive(Clone, Copy)]
enum NamedSet {
    Ascii,
    TextDefaults,
    EmojiDefaults,
    RightsMarks,
    Arrows,
    CardSuits,
    KeycapEmojis,
}

/// A finite set of formatter variation positions.
///
/// The universe has two domains: ordinary non-keycap positions and
/// keycap-character positions. Both domains are indexed by the generated
/// variation-sequence base table. Characters outside that table are never
/// members, including in [`VariationSet::all`].
///
/// # Examples
///
/// ```rust
/// use evfmt::{VariationSet, variation_set};
///
/// let rights_marks = variation_set::RIGHTS_MARKS;
/// assert!(rights_marks.contains('\u{00A9}'));
/// assert!(!rights_marks.contains_keycap('\u{00A9}'));
///
/// let keycap_hash = VariationSet::singleton_keycap('#');
/// assert!(keycap_hash.contains_keycap('#'));
/// assert!(!keycap_hash.contains('#'));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariationSet {
    chars: CharSet,
    keycap_chars: CharSet,
}

/// A private bitset over the generated variation-sequence base table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CharSet {
    bits: [u64; CHARSET_WORDS],
}

/// Return whether `ch` is an eligible variation-sequence base character.
///
/// This checks for a base code point in the crate's pinned
/// `emoji-variation-sequences.txt` data, not for a complete base-plus-selector
/// sequence.
///
/// # Examples
///
/// ```rust
/// use evfmt::variation_set::is_variation_sequence_character;
///
/// assert!(is_variation_sequence_character('\u{00A9}'));
/// assert!(!is_variation_sequence_character('A'));
/// ```
#[must_use]
pub fn is_variation_sequence_character(ch: char) -> bool {
    has_variation_sequence(ch)
}

impl VariationSet {
    /// Construct the set containing every eligible ordinary and keycap
    /// variation position.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            chars: ALL_CHARS,
            keycap_chars: ALL_CHARS,
        }
    }

    /// Construct the empty set.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            chars: CharSet::none(),
            keycap_chars: CharSet::none(),
        }
    }

    /// Construct a singleton set containing one eligible ordinary code point.
    ///
    /// Returns the empty set when `ch` is outside the variation-sequence
    /// character universe checked by [`is_variation_sequence_character`].
    #[must_use]
    pub fn singleton(ch: char) -> Self {
        Self {
            chars: CharSet::singleton(ch),
            keycap_chars: CharSet::none(),
        }
    }

    /// Construct a singleton set containing one eligible keycap-character
    /// position.
    ///
    /// Returns the empty set when `ch` is outside the variation-sequence
    /// character universe checked by [`is_variation_sequence_character`].
    #[must_use]
    pub fn singleton_keycap(ch: char) -> Self {
        Self {
            chars: CharSet::none(),
            keycap_chars: CharSet::singleton(ch),
        }
    }

    /// Return whether the set contains the given ordinary character position.
    #[must_use]
    pub fn contains(&self, ch: char) -> bool {
        self.chars.contains(ch)
    }

    /// Return whether the set contains the given keycap-character position.
    #[must_use]
    pub fn contains_keycap(&self, ch: char) -> bool {
        self.keycap_chars.contains(ch)
    }
}

impl CharSet {
    const fn none() -> Self {
        Self {
            bits: [0; CHARSET_WORDS],
        }
    }

    fn singleton(ch: char) -> Self {
        let mut set = Self::none();
        if let Some(index) = unicode::variation_sequence_index(ch) {
            set.set_index(index);
        }
        set
    }

    fn contains(&self, ch: char) -> bool {
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
        NamedSet::KeycapEmojis => ch == '#' || ch == '*' || ch.is_ascii_digit(),
    }
}

impl Default for VariationSet {
    fn default() -> Self {
        Self::none()
    }
}

impl Not for VariationSet {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::all() - self
    }
}

impl BitOr for VariationSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            chars: self.chars | rhs.chars,
            keycap_chars: self.keycap_chars | rhs.keycap_chars,
        }
    }
}

impl BitOrAssign for VariationSet {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitAnd for VariationSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            chars: self.chars & rhs.chars,
            keycap_chars: self.keycap_chars & rhs.keycap_chars,
        }
    }
}

impl BitAndAssign for VariationSet {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitXor for VariationSet {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self {
            chars: self.chars ^ rhs.chars,
            keycap_chars: self.keycap_chars ^ rhs.keycap_chars,
        }
    }
}

impl BitXorAssign for VariationSet {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl Sub for VariationSet {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            chars: self.chars - rhs.chars,
            keycap_chars: self.keycap_chars - rhs.keycap_chars,
        }
    }
}

impl SubAssign for VariationSet {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl fmt::Display for VariationSet {
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
        for index in 0..unicode::VARIATION_ENTRY_COUNT {
            if self.contains_keycap(unicode::variation_entry(index)) {
                if !first {
                    write!(f, ",")?;
                }
                // Keep this spelling parseable for diagnostics and tests, but
                // do not document it as a stable CLI-facing policy item yet.
                write!(f, "k({:04X})", unicode::variation_entry(index) as u32)?;
                first = false;
            }
        }
        Ok(())
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

impl BitAnd for CharSet {
    type Output = Self;

    fn bitand(mut self, rhs: Self) -> Self::Output {
        for index in 0..CHARSET_WORDS {
            self.bits[index] &= rhs.bits[index];
        }
        self
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

impl Sub for CharSet {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        for index in 0..CHARSET_WORDS {
            self.bits[index] &= !rhs.bits[index];
        }
        self
    }
}

#[cfg(test)]
mod tests;
