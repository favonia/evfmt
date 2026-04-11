//! Unicode emoji metadata used by scanning and canonicalization.
//!
//! This module provides the ability to look up whether a given character has
//! sanctioned text/emoji variation sequences and what its default presentation
//! side is.
//!
//! The actual data table is generated at build time by `build.rs` and stored
//! in `unicode_data.rs` inside the build output directory. This module
//! includes it with [`include!`].

// Generated file included at compile time from build.rs output.
// Defines `VariationEntry`, `VARIATION_ENTRIES`, and `EMOJI_MODIFIERS`.
// AUDIT NOTE: VARIATION_ENTRIES is sorted by code_point (BTreeMap in
// build.rs guarantees this), required for binary_search_by_key below.
include!(concat!(env!("OUT_DIR"), "/unicode_data.rs"));

/// Represents which presentation side a character defaults to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DefaultSide {
    /// The character defaults to text presentation (monochrome/outline).
    Text,
    /// The character defaults to emoji presentation (colorful).
    Emoji,
}

/// Information about a character with sanctioned text and emoji
/// variation sequences.
///
/// In this crate, this means the character appears in Unicode's
/// `emoji-variation-sequences.txt`. The build script asserts that every
/// such character has *both* a text (FE0E) and an emoji (FE0F) entry,
/// so table membership alone implies both selectors are sanctioned.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VariationSequenceInfo {
    /// The Unicode-defined default presentation side for this character.
    pub(crate) default_side: DefaultSide,
}

/// Return whether a character has a sanctioned text and/or emoji variation
/// sequence.
///
/// In this crate, this means the character appears in Unicode's
/// `emoji-variation-sequences.txt`.
#[must_use]
pub(crate) fn has_variation_sequence(ch: char) -> bool {
    VARIATION_ENTRIES
        .binary_search_by_key(&ch, |e| e.code_point)
        .is_ok()
}

/// Look up variation-sequence metadata for a character.
///
/// Returns `Some(VariationSequenceInfo)` iff [`has_variation_sequence`]
/// returns true for the same character, or `None` otherwise.
#[must_use]
pub(crate) fn variation_sequence_info(ch: char) -> Option<VariationSequenceInfo> {
    // O(log n) binary search on the sorted VARIATION_ENTRIES table.
    VARIATION_ENTRIES
        .binary_search_by_key(&ch, |e| e.code_point)
        .ok()
        .map(|idx| {
            let e = &VARIATION_ENTRIES[idx];
            VariationSequenceInfo {
                default_side: if e.default_emoji {
                    DefaultSide::Emoji
                } else {
                    DefaultSide::Text
                },
            }
        })
}

/// Return all code points with sanctioned text and/or emoji variation
/// sequences.
///
/// The returned iterator enumerates exactly the characters for which
/// [`has_variation_sequence`] returns true.
#[cfg(test)]
#[must_use]
pub(crate) fn variation_sequence_chars() -> impl ExactSizeIterator<Item = char> + Clone + 'static {
    VARIATION_ENTRIES.iter().map(|entry| entry.code_point)
}

/// Return whether a character has the Unicode `Emoji_Modifier` property.
#[must_use]
pub(crate) fn is_emoji_modifier(ch: char) -> bool {
    EMOJI_MODIFIERS.binary_search(&ch).is_ok()
}

// --- Tests ---
#[cfg(test)]
mod tests;
