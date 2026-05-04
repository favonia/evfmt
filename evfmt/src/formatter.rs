//! Core formatting engine.
//!
//! This module provides whole-text formatting.
//!
//! # Examples
//!
//! ```rust
//! use evfmt::{FormatResult, Policy, format_text};
//!
//! let policy = Policy::default();
//!
//! assert_eq!(
//!     format_text("#\u{FE0E}", &policy),
//!     FormatResult::Changed("#".into())
//! );
//! assert_eq!(format_text("#", &policy), FormatResult::Unchanged);
//! ```
//
// formatter.rs — The core formatting engine.
//
// Uses the sequence-aware scanner to process text, then asks item analysis
// for each policy-aware finding and applies the default canonical replacement.
//
// AUDIT NOTE — Key properties maintained by this module:
//
// 1. IDEMPOTENCY: format(format(x)) == format(x). Verified by prop_idempotent.
// 2. LOSSLESSNESS: only FE0E/FE0F are inserted/removed; all other content is
//    preserved. Verified by prop_only_modifies_selectors.
// 3. CANONICAL OUTPUT: re-scanning the output produces no findings under the
//    same policy. Verified by prop_no_analysis_findings_in_output.
// 4. CURRENT IMPLEMENTATION SHAPE: format_text scans the input once, analyzes
//    each item, and applies the default canonical replacement. This is this
//    module's chosen boundary, not a spec requirement; output canonicality is
//    verified by prop_no_analysis_findings_in_output.

use crate::analysis;
use crate::policy::Policy;
use crate::scanner;

/// The result of formatting a text string.
///
/// # Examples
///
/// ```rust
/// use evfmt::{FormatResult, Policy, format_text};
///
/// match format_text("#\u{FE0E}", &Policy::default()) {
///     FormatResult::Changed(text) => assert_eq!(text, "#"),
///     FormatResult::Unchanged => panic!("the selector should be removed"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
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
    let mut output = String::with_capacity(input.len());

    for item in scanner::scan(input) {
        if let Some(finding) = analysis::analyze_scan_item(&item, policy) {
            output.push_str(&finding.default_canonical_replacement());
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

#[cfg(test)]
mod tests;
