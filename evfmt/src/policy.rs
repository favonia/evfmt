//! Formatter policy configuration.
//!
//! Policy applies only to selector slots whose state remains ambiguous after
//! sequence-specific cleanup. Non-keycap and keycap-character slots query the
//! corresponding domains of the same policy sets; ZWJ,
//! malformed-selector, and other fixed-cleanup cases are repaired before
//! policy is consulted.
//!
//! A policy is expressed with two [`PolicyKeySet`] predicates:
//!
//! - `prefer_bare`: policy keys for which bare spelling is canonical when bare can
//!   preserve the selected presentation
//! - `bare_as_text`: policy keys for which bare spelling should be interpreted as text
//!   presentation, rather than emoji presentation
//!
//! The default policy uses [`policy_key_set::ASCII`] plus
//! [`policy_key_set::EMOJI_DEFAULTS`] for `prefer_bare` and
//! [`policy_key_set::TEXT_DEFAULTS`] plus
//! [`policy_key_set::KEYCAP_VARIATION_BASES`] for `bare_as_text`. That keeps
//! ASCII bare forms such as `#` canonical, removes redundant selectors such as
//! the `FE0E` in `#\u{FE0E}`, keeps emoji-default bare forms such as
//! `\u{2728}` canonical, resolves text-default bare forms such as `\u{00A9}` to
//! text presentation by inserting `FE0E`, and resolves bare keycap-character
//! forms such as `#\u{20E3}` to text presentation by inserting `FE0E` before
//! `U+20E3`.
//!
//! # Examples
//!
//! ```rust
//! use evfmt::{FormatResult, Policy, format_text};
//!
//! let policy = Policy::default();
//!
//! assert_eq!(format_text("#\u{FE0E}", &policy), FormatResult::Changed("#".into()));
//! assert_eq!(
//!     format_text("\u{00A9}", &policy),
//!     FormatResult::Changed("\u{00A9}\u{FE0E}".into())
//! );
//! ```

use crate::policy_key_set::{self, PolicyKeySet};

/// Formatting policy for ambiguous selector slots.
///
/// The policy is base-indexed with a non-keycap/keycap domain qualifier. When
/// policy is needed, `evfmt` builds a policy key from the variation-sequence
/// base character and the selected domain, then queries the `prefer_bare` and
/// `bare_as_text` sets with that key. The pair of answers determines the
/// canonical replacement outcomes:
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
/// use evfmt::{FormatResult, Policy, PolicyKeySet, format_text, policy_key_set};
///
/// let policy = Policy::default();
///
/// assert_eq!(format_text("#\u{FE0E}", &policy), FormatResult::Changed("#".into()));
/// assert_eq!(
///     format_text("\u{00A9}", &policy),
///     FormatResult::Changed("\u{00A9}\u{FE0E}".into())
/// );
/// assert_eq!(format_text("\u{2728}", &policy), FormatResult::Unchanged);
///
/// let ascii_and_copyright = policy_key_set::ASCII | PolicyKeySet::singleton('\u{00A9}');
/// let policy = Policy::default()
///     .with_prefer_bare(ascii_and_copyright)
///     .with_bare_as_text(ascii_and_copyright);
///
/// assert_eq!(format_text("\u{00A9}\u{FE0E}", &policy), FormatResult::Changed("\u{00A9}".into()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Policy {
    /// Policy keys for which bare spelling is canonical when it preserves presentation.
    prefer_bare: PolicyKeySet,
    /// Policy keys for which bare spelling represents text presentation.
    bare_as_text: PolicyKeySet,
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
    /// use evfmt::{FormatResult, Policy, PolicyKeySet, format_text, policy_key_set};
    ///
    /// let policy = Policy::default().with_prefer_bare(
    ///     policy_key_set::ASCII | PolicyKeySet::singleton('\u{00A9}'),
    /// );
    ///
    /// assert_eq!(
    ///     format_text("\u{00A9}\u{FE0E}", &policy),
    ///     FormatResult::Changed("\u{00A9}".into())
    /// );
    /// ```
    #[must_use]
    pub fn with_prefer_bare(mut self, prefer_bare: PolicyKeySet) -> Self {
        self.prefer_bare = prefer_bare;
        self
    }

    /// Return a copy of this policy with `prefer_bare` updated by `modify`.
    ///
    /// This is a convenience for applying set operations to the current
    /// `prefer_bare` value without restating the default set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use evfmt::{FormatResult, Policy, format_text, policy_key_set};
    ///
    /// let policy = Policy::default().modify_prefer_bare(|set| {
    ///     set | policy_key_set::TEXT_DEFAULTS
    /// });
    ///
    /// assert_eq!(
    ///     format_text("\u{00A9}\u{FE0E}", &policy),
    ///     FormatResult::Changed("\u{00A9}".into())
    /// );
    /// ```
    #[must_use]
    pub fn modify_prefer_bare(mut self, modify: impl FnOnce(PolicyKeySet) -> PolicyKeySet) -> Self {
        self.prefer_bare = modify(self.prefer_bare);
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
    /// use evfmt::{FormatResult, Policy, PolicyKeySet, format_text};
    ///
    /// let policy = Policy::default()
    ///     .with_prefer_bare(PolicyKeySet::none())
    ///     .with_bare_as_text(PolicyKeySet::all());
    ///
    /// assert_eq!(
    ///     format_text("\u{00A9}", &policy),
    ///     FormatResult::Changed("\u{00A9}\u{FE0E}".into())
    /// );
    /// ```
    #[must_use]
    pub fn with_bare_as_text(mut self, bare_as_text: PolicyKeySet) -> Self {
        self.bare_as_text = bare_as_text;
        self
    }

    /// Return a copy of this policy with `bare_as_text` updated by `modify`.
    ///
    /// This is a convenience for applying set operations to the current
    /// `bare_as_text` value without restating the default set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use evfmt::{FormatResult, Policy, format_text, policy_key_set};
    ///
    /// let policy = Policy::default()
    ///     .modify_bare_as_text(|set| set | policy_key_set::TEXT_DEFAULTS);
    ///
    /// assert_eq!(
    ///     format_text("\u{00A9}", &policy),
    ///     FormatResult::Changed("\u{00A9}\u{FE0E}".into())
    /// );
    /// ```
    #[must_use]
    pub fn modify_bare_as_text(
        mut self,
        modify: impl FnOnce(PolicyKeySet) -> PolicyKeySet,
    ) -> Self {
        self.bare_as_text = modify(self.bare_as_text);
        self
    }

    pub(crate) fn singleton_rule(&self, base: char, is_keycap: bool) -> SingletonRule {
        let prefer_bare = if is_keycap {
            self.prefer_bare.contains_keycap(base)
        } else {
            self.prefer_bare.contains(base)
        };
        let bare_as_text = if is_keycap {
            self.bare_as_text.contains_keycap(base)
        } else {
            self.bare_as_text.contains(base)
        };

        match (prefer_bare, bare_as_text) {
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
            prefer_bare: policy_key_set::ASCII | policy_key_set::EMOJI_DEFAULTS,
            bare_as_text: policy_key_set::TEXT_DEFAULTS | policy_key_set::KEYCAP_VARIATION_BASES,
        }
    }
}

pub(crate) enum SingletonRule {
    BareToEmoji,
    BareToText,
    TextToBare,
    EmojiToBare,
}
