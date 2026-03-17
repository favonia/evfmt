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
//!     Policy, ScanKind, canonicalize_item, classify, find_violations, scan,
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
//! assert!(classify(&items[3], &policy).is_some());
//! ```
//!
//! If the default policy is not suitable, parse expressions from [`expr`] and
//! construct a [`Policy`] explicitly. In this example, `rights-marks` contains
//! `\u{00A9}`, so bare COPYRIGHT SIGN is allowed to remain bare.
//!
//! ```rust
//! use evfmt::{Policy, expr, format_text};
//!
//! let policy = Policy::default()
//!     .with_prefer_bare_for(expr::parse_expr_only("union(ascii, rights-marks)")?)
//!     .with_treat_bare_as_text_for(expr::parse_expr_only(
//!         "union(ascii, rights-marks)",
//!     )?);
//!
//! let formatted = format_text("\u{00A9}", &policy);
//! assert_eq!(formatted, evfmt::FormatResult::Unchanged);
//! # Ok::<(), evfmt::expr::ParseError>(())
//! ```
//!
//! Here "variation-sequence character" means a character listed in Unicode's
//! `emoji-variation-sequences.txt`.
//!
//! Public module boundaries:
//!
//! - [`formatter`] is the rewrite-oriented API: whole-text formatting and
//!   per-item canonicalization
//! - [`mod@classify`] is the diagnostics-oriented API: scan an item and ask why it
//!   is non-canonical
//! - [`scanner`] owns structural tokenization into singletons, keycaps, ZWJ
//!   chains, standalone selector runs, and passthrough slices
//! - [`slot`] exposes the lower-level slot model for advanced tooling
//! - [`expr`] defines the character-set expression language used by policy
//!   options such as `--prefer-bare-for`, including parsing and evaluation
//! - [`unicode`] provides Unicode emoji metadata used by scanning and
//!   canonicalization

mod canonical;

pub mod classify;
pub mod expr;
pub mod formatter;
pub mod scanner;
pub mod slot;
pub mod unicode;

pub use classify::{Finding, ViolationKind, classify, find_violations};
pub use formatter::{FormatResult, Policy, canonicalize_item, format_text};
pub use scanner::{
    ScanItem, ScanKind, ZwjComponent, ZwjLink, ZwjSequence, effective_selector, scan,
};
