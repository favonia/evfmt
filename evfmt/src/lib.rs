//! `evfmt` is both a command-line formatter and a Rust library for
//! normalizing text/emoji variation selectors.
//!
//! Most callers will want [`format_text`] together with [`Policy`].
//!
//! # Stability
//!
//! This library API is experimental. `evfmt` follows
//! [Cargo's SemVer compatibility conventions][cargo-semver].
//!
//! [cargo-semver]: https://doc.rust-lang.org/cargo/reference/semver.html
//!
//! # Examples
//!
//! Use [`format_text`] for whole-input canonicalization under one [`Policy`].
//! In the example below, `#\u{FE0E}` is NUMBER SIGN followed by VS15, and
//! `\u{00A9}` is a bare COPYRIGHT SIGN. Under the default policy,
//! `#\u{FE0E}` loses the redundant variation selector, while bare `\u{00A9}` is
//! canonicalized to `\u{00A9}\u{FE0F}` because it is text-default.
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
//! assert_eq!(format_text("\u{2728}", &policy), FormatResult::Unchanged);
//! ```
//!
//! For interactive repair or editor integrations, scan the input and then work
//! item-by-item. In the next example, `A\u{FE0F}` contains an unsanctioned
//! presentation selector after `A`, and the caller chooses to apply the
//! formatter's fixed repair.
//! For the built-in `evfmt` decisions, callers can build repaired output from
//! the original scanned items without rescanning after each replacement choice.
//! Walk the original items in order, keeping `item.raw` for unchanged items and
//! substituting the selected replacement for findings.
//!
//! ```rust
//! use evfmt::{Policy, ScanKind, scan};
//! use evfmt::findings::{Violation, analyze_scan_item};
//!
//! let policy = Policy::default();
//! let input = "A\u{FE0F}";
//!
//! let mut items = scan(input);
//! assert!(matches!(items.next().unwrap().kind, ScanKind::Passthrough));
//! let item = items.next().unwrap();
//! assert!(matches!(
//!     item.kind,
//!     ScanKind::UnsanctionedPresentationSelectors(_)
//! ));
//!
//! let finding = analyze_scan_item(&item, &policy).unwrap();
//! assert_eq!(finding.violation(), Violation::UnsanctionedSelectorsOnly);
//! assert!(finding.decision_slots().is_empty());
//!
//! let repaired = finding.replacement(&[]).unwrap();
//! assert_eq!(repaired, "");
//! ```
//!
//! The [`mod@findings`] API is the usual entry point for interactive fixing.
//! It analyzes scanned items under the supplied [`Policy`] and returns the
//! presentation slots that must be chosen for each finding. Fixed repairs have
//! no slots and use the empty decision vector.
//!
//! Custom policies can be built from [`variation_set`] variation sets. In this example,
//! `rights-marks` contains `\u{00A9}`, so bare COPYRIGHT SIGN is allowed to
//! remain bare.
//!
//! ```rust
//! use evfmt::{Policy, format_text, variation_set};
//!
//! let ascii_and_rights_marks = variation_set::ASCII | variation_set::RIGHTS_MARKS;
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
//! - [`policy`] defines formatter policy configuration
//! - [`formatter`] owns whole-text formatting
//! - [`mod@findings`] analyzes scanned items under policy and reports violations
//!   plus available replacements
//! - [`scanner`] owns structural tokenization into singletons, keycaps, ZWJ
//!   chains, standalone variation selector runs, and passthrough slices
//! - [`variation_set`] defines the typed variation-set model used by the library
//!   policy API

pub mod findings;
pub mod formatter;
pub mod policy;
pub mod scanner;
mod unicode;
pub mod variation_set;

pub use findings::{
    DecisionSlot, Finding, PrimaryViolation, PrimaryViolationKind, ReplacementDecision, Violation,
    analyze_scan_item,
};
pub use formatter::{FormatResult, format_text};
pub use policy::Policy;
pub use scanner::{ScanItem, ScanKind, Scanner, scan};
pub use variation_set::VariationSet;
