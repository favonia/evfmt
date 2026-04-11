//! Formatter policy configuration.
//!
//! Policy applies only to standalone variation-sequence characters whose
//! selector state remains ambiguous after sequence-specific cleanup. Keycap,
//! ZWJ, malformed-selector, and other fixed-cleanup cases are repaired before
//! policy is consulted.
//!
//! A policy is expressed with two [`CharSet`] predicates:
//!
//! - `prefer_bare`: characters whose bare form is canonical when bare can
//!   preserve the selected presentation
//! - `bare_as_text`: characters whose bare form should be interpreted as text
//!   presentation, rather than emoji presentation
//!
//! The default policy uses [`charset::ASCII`] for both predicates. That
//! keeps ASCII bare forms such as `#` canonical, removes redundant selectors
//! such as the `FE0E` in `#\u{FE0E}`, and resolves non-ASCII bare forms such as
//! `\u{00A9}` to emoji presentation by inserting `FE0F`.

use crate::charset::{self, CharSet};

/// Formatting policy for standalone variation-sequence characters.
///
/// The policy is base-indexed: when policy is needed, `evfmt` uses the
/// standalone variation-sequence base character to query the `prefer_bare` and
/// `bare_as_text` sets. The pair of answers determines the canonical
/// replacement choices:
///
/// - in both sets: `FE0E` text presentation becomes bare, while bare stays bare
/// - only in `prefer_bare`: `FE0F` emoji presentation becomes bare, while bare
///   stays bare
/// - only in `bare_as_text`: bare becomes `FE0E` text presentation
/// - in neither set: bare becomes `FE0F` emoji presentation
///
/// Explicit selectors not described by those conversions are already
/// canonical for that standalone character, as long as they are sanctioned by
/// Unicode's variation-sequence data.
///
/// Use [`Policy::default`] for the command-line formatter's default behavior,
/// then override individual predicate sets with [`Policy::with_prefer_bare`]
/// and [`Policy::with_bare_as_text`].
///
/// # Examples
///
/// ```rust
/// use evfmt::{charset, FormatResult, Policy, format_text};
///
/// let policy = Policy::default();
///
/// assert_eq!(format_text("#\u{FE0E}", &policy), FormatResult::Changed("#".into()));
/// assert_eq!(
///     format_text("\u{00A9}", &policy),
///     FormatResult::Changed("\u{00A9}\u{FE0F}".into())
/// );
///
/// let rights_marks =
///     charset::ASCII | charset::RIGHTS_MARKS;
/// let policy = Policy::default()
///     .with_prefer_bare(rights_marks)
///     .with_bare_as_text(rights_marks);
///
/// assert_eq!(format_text("\u{00A9}\u{FE0E}", &policy), FormatResult::Changed("\u{00A9}".into()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Policy {
    /// Characters whose bare form is canonical when it preserves presentation.
    prefer_bare: CharSet,
    /// Characters whose bare form represents text presentation.
    bare_as_text: CharSet,
}

impl Policy {
    /// Return a copy of this policy with a new `prefer_bare` set.
    ///
    /// This set controls whether bare form is allowed as the canonical output
    /// for a standalone variation-sequence character. For a character that is
    /// also in `bare_as_text`, the formatter changes explicit text
    /// presentation (`FE0E`) to bare. For a character that is not in
    /// `bare_as_text`, the formatter changes explicit emoji presentation
    /// (`FE0F`) to bare.
    ///
    /// Removing a character from this set means bare form is not canonical for
    /// that character; bare input is then resolved by `bare_as_text`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use evfmt::{charset, FormatResult, Policy, format_text};
    ///
    /// let policy = Policy::default().with_prefer_bare(
    ///     charset::ASCII | charset::RIGHTS_MARKS,
    /// );
    ///
    /// assert_eq!(
    ///     format_text("\u{00A9}\u{FE0F}", &policy),
    ///     FormatResult::Changed("\u{00A9}".into())
    /// );
    /// ```
    #[must_use]
    pub fn with_prefer_bare(mut self, prefer_bare: CharSet) -> Self {
        self.prefer_bare = prefer_bare;
        self
    }

    /// Return a copy of this policy with a new `bare_as_text` set.
    ///
    /// This set controls what bare form means when a standalone
    /// variation-sequence character is not allowed to stay bare. Characters in
    /// this set resolve from bare to text presentation (`FE0E`); characters
    /// outside this set resolve from bare to emoji presentation (`FE0F`).
    ///
    /// When a character is also in `prefer_bare`, this set still matters: it
    /// decides whether the formatter treats `FE0E` or `FE0F` as the redundant
    /// selector that can be removed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use evfmt::{CharSet, FormatResult, Policy, format_text};
    ///
    /// let policy = Policy::default()
    ///     .with_prefer_bare(CharSet::none())
    ///     .with_bare_as_text(CharSet::all());
    ///
    /// assert_eq!(
    ///     format_text("\u{00A9}", &policy),
    ///     FormatResult::Changed("\u{00A9}\u{FE0E}".into())
    /// );
    /// ```
    #[must_use]
    pub fn with_bare_as_text(mut self, bare_as_text: CharSet) -> Self {
        self.bare_as_text = bare_as_text;
        self
    }

    pub(crate) fn singleton_rule(&self, base: char) -> SingletonRule {
        match (
            self.prefer_bare.contains(base),
            self.bare_as_text.contains(base),
        ) {
            (false, false) => SingletonRule::BareToEmoji,
            (false, true) => SingletonRule::BareToText,
            (true, false) => SingletonRule::EmojiToBare,
            (true, true) => SingletonRule::TextToBare,
        }
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            prefer_bare: charset::ASCII,
            bare_as_text: charset::ASCII,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SingletonRule {
    BareToEmoji,
    BareToText,
    TextToBare,
    EmojiToBare,
}
