//! `evfmt` is both a command-line formatter and a Rust library for
//! normalizing text/emoji variation selectors.
//!
//! The library surface is intended for tools that want to apply `evfmt`'s
//! normalization rules directly, without invoking the CLI as a subprocess.
//!
//! Most callers will want [`format_text`] together with [`Policy`].
//!
//! # Examples
//!
//! Use [`format_text`] for whole-input canonicalization under one [`Policy`].
//! In the example below, `#\u{FE0E}` is NUMBER SIGN followed by VS15, and
//! `\u{00A9}` is a bare COPYRIGHT SIGN. Under the default policy,
//! `#\u{FE0E}` loses the redundant selector, while bare `\u{00A9}` is
//! canonicalized to `\u{00A9}\u{FE0F}`.
//!
//! ```rust
//! use evfmt::{FormatResult, Policy, format_text};
//!
//! let policy = Policy::default();
//!
//! assert_eq!(
//!     format_text("#\u{FE0E}", &policy),
//!     FormatResult::Changed("#".to_owned())
//! );
//! assert_eq!(
//!     format_text("\u{00A9}", &policy),
//!     FormatResult::Changed("\u{00A9}\u{FE0F}".to_owned())
//! );
//! ```
//!
//! For diagnostics or editor integrations, scan once and then work item-by-item.
//! In the next example, `A\u{FE0F}` contains an illegal selector after `A`,
//! `#\u{FE0E}` contains a redundant text selector after NUMBER SIGN, and bare
//! `\u{00A9}` is a singleton that still needs presentation resolution.
//!
//! ```rust
//! use evfmt::{
//!     Policy, ScanKind, ViolationKind, canonicalize_item, classify,
//!     find_violations, scan,
//! };
//!
//! let policy = Policy::default();
//! let input = "A\u{FE0F} #\u{FE0E} \u{00A9}";
//!
//! let items = scan(input);
//! assert!(matches!(items[0].kind, ScanKind::Passthrough));
//! assert!(matches!(items[1].kind, ScanKind::StandaloneSelectors(_)));
//! assert!(matches!(items[3].kind, ScanKind::Singleton { .. }));
//!
//! let violations = find_violations(input, &policy);
//! assert_eq!(violations.len(), 3);
//! assert_eq!(violations[1].replacement, "#");
//!
//! let repaired = canonicalize_item(&items[1], &policy);
//! assert_eq!(repaired, "");
//! assert_eq!(
//!     classify(&items[5], &policy),
//!     Some(ViolationKind::BareNeedsResolution),
//! );
//! ```
//!
//! Use [`find_violations`] for a whole-input diagnostic report, or
//! [`analyze_text`] for whole-input slot analysis.
//!
//! If the default policy is not suitable, build charsets from [`charset`] and
//! construct a [`Policy`] explicitly. In this example, `rights-marks` contains
//! `\u{00A9}`, so bare COPYRIGHT SIGN is allowed to remain bare.
//!
//! ```rust
//! use evfmt::{CharSet, NamedSetId, Policy, format_text};
//!
//! let ascii_and_rights_marks =
//!     CharSet::named(NamedSetId::Ascii) | CharSet::named(NamedSetId::RightsMarks);
//! let policy = Policy::default()
//!     .with_prefer_bare(ascii_and_rights_marks)
//!     .with_bare_as_text(ascii_and_rights_marks);
//!
//! let formatted = format_text("\u{00A9}", &policy);
//! assert_eq!(formatted, evfmt::FormatResult::Unchanged);
//! ```
//!
//! Here "variation-sequence character" means a character listed in Unicode's
//! `emoji-variation-sequences.txt`.
//!
//! Public module boundaries:
//!
//! - the crate root is the high-level API: whole-input formatting and
//!   convenience analysis helpers
//! - [`formatter`] is the repair-oriented item API: formatter policy and
//!   per-item canonicalization
//! - [`mod@classify`] is the diagnostics-oriented item API: ask why a scanned
//!   item is non-canonical
//! - [`scanner`] owns structural tokenization into singletons, keycaps, ZWJ
//!   chains, standalone selector runs, and passthrough slices
//! - [`slot`] exposes the lower-level slot model for advanced tooling
//! - [`charset`] defines the typed character-set model used by the library
//!   policy API
//! - [`unicode`] provides Unicode emoji metadata used by scanning and
//!   canonicalization

mod canonical;

use std::ops::Range;

pub mod charset;
pub mod classify;
pub mod formatter;
pub mod scanner;
pub mod slot;
pub mod unicode;

pub use charset::{CharSet, NamedSetId};
pub use classify::{ViolationKind, classify};
pub use formatter::{FormatResult, Policy, canonicalize_item, format_text};
pub use scanner::{ScanItem, ScanKind, ZwjComponent, ZwjLink, ZwjSequence, scan};
pub use slot::{
    ReasonableSet, SelectorState, SlotAnalysis, SlotKind, analyze_scan_item, canonical_state,
    resolve_singleton,
};

/// A single non-canonical scanned item together with its canonical replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Byte range of the offending item in the original input.
    pub span: Range<usize>,
    /// Original raw source slice for the item.
    pub raw: String,
    /// Why the item is non-canonical.
    pub violation: ViolationKind,
    /// Canonical replacement for the item under the given policy.
    pub replacement: String,
}

/// Find all non-canonical items in an input string together with their
/// canonical replacements.
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, ViolationKind, find_violations};
///
/// let policy = Policy::default();
/// let findings = find_violations("A\u{FE0F} \u{00A9}", &policy);
///
/// assert_eq!(findings.len(), 2);
/// assert_eq!(findings[0].violation, ViolationKind::IllegalSelector);
/// assert_eq!(findings[0].replacement, "");
/// assert_eq!(findings[1].replacement, "\u{00A9}\u{FE0F}");
/// ```
#[must_use]
pub fn find_violations(input: &str, policy: &Policy) -> Vec<Finding> {
    scan(input)
        .into_iter()
        .filter_map(|item| {
            let violation = classify(&item, policy)?;
            Some(Finding {
                span: item.span.clone(),
                raw: item.raw.to_owned(),
                violation,
                replacement: canonicalize_item(&item, policy),
            })
        })
        .collect()
}

/// Analyze an entire input string into slot-level structures.
#[must_use]
pub fn analyze_text(input: &str) -> Vec<SlotAnalysis<'_>> {
    let items = scan(input);
    items.iter().map(analyze_scan_item).collect()
}
