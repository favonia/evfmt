//! Sequence-aware scanner for text/emoji variation sequences.
//!
//! The scanner groups input characters into structural units that the analysis
//! and formatting layers care about:
//!
//! - *emoji-like units*: singletons, flag pairs, optionally followed by
//!   emoji modifiers, enclosing keycaps, or tag modifiers
//! - *ZWJ-related sequences*, including malformed leading, consecutive, or
//!   trailing ZWJ links
//! - *unsanctioned presentation selector runs* that are not attached to any
//!   recognized structure
//! - *passthrough* for everything else
//!
//! The scanner preserves the input losslessly: every produced [`ScanItem`]
//! carries its original byte slice in [`ScanItem::raw`] and its byte range
//! in [`ScanItem::span`]. Concatenating the `raw` slices of all items from
//! a scan reconstructs the original input bit-for-bit.
//!
//! Adjacent [`ScanKind::Passthrough`] items are allowed by the API. They
//! carry no additional meaning; only their concatenated source text matters.
//! This keeps the scanner free to yield passthrough text incrementally
//! instead of buffering solely to coalesce it.
//!
//! The scanner API is streaming-shaped but not fully input-streaming yet:
//! [`scan`] still borrows one complete `str`, and [`Scanner`] yields items
//! lazily from that already-available input. This is enough for callers to
//! process and emit results incrementally without first materializing a full
//! item list, but it is not a chunked reader/parser API. Supporting true
//! input streaming would require carrying incomplete UTF-8, presentation
//! selector runs, and in-progress emoji sequences across input chunks.
//!
//! # Examples
//!
//! ```rust
//! use evfmt::scanner::{ScanKind, scan};
//!
//! let items: Vec<_> = scan("A\u{FE0F}\u{00A9}").collect();
//! let reconstructed = items.iter().map(|item| item.raw).collect::<String>();
//!
//! assert_eq!(reconstructed, "A\u{FE0F}\u{00A9}");
//! assert!(matches!(items[0].kind, ScanKind::Passthrough));
//! assert!(matches!(items[1].kind, ScanKind::UnsanctionedPresentationSelectors(_)));
//! assert!(matches!(items[2].kind, ScanKind::EmojiSequence(_)));
//! ```
//!
//! The item model is also shaped for `evfmt`'s built-in analysis and
//! formatting pipeline: callers can analyze the scanned items and rebuild
//! repaired output from the original items without rescanning after each
//! replacement decision. This is a library API affordance, not a requirement
//! of the formatting model. The scanner is intentionally permissive enough
//! to recognize structures that would become canonical after selector-only
//! edits; otherwise the formatter could create newly recognizable structure
//! on a second pass and lose idempotence.

use std::collections::VecDeque;
use std::iter::Peekable;
use std::ops::Range;
use std::str::CharIndices;

use crate::presentation::Presentation;
use crate::unicode;

#[cfg(test)]
mod tests;

// This module implements the structural-recognition contract documented in
// `docs/designs/features/sequence-handling.markdown`. Keep cross-module
// contracts there; keep concrete scanner state shapes and local invariants in
// comments near the code that enforces them.

fn is_presentation_selector(ch: char) -> bool {
    Presentation::from_selector(ch).is_some()
}

// --- Emoji-like components ---

/// A greedily collected run of emoji tag characters (`U+E0020`–`U+E007F`),
/// followed by any presentation selectors that appeared after the run.
///
/// Multiple tag characters are grouped into one run because they form a
/// single contiguous tag specification after selector cleanup. Interspersed
/// selectors are preserved as boundaries between successive [`EmojiTagRun`]s
/// inside an
/// [`EmojiModification::TagModifier`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EmojiTagRun {
    /// The tag characters in source order.
    pub tag: Vec<char>,
    /// Presentation selectors that immediately followed this tag run, in
    /// source order.
    pub presentation_selectors_after_tag: Vec<Presentation>,
}

