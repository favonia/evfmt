//! Sequence-aware scanner for text/emoji variation sequences.
//!
//! Recognizes singletons, keycap sequences, ZWJ chains, and standalone
//! variation selector runs.
//!
//! The scanner preserves the input losslessly: structured items retain enough
//! information to reconstruct their own raw slices bit-for-bit, and arbitrary
//! non-structural text is coalesced into [`ScanKind::Passthrough`].
//! Concatenating all [`ScanItem::raw`] values reconstructs the original input.
//!
//! The item model is also shaped for `evfmt`'s built-in review and formatting
//! pipeline: callers can review the scanned items and build repaired output
//! from the original items without rescanning after each valid replacement decision.
//! This is a library API affordance, not a requirement of the formatting model.

use std::ops::Range;

use crate::unicode;

#[cfg(any(test, fuzzing))]
use self::tests::legacy::scan_legacy;

#[cfg(any(test, fuzzing))]
mod tests {
    pub(super) mod legacy;

    #[cfg(test)]
    mod cases;
}

/// Unicode variation selector 15 (text presentation).
pub const VS_TEXT: char = '\u{FE0E}';
/// Unicode variation selector 16 (emoji presentation).
pub const VS_EMOJI: char = '\u{FE0F}';
pub(crate) const ZWJ: char = '\u{200D}';
pub(crate) const KEYCAP_CAP: char = '\u{20E3}';

/// Returns true if the character is a variation selector (VS15 or VS16).
#[must_use]
fn is_variation_selector(ch: char) -> bool {
    ch == VS_TEXT || ch == VS_EMOJI
}

/// A Unicode variation selector: text (U+FE0E) or emoji (U+FE0F).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariationSelector {
    /// Text presentation variation selector (U+FE0E).
    Text,
    /// Emoji presentation variation selector (U+FE0F).
    Emoji,
}

impl VariationSelector {
    /// Convert to the underlying Unicode character.
    #[must_use]
    pub const fn to_char(self) -> char {
        match self {
            Self::Text => VS_TEXT,
            Self::Emoji => VS_EMOJI,
        }
    }

    /// Try to convert a Unicode character to a variation selector.
    ///
    /// Returns `None` if the character is not `VS_TEXT` or `VS_EMOJI`.
    #[must_use]
    pub const fn from_char(ch: char) -> Option<Self> {
        match ch {
            VS_TEXT => Some(Self::Text),
            VS_EMOJI => Some(Self::Emoji),
            _ => None,
        }
    }
}

/// Returns true if the character is a valid keycap base (`#`, `*`, `0`–`9`).
#[must_use]
fn is_keycap_base(ch: char) -> bool {
    ch == '#' || ch == '*' || ch.is_ascii_digit()
}

fn is_emoji_modifier(ch: char) -> bool {
    unicode::is_emoji_modifier(ch)
}

fn is_valid_zwj_component_base(ch: char) -> bool {
    !is_variation_selector(ch) && ch != ZWJ && ch != KEYCAP_CAP && !is_emoji_modifier(ch)
}

// --- Scan item types ---

/// A single item produced by the scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScanItem<'a> {
    /// The raw source text for this item.
    pub raw: &'a str,
    /// Byte range in the original input.
    pub span: Range<usize>,
    /// What kind of item this is.
    pub kind: ScanKind,
}

/// The classification of a scanned item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScanKind {
    /// Non-emoji content (plain text, ineligible characters, standalone ZWJ, etc.).
    Passthrough,
    /// Standalone variation selectors not attached to a recognized logical unit.
    StandaloneVariationSelectors(Vec<VariationSelector>),
    /// Single variation-sequence code point with trailing variation selectors.
    Singleton {
        /// The base character.
        base: char,
        /// Trailing variation selectors, in source order.
        variation_selectors: Vec<VariationSelector>,
    },
    /// Keycap sequence: base, trailing variation selectors, then U+20E3.
    Keycap {
        /// The base character (`#`, `*`, or a digit).
        base: char,
        /// Trailing variation selectors before U+20E3, in source order.
        variation_selectors: Vec<VariationSelector>,
    },
    /// ZWJ sequence: two or more components joined by U+200D.
    ///
    /// The sequence structure is lossless: variation selectors after a joiner are stored
    /// on the link itself.
    Zwj(ZwjSequence),
}

