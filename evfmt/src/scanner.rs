//! Sequence-aware scanner for text/emoji variation sequences.
//!
//! Recognizes singletons, keycap sequences, ZWJ chains, and standalone
//! selector runs.
//!
//! Aside from coalescing arbitrary non-structural text into
//! [`ScanKind::Passthrough`], this scanner is purely structural and lossless.
//! Every non-passthrough [`ScanKind`] retains enough information to
//! reconstruct its own raw slice bit-for-bit. Concatenating all
//! [`ScanItem::raw`] values reconstructs the original input.

use std::ops::Range;

use crate::unicode;

#[cfg(any(test, fuzzing))]
mod legacy;
#[cfg(any(test, fuzzing))]
use self::legacy::scan_legacy;

/// Unicode variation selector 15 (text presentation).
pub const VS_TEXT: char = '\u{FE0E}';
/// Unicode variation selector 16 (emoji presentation).
pub const VS_EMOJI: char = '\u{FE0F}';
pub(crate) const ZWJ: char = '\u{200D}';
pub(crate) const KEYCAP_CAP: char = '\u{20E3}';

/// Returns true if the character is a variation selector (VS15 or VS16).
#[must_use]
pub(crate) fn is_variation_selector(ch: char) -> bool {
    ch == VS_TEXT || ch == VS_EMOJI
}

/// Returns true if the character is a valid keycap base (`#`, `*`, `0`–`9`).
#[must_use]
pub(crate) fn is_keycap_base(ch: char) -> bool {
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
pub enum ScanKind {
    /// Non-emoji content (plain text, ineligible characters, standalone ZWJ, etc.).
    Passthrough,
    /// Standalone variation selectors not attached to a recognized logical unit.
    StandaloneSelectors(Vec<char>),
    /// Single variation-sequence code point with trailing variation selectors.
    Singleton {
        /// The base character.
        base: char,
        /// Trailing variation selectors, in source order.
        selectors: Vec<char>,
    },
    /// Keycap sequence: base, trailing variation selectors, then U+20E3.
    Keycap {
        /// The base character (`#`, `*`, or a digit).
        base: char,
        /// Trailing variation selectors before U+20E3, in source order.
        selectors: Vec<char>,
    },
    /// ZWJ sequence: two or more components joined by U+200D.
    ///
    /// The sequence structure is lossless: selectors after a joiner are stored
    /// on the link itself rather than discarded or reassigned to a component.
    Zwj(ZwjSequence),
}

/// One component of a ZWJ sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZwjComponent {
    /// The base character of this component.
    pub base: char,
    /// Variation selectors immediately after the base and before any emoji
    /// modifier.
    pub selectors_after_base: Vec<char>,
    /// Unicode emoji modifier, if present.
    pub emoji_modifier: Option<char>,
    /// Variation selectors after the base when no modifier is present, or after
    /// the emoji modifier when one is present.
    pub selectors_after_modifier: Vec<char>,
}

/// One explicit U+200D join between two ZWJ components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZwjLink {
    /// Variation selectors immediately after the joiner and before the next
    /// component base.
    pub selectors: Vec<char>,
}

/// A lossless recursive representation of a ZWJ sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Return the only selector that can directly affect the base character.
///
/// Unicode variation sequences are a base character followed by a single
/// variation selector. Any additional selectors in the same run trail the
/// first selector rather than the base itself.
#[must_use]
pub(crate) fn effective_selector(selectors: &[char]) -> Option<char> {
    selectors.first().copied()
}

/// Return the selector run at the terminal selector-bearing position of a ZWJ
/// component.
#[must_use]
pub(crate) fn zwj_component_terminal_selectors(component: &ZwjComponent) -> &[char] {
    if component.emoji_modifier.is_some() {
        &component.selectors_after_modifier
    } else {
        &component.selectors_after_base
    }
}