/// A modification applied to an [`EmojiStem`].
///
/// After the stem is fully resolved, the scanner greedily collects zero or
/// more modifications — emoji modifiers, enclosing keycaps, and tag
/// specifications — each with its trailing presentation selectors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmojiModification {
    /// An emoji modifier (`U+1F3FB`–`U+1F3FF`) and its trailing presentation
    /// selectors.
    EmojiModifier {
        /// The emoji modifier character.
        modifier: char,
        /// Presentation selectors that immediately followed the modifier, in
        /// source order.
        presentation_selectors_after_modifier: Vec<Presentation>,
    },
    /// A combining enclosing keycap (`U+20E3`) and its trailing presentation
    /// selectors.
    ///
    /// The scanner does not check whether the stem is a valid keycap base
    /// (`0`–`9`, `#`, `*`): applying `U+20E3` to any other emoji-eligible
    /// base still produces this variant. Semantic validity is a concern for
    /// the analysis layer, not the scanner. This keeps the enclosing-keycap
    /// structure visible before selector-only cleanup decides whether the
    /// surrounding form is canonical.
    EnclosingKeycap {
        /// Presentation selectors that immediately followed the keycap, in
        /// source order.
        presentation_selectors_after_keycap: Vec<Presentation>,
    },
    /// One or more emoji tag runs forming a tag modifier specification.
    ///
    /// Multiple [`EmojiTagRun`] entries appear when presentation selectors
    /// are interspersed among the tag characters. Keeping them together
    /// lets the first formatting pass see the tag specification that would
    /// result after those selectors are removed.
    TagModifier(Vec<EmojiTagRun>),
}

/// The base of an emoji-like unit, before any modifications.
///
/// Trailing presentation selectors after each character of the stem are
/// greedily absorbed, so the stem is fully resolved before the scanner
/// starts collecting modifications.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmojiStem {
    /// A single emoji character.
    ///
    /// This variant also covers an unpaired regional indicator. The
    /// rationale: UTS #51 allows flag and keycap sequences to participate
    /// in ZWJ sequences, yet UAX #29's GB11 rule only permits
    /// `Extended_Pictographic` characters to be joined by ZWJ. We reconcile
    /// the two specifications by treating keycap bases and regional
    /// indicators as though they were `Extended_Pictographic`. A side
    /// effect is that a lone regional indicator becomes a valid
    /// emoji-like unit on its own — the scanner does not retroactively
    /// reject it if the second indicator never arrives.
    SingletonBase {
        /// The base character.
        base: char,
        /// Presentation selectors that immediately followed the base, in
        /// source order.
        presentation_selectors_after_base: Vec<Presentation>,
    },
    /// A flag sequence: two regional indicator characters.
    ///
    /// After seeing the first regional indicator and greedily consuming its
    /// trailing selectors, the scanner checks whether the next character is
    /// also a regional indicator. If so, both are collected here; otherwise
    /// the first is recorded as a [`SingletonBase`](Self::SingletonBase) stem.
    Flag {
        /// The first regional indicator of the pair.
        first_ri: char,
        /// Presentation selectors that followed the first indicator, in
        /// source order.
        presentation_selectors_after_first_ri: Vec<Presentation>,
        /// The second regional indicator of the pair.
        second_ri: char,
        /// Presentation selectors that followed the second indicator, in
        /// source order.
        presentation_selectors_after_second_ri: Vec<Presentation>,
    },
}

/// An emoji-like unit: a stem plus zero or more modifications.
///
/// "Emoji-like" because the scanner is permissive — it groups structurally
/// plausible sequences without validating that the result is a real Unicode
/// emoji. Semantic validation is the analysis layer's job.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EmojiLike {
    /// The base of the unit.
    pub stem: EmojiStem,
    /// Modifications applied to the stem, in source order.
    pub modifiers: Vec<EmojiModification>,
}