/// One component of a ZWJ sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ZwjComponent {
    /// The base character of this component.
    pub base: char,
    /// Variation selectors immediately after the base and before any emoji
    /// modifier.
    pub variation_selectors_after_base: Vec<VariationSelector>,
    /// Unicode emoji modifier, if present.
    pub emoji_modifier: Option<char>,
    /// Variation selectors after the base when no modifier is present, or after
    /// the emoji modifier when one is present.
    pub variation_selectors_after_modifier: Vec<VariationSelector>,
}

/// One explicit U+200D join between two ZWJ components.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ZwjLink {
    /// Variation selectors immediately after the joiner and before the next
    /// component base.
    pub variation_selectors: Vec<VariationSelector>,
}

/// A lossless recursive representation of a ZWJ sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ZwjSequence {
    /// Final component in the sequence.
    Terminal(ZwjComponent),
    /// One component, then a join, then the rest of the sequence.
    Joined {
        /// The component before the join.
        head: ZwjComponent,
        /// The explicit join edge to the following component.
        link: ZwjLink,
        /// The remainder of the sequence.
        tail: Box<ZwjSequence>,
    },
}

/// Return the only variation selector that can directly affect the base character.
///
/// Unicode variation sequences are a base character followed by a single
/// variation selector. Any additional variation selectors in the same run trail the
/// first variation selector rather than the base itself.
#[must_use]
pub(crate) fn effective_selector(
    variation_selectors: &[VariationSelector],
) -> Option<VariationSelector> {
    variation_selectors.first().copied()
}

/// Return the variation selector run at the terminal variation-selector-bearing position of a ZWJ
/// component.
#[must_use]
pub(crate) fn zwj_component_terminal_selectors(component: &ZwjComponent) -> &[VariationSelector] {
    if component.emoji_modifier.is_some() {
        &component.variation_selectors_after_modifier
    } else {
        &component.variation_selectors_after_base
    }
}

/// Return the only variation selector that can directly affect the terminal position of a
/// ZWJ component.
#[must_use]
pub(crate) fn zwj_component_effective_selector(
    component: &ZwjComponent,
) -> Option<VariationSelector> {
    effective_selector(zwj_component_terminal_selectors(component))
}

// --- Scanner ---

/// Scan input text into a sequence of items.
///
/// Reconstruction principle:
/// concatenating all [`ScanItem::raw`] slices reconstructs the original input
/// exactly, and every non-`Passthrough` item kind preserves enough structured
/// information to reconstruct its own raw slice bit-for-bit.
///
/// # Panics
///
/// Panics if `input` contains invalid UTF-8 (impossible for `&str`).
///
/// # Examples
///
/// ```rust
/// use evfmt::{ScanKind, scan};
///
/// let items = scan("x#\u{FE0F}\u{20E3}y");
///
/// assert!(matches!(items[0].kind, ScanKind::Passthrough));
/// assert!(matches!(items[1].kind, ScanKind::Keycap { .. }));
/// assert!(matches!(items[2].kind, ScanKind::Passthrough));
/// ```
#[must_use]
pub fn scan(input: &str) -> Vec<ScanItem<'_>> {
    scan_state_machine(input)
}

#[cfg(any(test, fuzzing))]
#[must_use]
/// Run the legacy and state-machine scanners on the same input for equivalence checking.
pub fn scan_crosscheck(input: &str) -> (Vec<ScanItem<'_>>, Vec<ScanItem<'_>>) {
    (scan_legacy(input), scan_state_machine(input))
}