/// Return the only selector that can directly affect the terminal position of a
/// ZWJ component.
#[must_use]
pub(crate) fn zwj_component_effective_selector(component: &ZwjComponent) -> Option<char> {
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
    // 1. Standalone selector run: VS not attached to a recognized logical unit
    // 2. Keycap: base `[VS] U+20E3`
    // 3. ZWJ chain: two+ components joined by `U+200D`
    // 4. Singleton: variation-sequence character `[VS]`
    // 5. Passthrough: everything else, coalesced into runs
    //
    // Keycap must be checked before singleton so that `#️⃣` stays a keycap
    // instead of splitting into `#` plus a stray selector run. ZWJ must also
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
            let (end, selectors) = consume_selector_run(input, pos);
            items.push(make_item(
                input,
                pos,
                end,
                ScanKind::StandaloneSelectors(selectors),
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
    let (after_base_selectors, selectors_after_base) =
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
                selectors: selectors_after_base,
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
        let (component_end, selectors_after_modifier) =
            consume_optional_selector_run(input, cursor);
        let first_component = ZwjComponent {
            base: ch,
            selectors_after_base: selectors_after_base.clone(),
            emoji_modifier,
            selectors_after_modifier,
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
                selectors: selectors_after_base,
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
                let (next_pos, selectors) = consume_optional_selector_run(input, zwj_end);
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
                let links = vec![ZwjLink { selectors }];
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
                let (next_pos, selectors) = consume_optional_selector_run(input, zwj_end);
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

                links.push(ZwjLink { selectors });
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
                selectors: first_component.selectors_after_base,
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

pub(crate) fn peek(input: &str, pos: usize) -> Option<char> {
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
        let (next_pos, selectors) = consume_optional_selector_run(input, zwj_end);

        // Need a valid next-component base after ZWJ and any selectors. If
        // none exists, the trailing ZWJ is not part of the recognized chain.
        let Some(next_base) = peek(input, next_pos) else {
            break;
        };
        if !is_valid_zwj_component_base(next_base) {
            break;
        }

        links.push(ZwjLink { selectors });
        cursor = next_pos; // commit: consume the ZWJ and its trailing selectors
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

/// Consume one ZWJ component: base selector* [emoji-modifier selector*].
fn consume_component(input: &str, pos: usize, base: char) -> (ZwjComponent, usize) {
    let mut cursor = pos + base.len_utf8();
    debug_assert!(cursor > pos, "component scan must advance past the base");

    let (new_cursor, selectors_after_base) = consume_optional_selector_run(input, cursor);
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

    let (new_cursor, selectors_after_modifier) = consume_optional_selector_run(input, cursor);
    cursor = new_cursor;

    (
        ZwjComponent {
            base,
            selectors_after_base,
            emoji_modifier,
            selectors_after_modifier,
        },
        cursor,
    )
}

fn consume_optional_selector_run(input: &str, pos: usize) -> (usize, Vec<char>) {
    if peek(input, pos).is_some_and(is_variation_selector) {
        consume_selector_run(input, pos)
    } else {
        (pos, Vec::new())
    }
}

fn consume_selector_run(input: &str, pos: usize) -> (usize, Vec<char>) {
    let mut cursor = pos;
    let mut selectors = Vec::new();
    while let Some(ch) = peek(input, cursor) {
        if !is_variation_selector(ch) {
            break;
        }
        selectors.push(ch);
        let next_cursor = cursor + ch.len_utf8();
        debug_assert!(
            next_cursor > cursor,
            "selector run must make forward progress"
        );
        cursor = next_cursor;
    }
    (cursor, selectors)
}

// --- Tests ---

#[cfg(test)]
mod tests {
    // Tests use unwrap/panic for concise assertions — a panic IS the failure signal.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use proptest::prelude::*;
    use proptest::sample::select;

    use super::*;
    use crate::unicode::DefaultSide;

    /// Reconstruct input from scan items.
    fn reconstruct(items: &[ScanItem<'_>]) -> String {
        items.iter().map(|it| it.raw).collect()
    }

    fn select_chars(chars: &'static [char]) -> BoxedStrategy<char> {
        select(chars).boxed()
    }

    fn selector_run_strategy(max_len: usize) -> BoxedStrategy<Vec<char>> {
        prop::collection::vec(prop_oneof![Just(VS_TEXT), Just(VS_EMOJI)], 0..max_len).boxed()
    }

    fn interesting_char_strategy() -> BoxedStrategy<char> {
        let mut chars: Vec<char> = unicode::variation_sequence_chars().collect();
        chars.extend([
            VS_TEXT,
            VS_EMOJI,
            ZWJ,
            KEYCAP_CAP,
            '#',
            '*',
            '0',
            '1',
            '9',
            'A',
            'a',
            '\u{00A9}',
            '\u{231A}',
            '\u{2764}',
            '\u{1F44D}',
            '\u{1F466}',
            '\u{1F468}',
            '\u{1F525}',
            '\u{1F3FB}',
            '\u{1F3FF}',
        ]);
        chars.sort_unstable();
        chars.dedup();
        select(chars).boxed()
    }

    fn scannerish_fragment_strategy() -> BoxedStrategy<String> {
        prop_oneof![
            8 => interesting_char_strategy().prop_map(|ch| ch.to_string()),
            3 => (interesting_char_strategy(), selector_run_strategy(4))
                .prop_map(|(base, selectors)| {
                    let mut s = String::new();
                    s.push(base);
                    s.extend(selectors);
                    s
                }),
            2 => (select_chars(&['#', '*', '0', '1', '9']), selector_run_strategy(3))
                .prop_map(|(base, selectors)| {
                    let mut s = String::new();
                    s.push(base);
                    s.extend(selectors);
                    s.push(KEYCAP_CAP);
                    s
                }),
            2 => (
                select_chars(&['\u{2764}', '\u{1F468}', '\u{1F525}', '\u{1F44D}']),
                selector_run_strategy(3),
                prop::option::of(select_chars(&['\u{1F3FB}', '\u{1F3FF}'])),
                selector_run_strategy(3),
                selector_run_strategy(3),
                select_chars(&['\u{2764}', '\u{1F466}', '\u{1F468}', '\u{1F525}', '\u{1F44D}']),
            ).prop_map(|(base1, selectors1, modifier1, selectors2, joiner_selectors, base2)| {
                let mut s = String::new();
                s.push(base1);
                s.extend(selectors1);
                if let Some(modifier) = modifier1 {
                    s.push(modifier);
                    s.extend(selectors2);
                }
                s.push(ZWJ);
                s.extend(joiner_selectors);
                s.push(base2);
                s
            }),
            1 => (
                select_chars(&['\u{2764}', '\u{1F525}', '\u{1F44D}']),
                select_chars(&['\u{2764}', '\u{1F466}', '\u{1F525}']),
            ).prop_map(|(base1, base2)| {
                let mut s = String::new();
                s.push(base1);
                s.push(ZWJ);
                s.push(base2);
                s.push(ZWJ);
                s.push(VS_EMOJI);
                s.push(ZWJ);
                s
            }),
            3 => any::<char>().prop_map(|ch| ch.to_string()),
        ]
        .boxed()
    }

    fn scannerish_input_strategy() -> BoxedStrategy<String> {
        prop::collection::vec(scannerish_fragment_strategy(), 0..32)
            .prop_map(|parts| parts.concat())
            .boxed()
    }

    // --- Losslessness ---

    #[test]
    fn test_lossless_plain_text() {
        let input = "Hello, world!";
        assert_eq!(reconstruct(&scan(input)), input);
    }

    #[test]
    fn test_lossless_singleton() {
        for input in ["#", "#\u{FE0F}", "\u{00A9}\u{FE0E}", "\u{2728}"] {
            assert_eq!(reconstruct(&scan(input)), input, "input: {input:?}");
        }
    }

    #[test]
    fn test_lossless_keycap() {
        for input in ["#\u{FE0F}\u{20E3}", "#\u{20E3}", "#\u{FE0E}\u{20E3}"] {
            assert_eq!(reconstruct(&scan(input)), input, "input: {input:?}");
        }
    }

    #[test]
    fn test_lossless_zwj() {
        // ❤️ (2764) ZWJ 🔥 (1F525)
        let input = "\u{2764}\u{200D}\u{1F525}";
        assert_eq!(reconstruct(&scan(input)), input);
    }

    #[test]
    fn test_lossless_orphaned() {
        for input in ["\u{FE0F}", "\u{FE0E}hello", "A\u{FE0F}"] {
            assert_eq!(reconstruct(&scan(input)), input, "input: {input:?}");
        }
    }

    #[test]
    fn test_lossless_mixed() {
        let input =
            "Hello #\u{FE0F}\u{20E3} world \u{00A9}\u{FE0E} \u{2764}\u{FE0F}\u{200D}\u{1F525}";
        assert_eq!(reconstruct(&scan(input)), input);
    }

    // --- Scan kinds ---

    #[test]
    fn test_scan_passthrough() {
        let items = scan("Hello");
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0].kind, ScanKind::Passthrough));
    }

    #[test]
    fn test_scan_standalone_selector_at_start() {
        let items = scan("\u{FE0F}");
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0].kind,
            ScanKind::StandaloneSelectors(ref selectors) if selectors == &[VS_EMOJI]
        ));
    }

    #[test]
    fn test_scan_standalone_selector_after_ineligible() {
        let items = scan("A\u{FE0F}");
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0].kind, ScanKind::Passthrough));
        assert!(matches!(
            items[1].kind,
            ScanKind::StandaloneSelectors(ref selectors) if selectors == &[VS_EMOJI]
        ));
    }

    #[test]
    fn test_is_variation_selector_only_accepts_vs15_and_vs16() {
        assert!(is_variation_selector(VS_TEXT));
        assert!(is_variation_selector(VS_EMOJI));
        assert!(!is_variation_selector('A'));
        assert!(!is_variation_selector(ZWJ));
    }

    #[test]
    fn test_consume_optional_selector_run_advances_over_variation_selectors() {
        let input = "\u{FE0F}\u{FE0E}A";
        let (end, selectors) = consume_optional_selector_run(input, 0);

        assert_eq!(selectors, vec![VS_EMOJI, VS_TEXT]);
        assert_eq!(end, "\u{FE0F}\u{FE0E}".len());
    }

    #[test]
    fn test_non_keycap_base_before_cap_is_passthrough() {
        let input = "A\u{20E3}";
        let items = scan(input);
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0].kind, ScanKind::Passthrough));
        assert_eq!(items[0].raw, input);
    }

    #[test]
    fn test_scan_singleton_bare() {
        let items = scan("\u{00A9}");
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0].kind,
            ScanKind::Singleton {
                base: '\u{00A9}',
                ref selectors
            } if selectors.is_empty()
        ));
    }

    #[test]
    fn test_scan_singleton_with_vs() {
        let items = scan("#\u{FE0F}");
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0].kind,
            ScanKind::Singleton {
                base: '#',
                ref selectors
            } if selectors == &[VS_EMOJI]
        ));
    }

    #[test]
    fn test_scan_crosscheck_runs_both_scanners() {
        let input = "#\u{FE0F}\u{20E3}\u{200D}";
        let (legacy, state_machine) = scan_crosscheck(input);

        assert_eq!(legacy, scan_legacy(input));
        assert_eq!(state_machine, scan_state_machine(input));
        assert_eq!(state_machine, scan(input));
    }

    #[test]
    fn test_scan_singleton_conflicting_selectors_stay_attached() {
        // # + FE0F + FE0E stays one logical singleton with a selector run.
        let items = scan("#\u{FE0F}\u{FE0E}");
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0].kind,
            ScanKind::Singleton {
                base: '#',
                ref selectors
            } if selectors == &[VS_EMOJI, VS_TEXT]
        ));
    }

    #[test]
    fn test_scan_standalone_selector_run() {
        let items = scan("\u{FE0F}\u{FE0E}");
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0].kind,
            ScanKind::StandaloneSelectors(ref selectors) if selectors == &[VS_EMOJI, VS_TEXT]
        ));
    }

    #[test]
    fn test_scan_keycap_correct() {
        let items = scan("#\u{FE0F}\u{20E3}");
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0].kind,
            ScanKind::Keycap {
                base: '#',
                ref selectors
            } if selectors == &[VS_EMOJI]
        ));
    }

    #[test]
    fn test_scan_keycap_bare() {
        let items = scan("#\u{20E3}");
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0].kind,
            ScanKind::Keycap {
                base: '#',
                ref selectors
            } if selectors.is_empty()
        ));
    }

    #[test]
    fn test_scan_keycap_wrong_vs() {
        let items = scan("#\u{FE0E}\u{20E3}");
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0].kind,
            ScanKind::Keycap {
                base: '#',
                ref selectors
            } if selectors == &[VS_TEXT]
        ));
    }

    #[test]
    fn test_scan_keycap_all_bases() {
        for base in ['#', '*', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'] {
            let input = format!("{base}\u{FE0F}\u{20E3}");
            let items = scan(&input);
            assert_eq!(items.len(), 1, "base: {base}");
            assert!(matches!(
                items[0].kind,
                ScanKind::Keycap {
                    ref selectors,
                    ..
                } if selectors == &[VS_EMOJI]
            ));
        }
    }

    #[test]
    fn test_scan_zwj_basic() {
        // ❤️ (2764) ZWJ 🔥 (1F525)
        let input = "\u{2764}\u{200D}\u{1F525}";
        let items = scan(input);
        assert_eq!(items.len(), 1);
        if let ScanKind::Zwj(ref seq) = items[0].kind {
            match seq {
                ZwjSequence::Joined { head, tail, .. } => {
                    assert_eq!(head.base, '\u{2764}');
                    match tail.as_ref() {
                        ZwjSequence::Terminal(last) => assert_eq!(last.base, '\u{1F525}'),
                        ZwjSequence::Joined { .. } => panic!("expected terminal tail"),
                    }
                }
                ZwjSequence::Terminal(_) => panic!("expected joined sequence"),
            }
        } else {
            panic!("expected Zwj");
        }
    }

    #[test]
    fn test_scan_zwj_with_fe0f() {
        // ❤️ FE0F ZWJ 🔥
        let input = "\u{2764}\u{FE0F}\u{200D}\u{1F525}";
        let items = scan(input);
        assert_eq!(items.len(), 1);
        if let ScanKind::Zwj(ref seq) = items[0].kind {
            match seq {
                ZwjSequence::Joined { head, .. } => {
                    assert_eq!(head.selectors_after_base, vec![VS_EMOJI]);
                    assert!(head.selectors_after_modifier.is_empty());
                }
                ZwjSequence::Terminal(_) => panic!("expected joined sequence"),
            }
        } else {
            panic!("expected Zwj");
        }
    }

    #[test]
    fn test_scan_zwj_with_emoji_modifier() {
        // 👨 🏻 ZWJ 👦
        let input = "\u{1F468}\u{1F3FB}\u{200D}\u{1F466}";
        let items = scan(input);
        assert_eq!(items.len(), 1);
        if let ScanKind::Zwj(ref seq) = items[0].kind {
            match seq {
                ZwjSequence::Joined { head, tail, .. } => {
                    assert_eq!(head.base, '\u{1F468}');
                    assert_eq!(head.emoji_modifier, Some('\u{1F3FB}'));
                    match tail.as_ref() {
                        ZwjSequence::Terminal(last) => assert_eq!(last.base, '\u{1F466}'),
                        ZwjSequence::Joined { .. } => panic!("expected terminal tail"),
                    }
                }
                ZwjSequence::Terminal(_) => panic!("expected joined sequence"),
            }
        } else {
            panic!("expected Zwj");
        }
    }

    #[test]
    fn test_scan_zwj_preserves_selector_run_after_joiner() {
        let input = "\u{1F525}\u{200D}\u{FE0F}\u{2764}";
        let items = scan(input);
        assert_eq!(items.len(), 1);
        if let ScanKind::Zwj(ref seq) = items[0].kind {
            match seq {
                ZwjSequence::Joined { head, link, tail } => {
                    assert_eq!(head.base, '\u{1F525}');
                    assert_eq!(link.selectors, vec![VS_EMOJI]);
                    match tail.as_ref() {
                        ZwjSequence::Terminal(last) => {
                            assert_eq!(last.base, '\u{2764}');
                            assert!(last.selectors_after_base.is_empty());
                            assert!(last.selectors_after_modifier.is_empty());
                        }
                        ZwjSequence::Joined { .. } => panic!("expected terminal tail"),
                    }
                }
                ZwjSequence::Terminal(_) => panic!("expected joined sequence"),
            }
            assert_eq!(items[0].raw, input);
        } else {
            panic!("expected Zwj");
        }
    }

    #[test]
    fn test_try_zwj_stops_before_trailing_joiner_selector_joiner() {
        let input = "\u{2764}\u{200D}\u{1F525}\u{200D}\u{FE0F}\u{200D}";
        let (end, seq) = try_zwj(input, 0, '\u{2764}').expect("expected valid ZWJ prefix");

        assert_eq!(end, "\u{2764}\u{200D}\u{1F525}".len());
        match seq {
            ZwjSequence::Joined { head, tail, .. } => {
                assert_eq!(head.base, '\u{2764}');
                match tail.as_ref() {
                    ZwjSequence::Terminal(last) => assert_eq!(last.base, '\u{1F525}'),
                    ZwjSequence::Joined { .. } => panic!("expected terminal tail"),
                }
            }
            ZwjSequence::Terminal(_) => panic!("expected joined sequence"),
        }
    }

    #[test]
    fn test_scan_zwj_leaves_trailing_joiner_selector_joiner_unconsumed() {
        let input = "\u{2764}\u{200D}\u{1F525}\u{200D}\u{FE0F}\u{200D}";
        let items = scan(input);

        assert_eq!(items.len(), 4);
        assert!(matches!(items[0].kind, ScanKind::Zwj(_)));
        assert_eq!(items[0].raw, "\u{2764}\u{200D}\u{1F525}");
        assert!(matches!(items[1].kind, ScanKind::Passthrough));
        assert_eq!(items[1].raw, "\u{200D}");
        assert!(matches!(
            items[2].kind,
            ScanKind::StandaloneSelectors(ref selectors) if selectors == &[VS_EMOJI]
        ));
        assert_eq!(items[2].raw, "\u{FE0F}");
        assert!(matches!(items[3].kind, ScanKind::Passthrough));
        assert_eq!(items[3].raw, "\u{200D}");
    }

    #[test]
    fn test_scan_zwj_keeps_longest_valid_prefix_before_invalid_post_joiner_base() {
        let input = "\u{2764}\u{200D}\u{1F525}\u{200D}\u{1F44D}\u{200D}\u{FE0F}\u{200D}";
        let items = scan(input);

        assert_eq!(items.len(), 4);
        assert_eq!(items[0].raw, "\u{2764}\u{200D}\u{1F525}\u{200D}\u{1F44D}");
        if let ScanKind::Zwj(ref seq) = items[0].kind {
            match seq {
                ZwjSequence::Joined { head, tail, .. } => {
                    assert_eq!(head.base, '\u{2764}');
                    match tail.as_ref() {
                        ZwjSequence::Joined { head, tail, .. } => {
                            assert_eq!(head.base, '\u{1F525}');
                            match tail.as_ref() {
                                ZwjSequence::Terminal(last) => assert_eq!(last.base, '\u{1F44D}'),
                                ZwjSequence::Joined { .. } => panic!("expected terminal tail"),
                            }
                        }
                        ZwjSequence::Terminal(_) => panic!("expected three components"),
                    }
                }
                ZwjSequence::Terminal(_) => panic!("expected joined sequence"),
            }
        } else {
            panic!("expected Zwj");
        }
        assert_eq!(items[1].raw, "\u{200D}");
        assert!(matches!(items[1].kind, ScanKind::Passthrough));
        assert_eq!(items[2].raw, "\u{FE0F}");
        assert!(matches!(
            items[2].kind,
            ScanKind::StandaloneSelectors(ref selectors) if selectors == &[VS_EMOJI]
        ));
        assert_eq!(items[3].raw, "\u{200D}");
        assert!(matches!(items[3].kind, ScanKind::Passthrough));
    }

    #[test]
    fn test_state_machine_matches_legacy_for_ineligible_base_selector_zwj() {
        let input = "\u{00A1}\u{FE0E}\u{200D}";
        let legacy = scan_legacy(input);
        let state_machine = scan_state_machine(input);

        assert_eq!(state_machine, legacy);
        assert_eq!(state_machine.len(), 3);
        assert!(matches!(state_machine[0].kind, ScanKind::Passthrough));
        assert_eq!(state_machine[0].raw, "\u{00A1}");
        assert!(matches!(
            state_machine[1].kind,
            ScanKind::StandaloneSelectors(ref selectors) if selectors == &[VS_TEXT]
        ));
        assert_eq!(state_machine[1].raw, "\u{FE0E}");
        assert!(matches!(state_machine[2].kind, ScanKind::Passthrough));
        assert_eq!(state_machine[2].raw, "\u{200D}");
    }

    #[test]
    fn test_state_machine_matches_legacy_for_singleton_then_modifier_then_zwj() {
        let input = "#\u{1F3FB}\u{200D}";
        let legacy = scan_legacy(input);
        let state_machine = scan_state_machine(input);

        assert_eq!(state_machine, legacy);
        assert_eq!(state_machine.len(), 2);
        assert!(matches!(
            state_machine[0].kind,
            ScanKind::Singleton {
                base: '#',
                ref selectors
            } if selectors.is_empty()
        ));
        assert_eq!(state_machine[0].raw, "#");
        assert!(matches!(state_machine[1].kind, ScanKind::Passthrough));
        assert_eq!(state_machine[1].raw, "\u{1F3FB}\u{200D}");
    }

    proptest! {
        #[test]
        fn proptest_state_machine_matches_legacy_scanner(
            input in scannerish_input_strategy()
        ) {
            let legacy = scan_legacy(&input);
            let state_machine = scan_state_machine(&input);

            prop_assert_eq!(&state_machine, &legacy);
            prop_assert_eq!(reconstruct(&state_machine), input.clone());
            prop_assert_eq!(reconstruct(&legacy), input);
        }
    }

    #[test]
    fn test_scan_does_not_treat_bare_zwj_as_component_base() {
        let input = "\u{200D}\u{200D}#";
        let items = scan(input);
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0].kind, ScanKind::Passthrough));
        assert_eq!(items[0].raw, "\u{200D}\u{200D}");
        assert!(matches!(
            items[1].kind,
            ScanKind::Singleton {
                base: '#',
                ref selectors
            } if selectors.is_empty()
        ));
    }

    #[test]
    fn test_scan_does_not_treat_keycap_cap_as_component_base() {
        let input = "\u{20E3}\u{200D}#";
        let items = scan(input);
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0].kind, ScanKind::Passthrough));
        assert_eq!(items[0].raw, "\u{20E3}\u{200D}");
        assert!(matches!(
            items[1].kind,
            ScanKind::Singleton {
                base: '#',
                ref selectors
            } if selectors.is_empty()
        ));
    }

    #[test]
    fn test_passthrough_stops_before_non_variation_zwj_start() {
        let input = "\u{200D}A\u{200D}\u{231A}";
        let items = scan(input);
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0].kind, ScanKind::Passthrough));
        assert_eq!(items[0].raw, "\u{200D}");
        assert!(matches!(items[1].kind, ScanKind::Zwj(_)));
    }

    // -------------------------------------------------------------------
    // AUDIT NOTE — Conformance tests against official Unicode emoji sequence data.
    //
    // These verify structural assumptions the scanner and formatter depend on:
    //   - Keycap bases are exactly #*0-9 (test_conformance_keycap_bases)
    //   - Only FE0F keycaps exist, no bare or FE0E forms (test_conformance_keycap_fe0f_only)
    //   - ZWJ sequences always contain ZWJ, never FE0E (test_conformance_zwj_structure)
    //   - Text-default ZWJ components have FE0F (test_conformance_zwj_text_default_has_fe0f)
    //   - All official sequences scan as the expected kind (test_conformance_all_sequences_classifiable)
    //
    // If a future Unicode version violates any of these, these tests fail.
    // -------------------------------------------------------------------

    /// Parse a line from emoji-sequences.txt or emoji-zwj-sequences.txt.
    /// Returns (`code_points`, `type_field`) or `None` for comments/blanks.
    fn parse_sequence_line(line: &str) -> Option<(Vec<u32>, String)> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let parts: Vec<&str> = line.splitn(3, ';').collect();
        if parts.len() < 2 {
            return None;
        }
        let cp_part = parts[0].trim();
        let type_field = parts[1].trim().to_owned();

        // Handle ranges like "231A..231B"
        if cp_part.contains("..") {
            let range_parts: Vec<&str> = cp_part.split("..").collect();
            let start = u32::from_str_radix(range_parts[0].trim(), 16).ok()?;
            let end = u32::from_str_radix(range_parts[1].trim(), 16).ok()?;
            // Return range as individual entries — but for our purposes,
            // Basic_Emoji ranges are single code points, not sequences.
            // We'll handle them specially in the test.
            return Some((vec![start, end], type_field));
        }

        let cps: Vec<u32> = cp_part
            .split_whitespace()
            .filter_map(|s| u32::from_str_radix(s, 16).ok())
            .collect();
        if cps.is_empty() {
            return None;
        }
        Some((cps, type_field))
    }

    #[test]
    fn test_conformance_keycap_bases() {
        // Verify keycap bases are exactly #*0-9.
        let data = include_str!("../data/emoji-sequences.txt");
        let mut keycap_bases: Vec<u32> = Vec::new();

        for line in data.lines() {
            if let Some((cps, type_field)) = parse_sequence_line(line)
                && type_field == "Emoji_Keycap_Sequence"
            {
                // Keycap: base FE0F 20E3
                assert_eq!(cps.len(), 3, "keycap sequence has wrong length: {cps:?}");
                assert_eq!(cps[1], 0xFE0F, "keycap middle must be FE0F");
                assert_eq!(cps[2], 0x20E3, "keycap end must be 20E3");
                keycap_bases.push(cps[0]);
            }
        }

        keycap_bases.sort_unstable();
        let expected: Vec<u32> = vec![
            0x0023, 0x002A, // # *
            0x0030, 0x0031, 0x0032, 0x0033, 0x0034, // 0-4
            0x0035, 0x0036, 0x0037, 0x0038, 0x0039, // 5-9
        ];
        assert_eq!(
            keycap_bases, expected,
            "keycap bases don't match expected set"
        );
    }

    #[test]
    fn test_conformance_keycap_fe0f_only() {
        // Verify no bare or FE0E keycap variants exist as distinct sequences.
        let data = include_str!("../data/emoji-sequences.txt");

        for line in data.lines() {
            if let Some((cps, _type_field)) = parse_sequence_line(line) {
                // Check for any sequence that looks like base 20E3 (bare keycap)
                // or base FE0E 20E3 (text keycap).
                assert!(
                    !(cps.len() == 2 && cps[1] == 0x20E3),
                    "bare keycap sequence found: U+{:04X} U+20E3 — \
                     our repair assumes only FE0F form is sanctioned",
                    cps[0]
                );
                assert!(
                    !(cps.len() == 3 && cps[1] == 0xFE0E && cps[2] == 0x20E3),
                    "FE0E keycap sequence found: U+{:04X} U+FE0E U+20E3 — \
                     our repair assumes only FE0F form is sanctioned",
                    cps[0]
                );
            }
        }
    }

    #[test]
    fn test_conformance_zwj_structure() {
        // Verify every ZWJ sequence:
        // 1. Contains at least one ZWJ (200D)
        // 2. All FE0F appear after a base (not standalone)
        // 3. No FE0E appears in any ZWJ sequence
        let data = include_str!("../data/emoji-zwj-sequences.txt");
        let mut count = 0;

        for line in data.lines() {
            if let Some((cps, type_field)) = parse_sequence_line(line) {
                if type_field != "RGI_Emoji_ZWJ_Sequence" {
                    continue;
                }
                count += 1;

                assert!(cps.contains(&0x200D), "ZWJ sequence missing ZWJ: {cps:?}");
                assert!(
                    !cps.contains(&0xFE0E),
                    "ZWJ sequence contains FE0E: {cps:?} — \
                     our repair assumes no text-presentation ZWJ sequences",
                );

                // FE0F should only appear after a base, not at position 0
                for (i, &cp) in cps.iter().enumerate() {
                    if cp == 0xFE0F {
                        assert!(i > 0, "FE0F at start of ZWJ sequence: {cps:?}");
                    }
                }
            }
        }

        assert!(
            count > 0,
            "no ZWJ sequences found — data file may be empty/malformed"
        );
    }

    #[test]
    fn test_conformance_zwj_text_default_has_fe0f() {
        // For each ZWJ sequence, verify that text-default variation-sequence
        // components WITHOUT an emoji modifier have FE0F. Components
        // with an emoji modifier don't need FE0F (modifier implies emoji).
        let data = include_str!("../data/emoji-zwj-sequences.txt");
        let mut checked = 0;

        for line in data.lines() {
            if let Some((cps, type_field)) = parse_sequence_line(line) {
                if type_field != "RGI_Emoji_ZWJ_Sequence" {
                    continue;
                }

                // Walk components: split by ZWJ, check each.
                let mut i = 0;
                while i < cps.len() {
                    let base_cp = cps[i];
                    if base_cp == 0x200D {
                        i += 1;
                        continue;
                    }

                    // Check if next is a Unicode emoji modifier
                    let next = cps.get(i + 1).copied().unwrap_or(0);
                    let has_modifier = char::from_u32(next).is_some_and(unicode::is_emoji_modifier);

                    // Check if this base has text-default variation-sequence data
                    if let Some(ch) = char::from_u32(base_cp)
                        && let Some(info) = unicode::variation_sequence_info(ch)
                        && info.default_side == DefaultSide::Text
                        && !has_modifier
                    {
                        // Text-default without modifier → must have FE0F after
                        assert_eq!(
                            cps.get(i + 1).copied(),
                            Some(0xFE0F),
                            "text-default U+{base_cp:04X} in ZWJ without FE0F: {cps:?}"
                        );
                        checked += 1;
                    }

                    // Skip past this component (base [emoji_modifier] [FE0F])
                    i += 1;
                    while i < cps.len() && cps[i] != 0x200D {
                        i += 1;
                    }
                }
            }
        }

        assert!(
            checked > 0,
            "no text-default ZWJ components found to verify"
        );
    }

    #[test]
    fn test_conformance_all_sequences_classifiable() {
        // Every official keycap and ZWJ sequence must be scannable and
        // produce exactly one item of the expected kind.

        // Keycap sequences
        let seq_data = include_str!("../data/emoji-sequences.txt");
        for line in seq_data.lines() {
            if let Some((cps, type_field)) = parse_sequence_line(line) {
                if type_field != "Emoji_Keycap_Sequence" {
                    continue;
                }
                let s: String = cps.iter().filter_map(|&cp| char::from_u32(cp)).collect();
                let items = scan(&s);
                assert_eq!(
                    items.len(),
                    1,
                    "keycap sequence didn't scan as single item: {cps:?} → {items:?}"
                );
                assert!(
                    matches!(items[0].kind, ScanKind::Keycap { .. }),
                    "keycap sequence scanned as wrong kind: {:?} → {:?}",
                    cps,
                    items[0].kind
                );
            }
        }

        // ZWJ sequences
        let zwj_data = include_str!("../data/emoji-zwj-sequences.txt");
        let mut zwj_count = 0;
        for line in zwj_data.lines() {
            if let Some((cps, type_field)) = parse_sequence_line(line) {
                if type_field != "RGI_Emoji_ZWJ_Sequence" {
                    continue;
                }
                let s: String = cps.iter().filter_map(|&cp| char::from_u32(cp)).collect();
                let items = scan(&s);
                assert_eq!(
                    items.len(),
                    1,
                    "ZWJ sequence didn't scan as single item: {cps:X?} → {items:#?}"
                );
                assert!(
                    matches!(items[0].kind, ScanKind::Zwj(_)),
                    "ZWJ sequence scanned as wrong kind: {:X?} → {:?}",
                    cps,
                    items[0].kind
                );
                zwj_count += 1;
            }
        }

        assert!(zwj_count > 100, "too few ZWJ sequences found: {zwj_count}");
    }
}