/// A ZWJ (`U+200D`) link, plus any presentation selectors that appeared
/// immediately after it.
///
/// Any such trailing presentation selectors are unsanctioned — the canonical
/// form attaches selectors to components, not to joiners — but the scanner
/// preserves them so the analysis layer can decide how to report and repair them.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ZwjLink {
    /// Presentation selectors that followed the ZWJ, in source order.
    pub presentation_selectors_after_link: Vec<Presentation>,
}

/// One `ZWJ + emoji` pair after the first emoji component of a sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ZwjJoinedEmoji {
    /// The ZWJ link before this emoji component.
    pub link: ZwjLink,
    /// The emoji-like component after the link.
    pub emoji: EmojiLike,
}

/// Emoji-related sequence structure produced by the scanner.
///
/// The type encodes the only ZWJ-like shapes the scanner can emit:
///
/// - `LinksOnly`: one or more ZWJ links with no emoji component
/// - `EmojiHeaded`: one emoji component, followed by zero or more joined emoji
///   components, followed by zero or more trailing ZWJ links
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmojiSequence {
    /// One or more ZWJ links with no emoji component.
    LinksOnly(Vec<ZwjLink>),
    /// An emoji component, optional joined components, and optional trailing links.
    EmojiHeaded {
        /// The first emoji-like component.
        first: EmojiLike,
        /// Components that were reached through a preceding ZWJ link.
        joined: Vec<ZwjJoinedEmoji>,
        /// ZWJ links after the final emoji component.
        trailing_links: Vec<ZwjLink>,
    },
}

// --- Scan items ---

/// A single item produced by the scanner, carrying its raw source slice and
/// a structural classification.
///
/// Concatenating all `raw` slices from a scan reconstructs the original
/// input exactly.
///
/// # Examples
///
/// ```rust
/// use evfmt::{ScanKind, scan};
///
/// let item = scan("\u{00A9}").next().unwrap();
/// assert_eq!(item.raw, "\u{00A9}");
/// assert_eq!(item.span, 0.."\u{00A9}".len());
/// assert!(matches!(item.kind, ScanKind::EmojiSequence(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScanItem<'a> {
    /// The raw source text for this item.
    pub raw: &'a str,
    /// Byte range of this item in the original input.
    pub span: Range<usize>,
    /// Structural classification of this item.
    pub kind: ScanKind,
}

/// The structural classification of a scanned item.
///
/// # Examples
///
/// ```rust
/// use evfmt::{ScanKind, scan};
///
/// let items: Vec<_> = scan("A\u{FE0F}").collect();
///
/// assert!(matches!(items[0].kind, ScanKind::Passthrough));
/// assert!(matches!(
///     items[1].kind,
///     ScanKind::UnsanctionedPresentationSelectors(_)
/// ));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScanKind {
    /// Non-structural content: plain text, characters that are neither
    /// emoji nor presentation selectors nor ZWJ, and similar material.
    ///
    /// Adjacent `Passthrough` items are allowed. They carry no additional
    /// structure; callers that want one plain-text span may concatenate them.
    Passthrough,
    /// A run of presentation selectors that was not attached to any
    /// recognized emoji structure at the scanner level.
    UnsanctionedPresentationSelectors(Vec<Presentation>),
    /// An emoji sequence, possibly containing ZWJ joins.
    EmojiSequence(EmojiSequence),
}

// --- Scanner state ---
//
// The scanner is forward-only with one-character lookahead. The only piece
// of state that persists across scanner advances is an in-progress emoji
// sequence: once we have consumed an emoji-like unit or ZWJ link we do not
// yet know whether the next character will extend the sequence, so we keep
// the legal sequence shape in `EmojiSequenceInProgress` and emit it as a
// single item once the sequence ends.
//
// Sub-structures inside a single emoji-like unit — presentation selector
// runs, regional indicator pairs, modifications, and tag character runs —
// are consumed greedily by helper methods that loop internally with
// one-character lookahead. Those loops run to completion within a single
// scanner advance and do not need persistent state.