fn scan_state_machine(input: &str) -> Vec<ScanItem<'_>> {
    // AUDIT NOTE: scan priority order matters here:
    // 1. Standalone variation selector run: VS not attached to a recognized logical unit
    // 2. Keycap: base `[VS] U+20E3`
    // 3. ZWJ chain: two+ components joined by `U+200D`
    // 4. Singleton: variation-sequence character `[VS]`
    // 5. Passthrough: everything else, coalesced into runs
    //
    // Keycap must be checked before singleton so that `#️⃣` stays a keycap
    // instead of splitting into `#` plus a stray variation selector run. ZWJ must also
    // precede singleton to avoid consuming the first component on its own.
    //
    // `pos` always advances by `ch.len_utf8()`, so all string slices stay on
    // valid UTF-8 boundaries.
    let mut items = Vec::new();
    let mut pos = 0;
    let mut passthrough_start: Option<usize> = None;

    while pos < input.len() {
        #[allow(clippy::expect_used)]
        let ch = peek(input, pos).expect("pos is within input.len() and on a UTF-8 boundary");

        if is_variation_selector(ch) {
            flush_passthrough(input, &mut items, &mut passthrough_start, pos);
            let (end, variation_selectors) = consume_selector_run(input, pos);
            items.push(make_item(
                input,
                pos,
                end,
                ScanKind::StandaloneVariationSelectors(variation_selectors),
            ));
            pos = end;
            continue;
        }

        match scan_structured_state_machine(input, pos, ch) {
            StructuredScan::Emit { end, kind } => {
                flush_passthrough(input, &mut items, &mut passthrough_start, pos);
                items.push(make_item(input, pos, end, kind));
                pos = end;
            }
            StructuredScan::Passthrough { end } => {
                let start = passthrough_start.get_or_insert(pos);
                debug_assert!(*start <= pos, "passthrough start must not move forward");
                debug_assert!(end > pos, "passthrough state must make forward progress");
                pos = end;
            }
        }
    }

    flush_passthrough(input, &mut items, &mut passthrough_start, input.len());
    items
}

fn starts_structured_item(input: &str, pos: usize, ch: char) -> bool {
    is_variation_selector(ch)
        || unicode::has_variation_sequence(ch)
        || try_zwj(input, pos, ch).is_some()
}

fn consume_passthrough_run(input: &str, pos: usize) -> usize {
    #[allow(clippy::expect_used)]
    let ch = peek(input, pos).expect("pos is within input.len() and on a UTF-8 boundary");
    let mut end = pos + ch.len_utf8();

    while end < input.len() {
        #[allow(clippy::expect_used)]
        let next = peek(input, end).expect("pos is within input.len() and on a UTF-8 boundary");
        if starts_structured_item(input, end, next) {
            break;
        }
        let next_end = end + next.len_utf8();
        debug_assert!(
            next_end > end,
            "passthrough scan must make forward progress"
        );
        end = next_end;
    }

    end
}

enum StructuredScan {
    Emit { end: usize, kind: ScanKind },
    Passthrough { end: usize },
}

enum ZwjMachine {
    Candidate {
        first_component: ZwjComponent,
        start: usize,
        component_end: usize,
        singleton_end: Option<usize>,
    },
    Confirmed {
        components: Vec<ZwjComponent>,
        links: Vec<ZwjLink>,
        cursor: usize,
    },
}

fn scan_structured_state_machine(input: &str, pos: usize, ch: char) -> StructuredScan {
    let ch_len = ch.len_utf8();
    let (after_base_selectors, variation_selectors_after_base) =
        consume_optional_selector_run(input, pos + ch_len);
    debug_assert!(
        after_base_selectors >= pos + ch_len,
        "structured scan must not rewind before the base"
    );

    if is_keycap_base(ch) && peek(input, after_base_selectors) == Some(KEYCAP_CAP) {
        let end = after_base_selectors + KEYCAP_CAP.len_utf8();
        return StructuredScan::Emit {
            end,
            kind: ScanKind::Keycap {
                base: ch,
                variation_selectors: variation_selectors_after_base,
            },
        };
    }

    let singleton_end = unicode::has_variation_sequence(ch).then_some(after_base_selectors);

    if is_valid_zwj_component_base(ch) {
        let mut cursor = after_base_selectors;
        let emoji_modifier = match peek(input, cursor) {
            Some(next) if is_emoji_modifier(next) => {
                cursor += next.len_utf8();
                Some(next)
            }
            _ => None,
        };
        let (component_end, variation_selectors_after_modifier) =
            consume_optional_selector_run(input, cursor);
        let first_component = ZwjComponent {
            base: ch,
            variation_selectors_after_base: variation_selectors_after_base.clone(),
            emoji_modifier,
            variation_selectors_after_modifier,
        };

        if peek(input, component_end) == Some(ZWJ) {
            return scan_zwj_machine(
                input,
                ZwjMachine::Candidate {
                    first_component,
                    start: pos,
                    component_end,
                    singleton_end,
                },
            );
        }
    }

    if let Some(end) = singleton_end {
        StructuredScan::Emit {
            end,
            kind: ScanKind::Singleton {
                base: ch,
                variation_selectors: variation_selectors_after_base,
            },
        }
    } else {
        StructuredScan::Passthrough { end: pos + ch_len }
    }
}

