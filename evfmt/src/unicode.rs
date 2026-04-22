//! Unicode emoji metadata used by scanning and canonicalization.
//!
//! This module provides the ability to look up whether a given character has
//! sanctioned text/emoji variation sequences and what its default presentation
//! side is.
//!
//! The actual data tables are generated at build time by `build.rs` and stored
//! in `unicode_data.rs` inside the build output directory. This module
//! includes them with [`include!`].

// Generated file included at compile time from build.rs output.
// Defines: VARIATION_ENTRIES, EMOJI_MODIFIERS, EMOJI_PRESENTATION_RANGES,
//          EMOJI_RANGES, RI_RANGES.
// AUDIT NOTE: VARIATION_ENTRIES and EMOJI_MODIFIERS are sorted by code point
// (BTreeSet in build.rs guarantees this), required for binary search below.
// Range tables are sorted and non-overlapping.
include!(concat!(env!("OUT_DIR"), "/unicode_data.rs"));

/// Text presentation selector (Unicode variation selector 15).
pub(crate) const TEXT_PRESENTATION_SELECTOR: char = '\u{FE0E}';

/// Emoji presentation selector (Unicode variation selector 16).
pub(crate) const EMOJI_PRESENTATION_SELECTOR: char = '\u{FE0F}';

/// Combining enclosing keycap.
pub(crate) const COMBINING_ENCLOSING_KEYCAP: char = '\u{20E3}';

/// Zero-width joiner.
pub(crate) const ZWJ: char = '\u{200D}';

// --- Inline Predicates ---

/// Returns true if the character is a tag character (U+E0020..U+E007F).
pub(crate) fn is_tag(ch: char) -> bool {
    ('\u{E0020}'..='\u{E007F}').contains(&ch)
}

// --- Table-Driven Predicates ---

/// Binary search a sorted, non-overlapping range table for `ch`.
///
/// We use a custom implementation instead of the standard library's
/// `binary_search_by` because this function needs to be `const` for
/// compile-time bitset construction in `variation_set.rs`.
const fn in_ranges(ranges: &[(char, char)], ch: char) -> bool {
    let val = ch as u32;
    let mut lo = 0usize;
    let mut hi = ranges.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let (start, end) = ranges[mid];
        if val < start as u32 {
            hi = mid;
        } else if val > end as u32 {
            lo = mid + 1;
        } else {
            return true;
        }
    }
    false
}

/// Binary search a sorted character table for `ch`.
///
/// This is intentionally separate from [`variation_sequence_index`]: callers
/// that build compile-time data need a `const` predicate, while runtime callers
/// still benefit from the standard-library search returning an index.
const fn in_char_table(table: &[char], ch: char) -> bool {
    let val = ch as u32;
    let mut lo = 0usize;
    let mut hi = table.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let current = table[mid] as u32;
        if val < current {
            hi = mid;
        } else if val > current {
            lo = mid + 1;
        } else {
            return true;
        }
    }
    false
}

/// The number of variation sequence entries.
pub(crate) const VARIATION_ENTRY_COUNT: usize = VARIATION_ENTRIES.len();

/// Return the variation sequence entry at `index`.
pub(crate) const fn variation_entry(index: usize) -> char {
    VARIATION_ENTRIES[index]
}

/// Return all code points with sanctioned text and/or emoji variation
/// sequences.
///
/// The returned iterator enumerates exactly the characters for which
/// [`has_variation_sequence`] returns true.
#[cfg(test)]
#[must_use]
pub(crate) fn variation_sequence_chars() -> impl ExactSizeIterator<Item = char> + Clone + 'static {
    VARIATION_ENTRIES.iter().copied()
}

/// Return whether a character has a sanctioned text and/or emoji variation
/// sequence, and if so, its index in the table.
///
/// In this crate, this means the character appears in Unicode's
/// `emoji-variation-sequences.txt`.
#[must_use]
pub(crate) fn variation_sequence_index(ch: char) -> Option<usize> {
    VARIATION_ENTRIES.binary_search(&ch).ok()
}

/// Return whether a character has a sanctioned text and/or emoji variation
/// sequence.
///
/// In this crate, this means the character appears in Unicode's
/// `emoji-variation-sequences.txt`.
#[must_use]
pub(crate) fn has_variation_sequence(ch: char) -> bool {
    variation_sequence_index(ch).is_some()
}

/// Return whether a variation-sequence character defaults to text
/// presentation in the pinned Unicode data.
///
/// UTS #51 §4, "Presentation Style", calls text-default characters those
/// expected to have text presentation by default, while still allowing emoji
/// presentation. ED-8a and ED-9a say the only valid text/emoji presentation
/// sequences are those listed in `emoji-variation-sequences.txt`; within that
/// base universe, absence of `Emoji_Presentation` selects the text-default
/// side.
pub(crate) const fn is_text_default(ch: char) -> bool {
    in_char_table(&VARIATION_ENTRIES, ch) && !in_ranges(&EMOJI_PRESENTATION_RANGES, ch)
}

/// Return whether a variation-sequence character defaults to emoji
/// presentation in the pinned Unicode data.
///
/// UTS #51 §4, "Presentation Style", calls emoji-default characters those
/// expected to have emoji presentation by default, while still allowing text
/// presentation. ED-8a and ED-9a say the only valid text/emoji presentation
/// sequences are those listed in `emoji-variation-sequences.txt`; within that
/// base universe, presence of `Emoji_Presentation` selects the emoji-default
/// side.
pub(crate) const fn is_emoji_default(ch: char) -> bool {
    in_char_table(&VARIATION_ENTRIES, ch) && in_ranges(&EMOJI_PRESENTATION_RANGES, ch)
}

/// Return whether a character has the Unicode `Emoji_Modifier` property.
#[must_use]
pub(crate) fn is_emoji_modifier(ch: char) -> bool {
    EMOJI_MODIFIERS.binary_search(&ch).is_ok()
}

/// Return whether a character has the Unicode `Emoji` property.
#[must_use]
pub(crate) const fn is_emoji(ch: char) -> bool {
    in_ranges(&EMOJI_RANGES, ch)
}

/// Return whether a character has the Unicode `Regional_Indicator` property.
#[must_use]
pub(crate) const fn is_ri(ch: char) -> bool {
    in_ranges(&RI_RANGES, ch)
}

// --- Tests ---
#[cfg(test)]
mod tests;