/// Emoji sequence currently being assembled.
///
/// The state itself is always one of the legal scanner shapes: empty,
/// links-only, or emoji-headed.
#[derive(Debug, Default)]
enum EmojiSequenceInProgress {
    #[default]
    Empty,
    LinksOnly {
        links: Vec<ZwjLink>,
        end: usize,
    },
    EmojiHeaded {
        first: EmojiLike,
        joined: Vec<ZwjJoinedEmoji>,
        trailing_links: Vec<ZwjLink>,
        end: usize,
    },
}

impl EmojiSequenceInProgress {
    /// Does the in-progress sequence contain no parts?
    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Append a ZWJ link. Every legal scanner sequence can accept one more
    /// trailing link.
    fn push_link(&mut self, link: ZwjLink, end: usize) {
        match self {
            Self::Empty => {
                *self = Self::LinksOnly {
                    links: vec![link],
                    end,
                };
            }
            Self::LinksOnly {
                links,
                end: sequence_end,
            } => {
                links.push(link);
                *sequence_end = end;
            }
            Self::EmojiHeaded {
                trailing_links,
                end: sequence_end,
                ..
            } => {
                trailing_links.push(link);
                *sequence_end = end;
            }
        }
    }

    /// Try to consume an emoji as the one awaited by the current sequence.
    ///
    /// This combines the shape check and the mutation. It succeeds only for
    /// `emoji (ZWJ emoji)* ZWJ`. If the current state is empty, links-only, or
    /// has zero or multiple trailing ZWJ links, the caller must flush before
    /// starting a new emoji-headed sequence.
    fn try_push_awaited_emoji(&mut self, emoji: EmojiLike, end: usize) -> Result<(), EmojiLike> {
        let Self::EmojiHeaded {
            joined,
            trailing_links,
            end: sequence_end,
            ..
        } = self
        else {
            return Err(emoji);
        };

        if trailing_links.len() == 1
            && let Some(link) = trailing_links.pop()
        {
            joined.push(ZwjJoinedEmoji { link, emoji });
            *sequence_end = end;
            Ok(())
        } else {
            Err(emoji)
        }
    }

    /// Build an in-progress sequence whose first part is one emoji-like unit.
    fn from_emoji(emoji: EmojiLike, end: usize) -> Self {
        Self::EmojiHeaded {
            first: emoji,
            joined: vec![],
            trailing_links: vec![],
            end,
        }
    }

    fn take_sequence(&mut self) -> Option<(EmojiSequence, usize)> {
        match std::mem::take(self) {
            Self::Empty => None,
            Self::LinksOnly { links, end } => Some((EmojiSequence::LinksOnly(links), end)),
            Self::EmojiHeaded {
                first,
                joined,
                trailing_links,
                end,
            } => Some((
                EmojiSequence::EmojiHeaded {
                    first,
                    joined,
                    trailing_links,
                },
                end,
            )),
        }
    }
}

// --- Scanner ---

/// A streaming scanner that yields [`ScanItem`]s from input text.
///
/// Created by [`scan`]. Implements [`Iterator`] for lazy, forward-only
/// scanning with one-character lookahead.
///
/// # Examples
///
/// ```rust
/// use evfmt::scan;
///
/// let mut scanner = scan("#\u{FE0E}");
/// let item = scanner.next().unwrap();
///
/// assert_eq!(item.raw, "#\u{FE0E}");
/// assert!(scanner.next().is_none());
/// ```
#[derive(Debug)]
pub struct Scanner<'a> {
    /// The full input string.
    input: &'a str,
    /// Fully classified items waiting to be returned by [`Iterator::next`].
    ///
    /// Most scanner advances enqueue exactly one item, but the queue keeps
    /// `next` independent from that implementation detail.
    ready: VecDeque<ScanItem<'a>>,
    /// Byte offset where the most recently enqueued item ended. The next
    /// enqueued item's span starts here.
    ready_end: usize,
    /// One-character-lookahead cursor over the input.
    cursor: Peekable<CharIndices<'a>>,
    /// Legal emoji sequence shape that has not yet been emitted as a
    /// [`ScanKind::EmojiSequence`] item.
    sequence_in_progress: EmojiSequenceInProgress,
}