fn scan_zwj_machine(input: &str, mut state: ZwjMachine) -> StructuredScan {
    loop {
        match state {
            ZwjMachine::Candidate {
                first_component,
                start,
                component_end,
                singleton_end,
            } => {
                let zwj_end = component_end + ZWJ.len_utf8();
                let (next_pos, variation_selectors) = consume_optional_selector_run(input, zwj_end);
                let Some(next_base) = peek(input, next_pos) else {
                    return fallback_from_zwj_candidate(
                        input,
                        start,
                        first_component,
                        singleton_end,
                    );
                };
                if !is_valid_zwj_component_base(next_base) {
                    return fallback_from_zwj_candidate(
                        input,
                        start,
                        first_component,
                        singleton_end,
                    );
                }

                let mut components = vec![first_component];
                let links = vec![ZwjLink {
                    variation_selectors,
                }];
                let (next_component, cursor) = consume_component(input, next_pos, next_base);
                components.push(next_component);
                state = ZwjMachine::Confirmed {
                    components,
                    links,
                    cursor,
                };
            }
            ZwjMachine::Confirmed {
                mut components,
                mut links,
                cursor,
            } => {
                if peek(input, cursor) != Some(ZWJ) {
                    return StructuredScan::Emit {
                        end: cursor,
                        kind: ScanKind::Zwj(build_zwj_sequence(components, links)),
                    };
                }

                let zwj_end = cursor + ZWJ.len_utf8();
                debug_assert!(zwj_end > cursor, "ZWJ scan must advance past the joiner");
                let (next_pos, variation_selectors) = consume_optional_selector_run(input, zwj_end);
                let Some(next_base) = peek(input, next_pos) else {
                    return StructuredScan::Emit {
                        end: cursor,
                        kind: ScanKind::Zwj(build_zwj_sequence(components, links)),
                    };
                };
                if !is_valid_zwj_component_base(next_base) {
                    return StructuredScan::Emit {
                        end: cursor,
                        kind: ScanKind::Zwj(build_zwj_sequence(components, links)),
                    };
                }

                links.push(ZwjLink {
                    variation_selectors,
                });
                let (next_component, new_cursor) = consume_component(input, next_pos, next_base);
                components.push(next_component);
                state = ZwjMachine::Confirmed {
                    components,
                    links,
                    cursor: new_cursor,
                };
            }
        }
    }
}

fn fallback_from_zwj_candidate(
    input: &str,
    start: usize,
    first_component: ZwjComponent,
    singleton_end: Option<usize>,
) -> StructuredScan {
    if let Some(end) = singleton_end {
        return StructuredScan::Emit {
            end,
            kind: ScanKind::Singleton {
                base: first_component.base,
                variation_selectors: first_component.variation_selectors_after_base,
            },
        };
    }

    StructuredScan::Passthrough {
        end: consume_passthrough_run(input, start),
    }
}

fn flush_passthrough<'a>(
    input: &'a str,
    items: &mut Vec<ScanItem<'a>>,
    passthrough_start: &mut Option<usize>,
    end: usize,
) {
    if let Some(start) = passthrough_start.take() {
        items.push(make_item(input, start, end, ScanKind::Passthrough));
    }
}

fn make_item(input: &str, start: usize, end: usize, kind: ScanKind) -> ScanItem<'_> {
    ScanItem {
        #[allow(clippy::string_slice)]
        raw: &input[start..end],
        span: start..end,
        kind,
    }
}

fn peek(input: &str, pos: usize) -> Option<char> {
    #[allow(clippy::string_slice)]
    input[pos..].chars().next()
}

