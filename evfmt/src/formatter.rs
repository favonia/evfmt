//! Core formatting engine.
//!
//! This module provides whole-text formatting.
//
// formatter.rs — The core formatting engine.
//
// Uses the sequence-aware scanner to process text, then asks review for each
// policy-aware finding and applies the default replacement decision.
//
// AUDIT NOTE — Key properties maintained by this module:
//
// 1. IDEMPOTENCY: format(format(x)) == format(x). Verified by prop_idempotent.
// 2. LOSSLESSNESS: only FE0E/FE0F are inserted/removed; all other content is
//    preserved. Verified by prop_only_modifies_selectors.
// 3. NO VIOLATIONS: re-scanning the output produces zero violations under
//    the same policy. Verified by prop_no_violations_in_output.
// 4. CURRENT IMPLEMENTATION SHAPE: format_text scans the input once, reviews
//    each item, and applies the default replacement decision. This is this
//    module's chosen boundary, not a spec requirement; output canonicality is
//    verified by prop_no_violations_in_output.

use crate::policy::Policy;
use crate::review;
use crate::scanner;

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
        if let Some(finding) = review::review_item(item, policy) {
            output.push_str(finding.default_replacement());
        } else {
            output.push_str(item.raw);
        }
    }

    if output == input {
        FormatResult::Unchanged
    } else {
        FormatResult::Changed(output)
    }
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
