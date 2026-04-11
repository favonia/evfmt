//! `evfmt` is both a command-line formatter and a Rust library for
//! normalizing text/emoji variation selectors.
//!
//! Most callers will want [`format_text`] together with [`Policy`].
//!
//! # Examples
//!
//! Use [`format_text`] for whole-input canonicalization under one [`Policy`].
//! In the example below, `#\u{FE0E}` is NUMBER SIGN followed by VS15, and
//! `\u{00A9}` is a bare COPYRIGHT SIGN. Under the default policy,
//! `#\u{FE0E}` loses the redundant variation selector, while bare `\u{00A9}` is
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
//! For interactive repair or editor integrations, scan the input and then work
//! item-by-item. In the next example, `A\u{FE0F}` contains an illegal variation selector
//! after `A`, and the caller chooses to apply the formatter's fixed repair.
//! For the built-in `evfmt` decisions, callers can build repaired output from
//! the original scanned items without rescanning after each replacement choice.
//! Walk the original items in order, keeping `item.raw` for unchanged items and
//! substituting the selected replacement for reviewed findings.
//!
//! ```rust
//! use evfmt::{
//!     Policy, ReplacementDecision, ScanKind, ViolationKind, review_item, scan,
//! };
//!
//! let policy = Policy::default();
//! let input = "A\u{FE0F}";
//!
//! let items = scan(input);
//! assert!(matches!(items[0].kind, ScanKind::Passthrough));
//! assert!(matches!(items[1].kind, ScanKind::StandaloneVariationSelectors(_)));
//!
//! let finding = review_item(&items[1], &policy).unwrap();
//! assert_eq!(finding.violation(), ViolationKind::IllegalVariationSelector);
//! assert_eq!(finding.choices(), &[ReplacementDecision::Fix]);
//!
//! let repaired = finding.replacement(ReplacementDecision::Fix).unwrap();
//! assert_eq!(repaired, "");
//! ```
//!
//! Use [`review_text`] for a whole-input diagnostic report.
//!
//! The [`mod@review`] API is the usual entry point for interactive fixing. It
//! applies the supplied [`Policy`] only where policy is relevant, and returns
//! the valid replacement choices for each finding.
//!
//! Custom policies can be built from [`charset`] charsets. In this example,
//! `rights-marks` contains `\u{00A9}`, so bare COPYRIGHT SIGN is allowed to
//! remain bare.
//!
//! ```rust
//! use evfmt::{charset, Policy, format_text};
//!
//! let ascii_and_rights_marks = charset::ASCII | charset::RIGHTS_MARKS;
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
//! - [`mod@review`] is the policy-aware review API: ask why a scanned item is
//!   non-canonical and which replacements are available; it is the normal API
//!   for interactive fixing
//! - [`scanner`] owns structural tokenization into singletons, keycaps, ZWJ
//!   chains, standalone variation selector runs, and passthrough slices
//! - [`charset`] defines the typed character-set model used by the library
//!   policy API

pub mod charset;
pub mod formatter;
pub mod policy;
pub mod review;
pub mod scanner;
mod unicode;

pub use charset::CharSet;
pub use formatter::{FormatResult, format_text};
pub use policy::Policy;
pub use review::{ReplacementDecision, ReviewFinding, ViolationKind, review_item, review_text};
pub use scanner::{ScanItem, ScanKind, scan};