/// Try to match a ZWJ chain starting at `pos`. Returns (`end_byte`, sequence).
/// Requires at least two components joined by ZWJ.
fn try_zwj(input: &str, pos: usize, first_base: char) -> Option<(usize, ZwjSequence)> {
    if !is_valid_zwj_component_base(first_base) {
        return None;
    }

    let (first_comp, mut cursor) = consume_component(input, pos, first_base);

    // Must have ZWJ after first component.
    if peek(input, cursor) != Some(ZWJ) {
        return None;
    }

    let mut components = vec![first_comp];
    let mut links = Vec::new();

    while peek(input, cursor) == Some(ZWJ) {
        let zwj_end = cursor + ZWJ.len_utf8();
        debug_assert!(zwj_end > cursor, "ZWJ scan must advance past the joiner");
        let (next_pos, variation_selectors) = consume_optional_selector_run(input, zwj_end);

        // Need a valid next-component base after ZWJ and any variation selectors. If
        // none exists, the trailing ZWJ is not part of the recognized chain.
        let Some(next_base) = peek(input, next_pos) else {
            break;
        };
        if !is_valid_zwj_component_base(next_base) {
            break;
        }

        links.push(ZwjLink {
            variation_selectors,
        });
        cursor = next_pos; // commit: consume the ZWJ and its trailing variation selectors
        let (comp, new_cursor) = consume_component(input, cursor, next_base);
        components.push(comp);
        cursor = new_cursor;
    }

    if components.len() >= 2 {
        Some((cursor, build_zwj_sequence(components, links)))
    } else {
        None
    }
}

fn build_zwj_sequence(mut components: Vec<ZwjComponent>, mut links: Vec<ZwjLink>) -> ZwjSequence {
    debug_assert_eq!(components.len(), links.len() + 1);

    #[allow(clippy::expect_used)]
    let last = components
        .pop()
        .expect("ZWJ sequence must contain at least one component");
    let mut sequence = ZwjSequence::Terminal(last);

    while let Some(head) = components.pop() {
        #[allow(clippy::expect_used)]
        let link = links
            .pop()
            .expect("ZWJ sequence must have exactly one fewer link than components");
        sequence = ZwjSequence::Joined {
            head,
            link,
            tail: Box::new(sequence),
        };
    }

    sequence
}

/// Consume one ZWJ component: base variation-selector* [emoji-modifier variation-selector*].
fn consume_component(input: &str, pos: usize, base: char) -> (ZwjComponent, usize) {
    let mut cursor = pos + base.len_utf8();
    debug_assert!(cursor > pos, "component scan must advance past the base");

    let (new_cursor, variation_selectors_after_base) = consume_optional_selector_run(input, cursor);
    cursor = new_cursor;

    let emoji_modifier = match peek(input, cursor) {
        Some(ch) if is_emoji_modifier(ch) => {
            cursor += ch.len_utf8();
            debug_assert!(
                cursor > pos,
                "component scan must advance past the emoji modifier"
            );
            Some(ch)
        }
        _ => None,
    };

    let (new_cursor, variation_selectors_after_modifier) =
        consume_optional_selector_run(input, cursor);
    cursor = new_cursor;

    (
        ZwjComponent {
            base,
            variation_selectors_after_base,
            emoji_modifier,
            variation_selectors_after_modifier,
        },
        cursor,
    )
}

fn consume_optional_selector_run(input: &str, pos: usize) -> (usize, Vec<VariationSelector>) {
    if peek(input, pos).is_some_and(is_variation_selector) {
        consume_selector_run(input, pos)
    } else {
        (pos, Vec::new())
    }
}

fn consume_selector_run(input: &str, pos: usize) -> (usize, Vec<VariationSelector>) {
    let mut cursor = pos;
    let mut variation_selectors = Vec::new();
    while let Some(ch) = peek(input, cursor) {
        let Some(vs) = VariationSelector::from_char(ch) else {
            break;
        };
        variation_selectors.push(vs);
        let next_cursor = cursor + ch.len_utf8();
        debug_assert!(
            next_cursor > cursor,
            "variation selector run must make forward progress"
        );
        cursor = next_cursor;
    }
    (cursor, variation_selectors)
}
