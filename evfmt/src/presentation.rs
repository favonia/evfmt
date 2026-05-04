//! Presentation style requested by text/emoji variation selectors.

use crate::unicode;

/// A presentation style requested by a variation selector.
///
/// Text presentation is requested by `U+FE0E` (VS15); emoji presentation by
/// `U+FE0F` (VS16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    /// Text presentation, requested by `U+FE0E`.
    Text,
    /// Emoji presentation, requested by `U+FE0F`.
    Emoji,
}

impl Presentation {
    /// Return the variation selector character that requests this
    /// presentation.
    #[must_use]
    pub(crate) fn as_selector(self) -> char {
        match self {
            Self::Text => unicode::TEXT_PRESENTATION_SELECTOR,
            Self::Emoji => unicode::EMOJI_PRESENTATION_SELECTOR,
        }
    }

    /// Parse a variation selector character into the presentation it requests.
    ///
    /// Returns `None` if the character is not `U+FE0E` or `U+FE0F`.
    #[must_use]
    pub const fn from_selector(ch: char) -> Option<Self> {
        match ch {
            unicode::TEXT_PRESENTATION_SELECTOR => Some(Self::Text),
            unicode::EMOJI_PRESENTATION_SELECTOR => Some(Self::Emoji),
            _ => None,
        }
    }
}
