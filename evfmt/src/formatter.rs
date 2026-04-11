//! Core formatting engine.
//!
//! This module provides whole-text formatting, per-item canonicalization, and
//! the public formatter policy type.
//
// formatter.rs — The core formatting engine.
//
// Uses the sequence-aware scanner to process text, then applies policy-based
// decisions to each scan item (singletons, keycap sequences, ZWJ chains).
//
// AUDIT NOTE — Key invariants maintained by this module:
//
// 1. IDEMPOTENCY: format(format(x)) == format(x). Verified by prop_idempotent.
// 2. LOSSLESSNESS: only FE0E/FE0F are inserted/removed; all other content is
//    preserved. Verified by prop_only_modifies_selectors.
// 3. NO VIOLATIONS: re-scanning the output produces zero violations under
//    the same policy. Verified by prop_no_violations_in_output.
// 4. SINGLE-PASS STABILITY: the scanner recognizes any structure that could be
//    exposed by selector deletion in the same pass, so formatting does not
//    need a fixpoint loop and `format_once` is already stable.

use crate::canonical;
use crate::charset::{CharSet, NamedSetId};
use crate::scanner;
use crate::slot::PolicyView;

/// The formatting policy, configured by command-line arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Policy {
    /// Charset for characters whose bare form is preferred (canonical).
    pub prefer_bare: CharSet,
    /// Charset for characters whose bare form is treated as text presentation.
    pub bare_as_text: CharSet,
}

impl Policy {
    /// Return a copy of this policy with a new `prefer_bare` charset.
    #[must_use]
    pub fn with_prefer_bare(mut self, prefer_bare: CharSet) -> Self {
        self.prefer_bare = prefer_bare;
        self
    }

    /// Return a copy of this policy with a new `bare_as_text` charset.
    #[must_use]
    pub fn with_bare_as_text(mut self, bare_as_text: CharSet) -> Self {
        self.bare_as_text = bare_as_text;
        self
    }

    pub(crate) const fn as_view(&self) -> PolicyView<'_> {
        PolicyView {
            prefer_bare: &self.prefer_bare,
            bare_as_text: &self.bare_as_text,
        }
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            prefer_bare: CharSet::named(NamedSetId::Ascii),
            bare_as_text: CharSet::named(NamedSetId::Ascii),
        }
    }
}

/// The result of formatting a text string.
#[derive(Debug, PartialEq, Eq)]
pub enum FormatResult {
    /// The input was already canonical; no changes needed.
    Unchanged,
    /// The input was modified; contains the new text.
    Changed(String),
}

/// Format the input text according to the given policy.
///
/// # Examples
///
/// ```rust
/// use evfmt::{FormatResult, Policy, format_text};
///
/// let policy = Policy::default();
///
/// assert_eq!(
///     format_text("#\u{FE0E}", &policy),
///     FormatResult::Changed("#".to_owned())
/// );
/// assert_eq!(
///     format_text("plain text", &policy),
///     FormatResult::Unchanged
/// );
/// ```
#[must_use]
pub fn format_text(input: &str, policy: &Policy) -> FormatResult {
    let items = scanner::scan(input);
    let mut output = String::with_capacity(input.len());

    for item in &items {
        output.push_str(&canonicalize_item(item, policy));
    }

    if output == input {
        FormatResult::Unchanged
    } else {
        FormatResult::Changed(output)
    }
}

/// Canonicalize a single scanned item according to the formatter policy.
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, canonicalize_item, scan};
///
/// let policy = Policy::default();
/// let items = scan("#\u{FE0E}");
///
/// assert_eq!(canonicalize_item(&items[0], &policy), "#");
/// ```
#[must_use]
pub fn canonicalize_item(item: &scanner::ScanItem<'_>, policy: &Policy) -> String {
    canonical::canonicalize_item(item, &policy.as_view())
}

// --- Tests ---
//
// AUDIT NOTE — Testing strategy overview:
//
// 1. Unit tests (below): hand-written cases with the default policy.
// 2. Decision table (test_decision_table): exhaustive enumeration of all
//    (char_type × input_selector × prefer_bare × bare_as_text)
//    combinations — human-auditable semantic matrix.
// 3. Exhaustive per-entry (test_exhaustive_per_entry): every VARIATION_ENTRIES
//    entry × 4 policies × 3 input forms, with independently computed expected
//    output (no shared code with the formatter).
// 4. Property-based (proptest): 7 invariants over random inputs — idempotence,
//    no violations, no orphans, singleton/keycap/ZWJ properties, losslessness.
#[cfg(test)]
mod tests;
