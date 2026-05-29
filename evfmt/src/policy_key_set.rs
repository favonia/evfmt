//! Finite policy-key sets for formatter policy.
//!
//! This module owns the typed `PolicyKeySet` model used by policy configuration.
//! The public universe contains policy keys. Each key is one variation-sequence
//! base plus one domain:
//!
//! - ordinary, for non-keycap selector slots
//! - keycap-character, where the same base is followed by
//!   `U+20E3 COMBINING ENCLOSING KEYCAP`
//!
//! # Examples
//!
//! ```rust
//! use evfmt::{FormatResult, Policy, PolicyKeySet, format_text, policy_key_set};
//!
//! let policy = Policy::default()
//!     .with_prefer_bare(policy_key_set::ASCII | PolicyKeySet::singleton('\u{00A9}'));
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

/// ASCII variation-sequence bases (`#`, `*`, and `0`-`9`) as non-keycap policy
/// keys.
pub const ASCII: PolicyKeySet = PolicyKeySet {
    chars: CharSet {
        bits: named_bits(NamedSet::Ascii),
    },
    keycap_chars: CharSet::none(),
};
/// Text-default variation-sequence bases as non-keycap policy keys.
pub const TEXT_DEFAULTS: PolicyKeySet = PolicyKeySet {
    chars: CharSet {
        bits: named_bits(NamedSet::TextDefaults),
    },
    keycap_chars: CharSet::none(),
};
/// Emoji-default variation-sequence bases as non-keycap policy keys.
pub const EMOJI_DEFAULTS: PolicyKeySet = PolicyKeySet {
    chars: CharSet {
        bits: named_bits(NamedSet::EmojiDefaults),
    },
    keycap_chars: CharSet::none(),
};
/// Every non-keycap policy key for a variation-sequence base.
pub const VARIATION_BASES: PolicyKeySet = PolicyKeySet {
    chars: ALL_CHARS,
    keycap_chars: CharSet::none(),
};
/// Text-default variation-sequence bases as keycap-character policy keys.
pub const KEYCAP_TEXT_DEFAULTS: PolicyKeySet = PolicyKeySet {
    chars: CharSet::none(),
    keycap_chars: CharSet {
        bits: named_bits(NamedSet::TextDefaults),
    },
};
/// Emoji-default variation-sequence bases as keycap-character policy keys.
pub const KEYCAP_EMOJI_DEFAULTS: PolicyKeySet = PolicyKeySet {
    chars: CharSet::none(),
    keycap_chars: CharSet {
        bits: named_bits(NamedSet::EmojiDefaults),
    },
};
/// Every keycap-character policy key for a variation-sequence base.
pub const KEYCAP_VARIATION_BASES: PolicyKeySet = PolicyKeySet {
    chars: CharSet::none(),
    keycap_chars: ALL_CHARS,
};
/// RGI emoji keycap bases (`#`, `*`, `0`-`9`) as keycap-character policy keys.
pub const KEYCAP_RGI: PolicyKeySet = PolicyKeySet {
    chars: CharSet::none(),
    keycap_chars: CharSet {
        bits: named_bits(NamedSet::KeycapRgi),
    },
};

#[derive(Clone, Copy)]
enum NamedSet {
    Ascii,
    TextDefaults,
    EmojiDefaults,
    KeycapRgi,
}

/// A finite set of formatter policy keys.
///
/// The universe has two domains: non-keycap and keycap-character. Both domains
/// are indexed by the generated variation-sequence base table. Characters
/// outside that table never form policy keys, including in [`PolicyKeySet::all`].
///
/// # Examples
///
/// ```rust
/// use evfmt::{PolicyKeySet, policy_key_set};
///
/// assert!(policy_key_set::TEXT_DEFAULTS.contains('\u{00A9}'));
/// assert!(!policy_key_set::TEXT_DEFAULTS.contains_keycap('\u{00A9}'));
///
/// let keycap_hash = PolicyKeySet::singleton_keycap('#');
/// assert!(keycap_hash.contains_keycap('#'));
/// assert!(!keycap_hash.contains('#'));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyKeySet {
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
/// use evfmt::policy_key_set::is_variation_sequence_character;
///
/// assert!(is_variation_sequence_character('\u{00A9}'));
/// assert!(!is_variation_sequence_character('A'));
/// ```
#[must_use]
pub fn is_variation_sequence_character(ch: char) -> bool {
    has_variation_sequence(ch)
}

impl PolicyKeySet {
    /// Construct the set containing every eligible non-keycap and keycap-character
    /// policy key.
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

    /// Construct a singleton set containing one eligible non-keycap policy key.
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
    /// policy key.
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

    /// Return whether the set contains the non-keycap policy key for `ch`.
    #[must_use]
    pub fn contains(&self, ch: char) -> bool {
        self.chars.contains(ch)
    }

    /// Return whether the set contains the keycap-character policy key for `ch`.
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
        NamedSet::KeycapRgi => ch == '#' || ch == '*' || ch.is_ascii_digit(),
    }
}

impl Default for PolicyKeySet {
    fn default() -> Self {
        Self::none()
    }
}

impl Not for PolicyKeySet {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::all() - self
    }
}

impl BitOr for PolicyKeySet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            chars: self.chars | rhs.chars,
            keycap_chars: self.keycap_chars | rhs.keycap_chars,
        }
    }
}

impl BitOrAssign for PolicyKeySet {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitAnd for PolicyKeySet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            chars: self.chars & rhs.chars,
            keycap_chars: self.keycap_chars & rhs.keycap_chars,
        }
    }
}

impl BitAndAssign for PolicyKeySet {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitXor for PolicyKeySet {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self {
            chars: self.chars ^ rhs.chars,
            keycap_chars: self.keycap_chars ^ rhs.keycap_chars,
        }
    }
}

impl BitXorAssign for PolicyKeySet {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl Sub for PolicyKeySet {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            chars: self.chars - rhs.chars,
            keycap_chars: self.keycap_chars - rhs.keycap_chars,
        }
    }
}

impl SubAssign for PolicyKeySet {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl fmt::Display for PolicyKeySet {
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
                write!(
                    f,
                    "keycap:u({:04X})",
                    unicode::variation_entry(index) as u32
                )?;
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