/// Scan input text into a streaming sequence of items.
///
/// Returns a [`Scanner`] iterator. Concatenating all [`ScanItem::raw`]
/// slices from the iterator reconstructs the original input exactly.
///
/// # Examples
///
/// ```rust
/// use evfmt::scan;
///
/// let input = "plain #\u{FE0E}";
/// let reconstructed = scan(input).map(|item| item.raw).collect::<String>();
///
/// assert_eq!(reconstructed, input);
/// ```
#[must_use]
pub fn scan(input: &str) -> Scanner<'_> {
    Scanner {
        input,
        ready: VecDeque::new(),
        ready_end: 0,
        cursor: input.char_indices().peekable(),
        sequence_in_progress: EmojiSequenceInProgress::Empty,
    }
}

impl Scanner<'_> {
    // --- Cursor helpers ---

    /// Current byte offset in the input.
    fn offset(&mut self) -> usize {
        self.cursor.peek().map_or(self.input.len(), |&(i, _)| i)
    }

    /// True if the cursor has consumed all input.
    fn at_eof(&mut self) -> bool {
        self.peek().is_none()
    }

    /// Peek the next character without consuming.
    fn peek(&mut self) -> Option<char> {
        self.cursor.peek().map(|&(_, c)| c)
    }

    /// Consume and return the next character if it satisfies `f`.
    fn next_if(&mut self, f: impl FnOnce(char) -> bool) -> Option<char> {
        self.cursor.next_if(|(_, c)| f(*c)).map(|(_, c)| c)
    }

    /// Consume and return the next character if it equals `ch`.
    fn next_if_eq(&mut self, ch: char) -> Option<char> {
        self.cursor.next_if(|(_, c)| *c == ch).map(|_| ch)
    }

    /// Consume the next character and map it through `f` if `f` returns
    /// `Some`; otherwise leave the cursor unchanged.
    fn next_if_map<R>(&mut self, f: impl FnOnce(char) -> Option<R>) -> Option<R> {
        // Rust 1.94's `Peekable::next_if_map_mut` would express this
        // directly, but the crate currently supports Rust 1.88.
        let ch = self.cursor.peek().map(|&(_, c)| c)?;
        let mapped = f(ch)?;
        self.cursor.next();
        Some(mapped)
    }

    /// Consume characters while `f` holds, discarding them.
    fn skip_while(&mut self, mut f: impl FnMut(char) -> bool) {
        while self.next_if(&mut f).is_some() {}
    }

    /// Consume characters while `f` holds, collecting them.
    fn consume_while(&mut self, mut f: impl FnMut(char) -> bool) -> Vec<char> {
        std::iter::from_fn(|| self.next_if(&mut f)).collect()
    }

    /// Consume characters while `f` returns `Some`, collecting the mapped
    /// values.
    fn consume_while_map<R>(&mut self, mut f: impl FnMut(char) -> Option<R>) -> Vec<R> {
        std::iter::from_fn(|| self.next_if_map(&mut f)).collect()
    }

    /// Greedily consume a (possibly empty) run of presentation selectors at
    /// the cursor.
    fn consume_presentation_selectors(&mut self) -> Vec<Presentation> {
        self.consume_while_map(Presentation::from_selector)
    }

    /// Return whether `ch` must be classified by a scanner branch rather
    /// than absorbed into passthrough.
    fn is_structural_start(ch: char) -> bool {
        is_presentation_selector(ch) || ch == unicode::ZWJ || unicode::is_emoji(ch)
    }

    // --- Emission helpers ---

    /// Enqueue a scan item spanning from the end of the previous item to
    /// `end`.
    fn emit_item(&mut self, kind: ScanKind, end: usize) {
        debug_assert!(
            end > self.ready_end,
            "scanner must not emit zero-width items"
        );
        self.ready.push_back(ScanItem {
            #[allow(clippy::string_slice)]
            raw: &self.input[self.ready_end..end],
            span: self.ready_end..end,
            kind,
        });
        self.ready_end = end;
    }

    fn emit_passthrough(&mut self, end: usize) {
        self.emit_item(ScanKind::Passthrough, end);
    }

    fn emit_unsanctioned_selectors(&mut self, selectors: Vec<Presentation>, end: usize) {
        self.emit_item(ScanKind::UnsanctionedPresentationSelectors(selectors), end);
    }

    fn emit_sequence(&mut self, sequence: EmojiSequence, end: usize) {
        self.emit_item(ScanKind::EmojiSequence(sequence), end);
    }

    // --- Sequence-in-progress mutation ---

    /// Emit the sequence in progress, if any, as a single
    /// [`ScanKind::EmojiSequence`] item.
    fn emit_sequence_in_progress(&mut self) {
        if let Some((sequence, end)) = self.sequence_in_progress.take_sequence() {
            self.emit_sequence(sequence, end);
        }
    }

    /// Append a ZWJ link to the in-progress sequence. Never emits.
    fn fold_zwj_link(&mut self, presentation_selectors_after_link: Vec<Presentation>) {
        let end = self.offset();
        self.sequence_in_progress.push_link(
            ZwjLink {
                presentation_selectors_after_link,
            },
            end,
        );
    }

    /// Append an emoji-like unit to the in-progress sequence.
    ///
    /// If the current sequence does not end with an emoji-like component
    /// followed by one ZWJ link, it cannot be extended by this stem and is
    /// emitted before this stem starts the next sequence.
    fn fold_emoji_like(&mut self, stem: EmojiStem, modifiers: Vec<EmojiModification>) {
        let emoji = EmojiLike { stem, modifiers };
        let end = self.offset();
        if let Err(emoji) = self.sequence_in_progress.try_push_awaited_emoji(emoji, end) {
            self.emit_sequence_in_progress();
            self.sequence_in_progress = EmojiSequenceInProgress::from_emoji(emoji, end);
        }
    }

    // --- Local structure consumers ---
    //
    // `consume_*` helpers advance the cursor and return parsed structure.
    // They do not enqueue ready items or mutate `sequence_in_progress`.

    /// Consume one tag-character run followed by its trailing presentation
    /// selectors, if the cursor is at a tag character.
    fn consume_emoji_tag_run(&mut self) -> Option<EmojiTagRun> {
        self.peek().is_some_and(unicode::is_tag).then(|| {
            let tag = self.consume_while(unicode::is_tag);
            let presentation_selectors_after_tag = self.consume_presentation_selectors();
            EmojiTagRun {
                tag,
                presentation_selectors_after_tag,
            }
        })
    }

    /// Consume a ZWJ followed by its trailing presentation selectors, and
    /// return the selectors. Returns `None` if the cursor is not at a ZWJ.
    fn consume_zwj_link(&mut self) -> Option<Vec<Presentation>> {
        self.next_if_eq(unicode::ZWJ)
            .map(|_| self.consume_presentation_selectors())
    }

    /// Consume one modification (modifier, enclosing keycap, or tag
    /// specification) at the cursor, if present.
    fn consume_emoji_modification(&mut self) -> Option<EmojiModification> {
        if let Some(modifier) = self.next_if(unicode::is_emoji_modifier) {
            Some(EmojiModification::EmojiModifier {
                modifier,
                presentation_selectors_after_modifier: self.consume_presentation_selectors(),
            })
        } else if self
            .next_if_eq(unicode::COMBINING_ENCLOSING_KEYCAP)
            .is_some()
        {
            Some(EmojiModification::EnclosingKeycap {
                presentation_selectors_after_keycap: self.consume_presentation_selectors(),
            })
        } else if self.peek().is_some_and(unicode::is_tag) {
            Some(EmojiModification::TagModifier(
                std::iter::from_fn(|| self.consume_emoji_tag_run()).collect(),
            ))
        } else {
            None
        }
    }

    /// Consume zero or more modifications after an emoji stem.
    fn consume_emoji_modifications(&mut self) -> Vec<EmojiModification> {
        std::iter::from_fn(|| self.consume_emoji_modification()).collect()
    }

    /// Consume a regional-indicator stem starting with `first_ri`.
    ///
    /// If a second regional indicator follows, this returns a flag stem;
    /// otherwise it returns a singleton stem for the unpaired indicator.
    fn consume_regional_indicator_stem(&mut self, first_ri: char) -> EmojiStem {
        let presentation_selectors_after_first_ri = self.consume_presentation_selectors();
        let Some(second_ri) = self.next_if(unicode::is_ri) else {
            return EmojiStem::SingletonBase {
                base: first_ri,
                presentation_selectors_after_base: presentation_selectors_after_first_ri,
            };
        };
        let presentation_selectors_after_second_ri = self.consume_presentation_selectors();
        EmojiStem::Flag {
            first_ri,
            presentation_selectors_after_first_ri,
            second_ri,
            presentation_selectors_after_second_ri,
        }
    }

    // --- Main step ---

    /// Advance the scanner until an item is ready to yield.
    ///
    /// Returns `false` only when the input and the sequence in progress are
    /// both exhausted.
    fn prepare_next_item(&mut self) -> bool {
        while self.ready.is_empty() {
            // 1. ZWJ is recognized cluster structure even when malformed.
            //    Preserve it for the analysis layer instead of absorbing it
            //    into passthrough.
            if let Some(selectors) = self.consume_zwj_link() {
                self.fold_zwj_link(selectors);
                continue;
            }

            // 2. A presentation selector at this point is not attached to
            //    any stem. Stems and links absorb trailing selectors
            //    eagerly, so the selector run is unsanctioned.
            if self.peek().is_some_and(is_presentation_selector) {
                debug_assert!(self.sequence_in_progress.is_empty());
                let selectors = self.consume_presentation_selectors();
                let end = self.offset();
                self.emit_unsanctioned_selectors(selectors, end);
                continue;
            }

            // 3. A regional indicator may start a flag. Without a second
            //    regional indicator, it becomes a singleton stem.
            if let Some(first_ri) = self.next_if(unicode::is_ri) {
                let stem = self.consume_regional_indicator_stem(first_ri);
                let modifiers = self.consume_emoji_modifications();
                self.fold_emoji_like(stem, modifiers);
                continue;
            }

            // 4. Any other emoji-eligible character starts a singleton stem.
            if let Some(base) = self.next_if(unicode::is_emoji) {
                let presentation_selectors_after_base = self.consume_presentation_selectors();
                let stem = EmojiStem::SingletonBase {
                    base,
                    presentation_selectors_after_base,
                };
                let modifiers = self.consume_emoji_modifications();
                self.fold_emoji_like(stem, modifiers);
                continue;
            }

            // 5. Nothing at the cursor can extend or start a sequence. Emit
            //    any sequence in progress before the passthrough that
            //    follows it.
            self.emit_sequence_in_progress();
            if !self.ready.is_empty() {
                continue;
            }

            // 6. Past EOF, nothing more to do.
            if self.at_eof() {
                return false;
            }

            // 7. Skip to the next potentially structural character and emit
            //    everything we skipped as passthrough.
            self.skip_while(|ch| !Self::is_structural_start(ch));
            let end = self.offset();
            self.emit_passthrough(end);
        }
        true
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = ScanItem<'a>;

    fn next(&mut self) -> Option<ScanItem<'a>> {
        if let Some(item) = self.ready.pop_front() {
            return Some(item);
        }
        self.prepare_next_item();
        // Either `prepare_next_item` just enqueued items, or we are at
        // EOF. In either case, return whatever is queued (if anything).
        self.ready.pop_front()
    }
}
