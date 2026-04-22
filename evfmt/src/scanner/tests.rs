use super::*;
use crate::formatter::{FormatResult, format_text};
use crate::policy::Policy;
use crate::unicode;

/// Collect scan items into a Vec for testing.
fn scan(input: &str) -> Vec<ScanItem<'_>> {
    super::scan(input).collect()
}

/// Reconstruct input from scan items.
fn reconstruct(items: &[ScanItem<'_>]) -> String {
    items.iter().map(|it| it.raw).collect()
}

#[derive(Debug, Clone, Copy)]
enum ExpectedKind {
    Passthrough,
    UnsanctionedPresentationSelectors,
    EmojiSequence,
}

impl ExpectedKind {
    fn matches(self, actual: &ScanKind) -> bool {
        matches!(
            (self, actual),
            (Self::Passthrough, ScanKind::Passthrough)
                | (
                    Self::UnsanctionedPresentationSelectors,
                    ScanKind::UnsanctionedPresentationSelectors(_)
                )
                | (Self::EmojiSequence, ScanKind::EmojiSequence(_))
        )
    }
}

fn assert_scan_items(input: &str, expected: &[(&str, ExpectedKind)]) {
    let items = scan(input);

    let actual_raw: Vec<_> = items.iter().map(|item| item.raw).collect();
    let expected_raw: Vec<_> = expected.iter().map(|(raw, _)| *raw).collect();
    assert_eq!(actual_raw, expected_raw);

    let mut start = 0;
    for (item, (raw, kind)) in items.iter().zip(expected) {
        let end = start + raw.len();
        assert_eq!(item.span, start..end, "raw item: {raw:?}");
        assert!(
            kind.matches(&item.kind),
            "raw item {raw:?} had kind {:?}, expected {kind:?}",
            item.kind
        );
        start = end;
    }
    assert_eq!(start, input.len());
}

fn default_policy() -> Policy {
    Policy::default()
}

fn assert_format(input: &str, expected: &FormatResult) {
    assert_eq!(&format_text(input, &default_policy()), expected);
}

// --- Losslessness ---

#[test]
fn scanner_is_lossless_for_representative_inputs() {
    for input in [
        "Hello, world!",
        "#",
        "#\u{FE0F}",
        "\u{00A9}\u{FE0E}",
        "\u{2728}",
        "#\u{FE0F}\u{20E3}",
        "#\u{20E3}",
        "#\u{FE0E}\u{20E3}",
        "\u{2764}\u{200D}\u{1F525}",
        "\u{FE0F}",
        "\u{FE0E}hello",
        "A\u{FE0F}",
        "Hello #\u{FE0F}\u{20E3} world \u{00A9}\u{FE0E} \u{2764}\u{FE0F}\u{200D}\u{1F525}",
    ] {
        assert_eq!(reconstruct(&scan(input)), input, "input: {input:?}");
    }
}

// --- Public scan boundaries and coarse classification ---

#[test]
fn scanner_exposes_stable_item_boundaries() {
    use ExpectedKind::{
        EmojiSequence as Emoji, Passthrough as Pass, UnsanctionedPresentationSelectors as Selectors,
    };

    for (input, expected) in [
        ("Hello", vec![("Hello", Pass)]),
        ("\u{FE0F}", vec![("\u{FE0F}", Selectors)]),
        ("A\u{FE0F}", vec![("A", Pass), ("\u{FE0F}", Selectors)]),
        ("A\u{20E3}", vec![("A\u{20E3}", Pass)]),
        ("\u{00A9}", vec![("\u{00A9}", Emoji)]),
        ("#\u{FE0F}\u{FE0E}", vec![("#\u{FE0F}\u{FE0E}", Emoji)]),
        ("\u{FE0F}\u{FE0E}", vec![("\u{FE0F}\u{FE0E}", Selectors)]),
        ("#\u{FE0F}\u{20E3}", vec![("#\u{FE0F}\u{20E3}", Emoji)]),
        ("#\u{20E3}", vec![("#\u{20E3}", Emoji)]),
        ("#\u{FE0E}\u{20E3}", vec![("#\u{FE0E}\u{20E3}", Emoji)]),
        (
            "\u{1F1E6}\u{FE0F}\u{1F1E8}\u{FE0E}",
            vec![("\u{1F1E6}\u{FE0F}\u{1F1E8}\u{FE0E}", Emoji)],
        ),
        (
            "\u{2764}\u{200D}\u{1F525}",
            vec![("\u{2764}\u{200D}\u{1F525}", Emoji)],
        ),
        (
            "\u{1F525}\u{200D}\u{FE0F}\u{2764}",
            vec![("\u{1F525}\u{200D}\u{FE0F}\u{2764}", Emoji)],
        ),
        (
            "\u{2764}\u{200D}\u{1F525}\u{200D}\u{FE0F}\u{200D}",
            vec![("\u{2764}\u{200D}\u{1F525}\u{200D}\u{FE0F}\u{200D}", Emoji)],
        ),
        (
            "\u{200D}\u{200D}#",
            vec![("\u{200D}\u{200D}", Emoji), ("#", Emoji)],
        ),
        (
            "\u{200D}\u{FE0F}\u{231A}",
            vec![("\u{200D}\u{FE0F}", Emoji), ("\u{231A}", Emoji)],
        ),
        (
            "\u{20E3}\u{200D}#",
            vec![("\u{20E3}", Pass), ("\u{200D}", Emoji), ("#", Emoji)],
        ),
        (
            "\u{200D}A\u{231A}",
            vec![("\u{200D}", Emoji), ("A", Pass), ("\u{231A}", Emoji)],
        ),
    ] {
        assert_scan_items(input, &expected);
    }
}

#[test]
fn scanner_emits_no_empty_items_around_structural_boundaries() {
    for input in [
        "plain",
        "plain\u{FE0F}",
        "\u{FE0F}plain",
        "\u{200D}\u{FE0F}plain#\u{20E3}",
        "plain\u{2764}\u{200D}\u{1F525}tail",
    ] {
        for item in scan(input) {
            assert!(!item.raw.is_empty(), "empty item in scan of {input:?}");
            assert!(item.span.start < item.span.end, "empty span for {item:?}");
        }
    }
}

#[test]
fn emoji_sequence_in_progress_is_empty_only_before_parts_are_added() {
    assert!(EmojiSequenceInProgress::Empty.is_empty());

    let mut links_only = EmojiSequenceInProgress::Empty;
    links_only.push_link(
        ZwjLink {
            presentation_selectors_after_link: vec![],
        },
        unicode::ZWJ.len_utf8(),
    );
    assert!(!links_only.is_empty());

    let emoji_headed = EmojiSequenceInProgress::from_emoji(
        EmojiLike {
            stem: EmojiStem::SingletonBase {
                base: '#',
                presentation_selectors_after_base: vec![],
            },
            modifiers: vec![],
        },
        '#'.len_utf8(),
    );
    assert!(!emoji_headed.is_empty());
}

#[test]
#[allow(clippy::expect_used)]
fn tag_run_consumer_advances_over_tag_runs() {
    let mut scanner = super::scan("\u{E0067}\u{E0062}\u{FE0F}tail");
    let tag_run = scanner
        .consume_emoji_tag_run()
        .expect("tag run should be consumed");

    assert_eq!(tag_run.tag, ['\u{E0067}', '\u{E0062}']);
    assert_eq!(
        tag_run.presentation_selectors_after_tag,
        [Presentation::Emoji]
    );
    assert_eq!(scanner.offset(), "\u{E0067}\u{E0062}\u{FE0F}".len());
    assert_eq!(scanner.peek(), Some('t'));
}

#[test]
fn prepare_next_item_stops_after_emitting_pending_sequence() {
    let mut scanner = super::scan("#tail");

    assert!(scanner.prepare_next_item());
    assert_eq!(scanner.ready.len(), 1);
    assert_eq!(scanner.ready[0].raw, "#");
    assert!(matches!(scanner.ready[0].kind, ScanKind::EmojiSequence(_)));
    assert_eq!(scanner.ready_end, "#".len());
    assert_eq!(scanner.peek(), Some('t'));
}

#[test]
#[allow(clippy::expect_used)]
fn iterator_returns_already_prepared_item_before_scanning_more() {
    let mut scanner = super::scan("#tail");
    assert!(scanner.prepare_next_item());

    let item = scanner.next().expect("prepared item should be returned");

    assert_eq!(item.raw, "#");
    assert!(matches!(item.kind, ScanKind::EmojiSequence(_)));
    assert!(scanner.ready.is_empty());
    assert_eq!(scanner.ready_end, "#".len());
    assert_eq!(scanner.peek(), Some('t'));
}

#[test]
fn unpaired_regional_indicator_scans_as_singleton_stem() {
    let items = scan("\u{1F1E6}\u{FE0F}x");

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].raw, "\u{1F1E6}\u{FE0F}");
    assert_eq!(items[1].raw, "x");
    assert!(matches!(
        &items[0].kind,
        ScanKind::EmojiSequence(EmojiSequence::EmojiHeaded {
            first: EmojiLike {
                stem: EmojiStem::SingletonBase {
                    base: '\u{1F1E6}',
                    presentation_selectors_after_base,
                },
                modifiers,
            },
            joined,
            trailing_links,
        }) if presentation_selectors_after_base == &[Presentation::Emoji]
            && modifiers.is_empty()
            && joined.is_empty()
            && trailing_links.is_empty()
    ));
}

#[test]
fn presentation_from_selector_only_accepts_vs15_and_vs16() {
    assert_eq!(
        Presentation::from_selector(unicode::TEXT_PRESENTATION_SELECTOR),
        Some(Presentation::Text)
    );
    assert_eq!(
        Presentation::from_selector(unicode::EMOJI_PRESENTATION_SELECTOR),
        Some(Presentation::Emoji)
    );
    assert_eq!(Presentation::from_selector('A'), None);
    assert_eq!(Presentation::from_selector(unicode::ZWJ), None);
}

// --- Downstream behavior for permissive scanner grouping ---

#[test]
fn keycap_forms_match_formatter_contract() {
    assert_format("#\u{FE0F}\u{20E3}", &FormatResult::Unchanged);
    assert_format(
        "#\u{20E3}",
        &FormatResult::Changed("#\u{FE0E}\u{20E3}".to_owned()),
    );
    assert_format("#\u{FE0E}\u{20E3}", &FormatResult::Unchanged);
    assert_format(
        "#\u{FE0E}\u{20E3}\u{200D}\u{1F525}",
        &FormatResult::Changed("#\u{FE0F}\u{20E3}\u{200D}\u{1F525}".to_owned()),
    );
}

#[test]
fn zwj_related_selector_cases_match_formatter_contract() {
    assert_format(
        "\u{2764}\u{200D}\u{1F525}",
        &FormatResult::Changed("\u{2764}\u{FE0F}\u{200D}\u{1F525}".to_owned()),
    );
    assert_format(
        "\u{1F525}\u{200D}\u{FE0F}\u{2764}",
        &FormatResult::Changed("\u{1F525}\u{200D}\u{2764}\u{FE0F}".to_owned()),
    );
    assert_format(
        "\u{00A1}\u{FE0E}\u{200D}",
        &FormatResult::Changed("\u{00A1}\u{200D}".to_owned()),
    );
    assert_format(
        "\u{200D}\u{FE0F}\u{231A}",
        &FormatResult::Changed("\u{200D}\u{231A}".to_owned()),
    );
}

#[test]
fn permissive_modifier_and_flag_cases_match_formatter_contract() {
    assert_format("#\u{1F3FB}\u{200D}", &FormatResult::Unchanged);
    assert_format(
        "\u{1F1E6}\u{FE0F}\u{1F1E8}\u{FE0E}",
        &FormatResult::Changed("\u{1F1E6}\u{1F1E8}".to_owned()),
    );
}

// -------------------------------------------------------------------
// AUDIT NOTE: Conformance tests against official Unicode emoji sequence data.
//
// These verify structural assumptions the scanner and formatter depend on:
//   - RGI emoji keycap bases are exactly #*0-9
//   - RGI emoji keycap sequences use the fully-qualified FE0F form
//   - RGI ZWJ sequences always contain ZWJ, never FE0E
//   - Text-default RGI ZWJ components have FE0F
//   - Official keycap and ZWJ sequences scan as one visible structural item
//     and are already canonical to the formatter
//
// If a future Unicode version violates any of these, these tests fail.
// -------------------------------------------------------------------

/// Parse a line from emoji-sequences.txt or emoji-zwj-sequences.txt.
/// Returns (`code_points`, `type_field`) or `None` for comments/blanks.
// Malformed pinned Unicode data is a conformance-test fixture failure.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
fn parse_sequence_line(line: &str) -> Option<(Vec<u32>, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (cp_part, rest) = line
        .split_once(';')
        .unwrap_or_else(|| panic!("sequence line is missing type field: {line}"));
    let type_field = rest
        .split(';')
        .next()
        .expect("split always yields the first field")
        .trim()
        .to_owned();
    assert!(
        !type_field.is_empty(),
        "sequence line has empty type field: {line}"
    );

    // Handle ranges like "231A..231B"
    if let Some((start, end)) = cp_part.split_once("..") {
        assert!(
            !end.contains(".."),
            "sequence range has more than one separator: {line}"
        );
        let start = u32::from_str_radix(start.trim(), 16)
            .unwrap_or_else(|_| panic!("invalid range start in sequence line: {line}"));
        let end = u32::from_str_radix(end.trim(), 16)
            .unwrap_or_else(|_| panic!("invalid range end in sequence line: {line}"));
        return Some((vec![start, end], type_field));
    }

    let cps = cp_part
        .split_whitespace()
        .map(|cp| {
            u32::from_str_radix(cp, 16)
                .unwrap_or_else(|_| panic!("invalid code point in sequence line: {line}"))
        })
        .collect::<Vec<_>>();
    assert!(
        !cps.is_empty(),
        "sequence line has empty code point list: {line}"
    );
    Some((cps, type_field))
}

// Malformed pinned Unicode data is a conformance-test fixture failure.
#[allow(clippy::expect_used)]
fn code_point_string(cps: &[u32]) -> String {
    cps.iter()
        .map(|&cp| char::from_u32(cp).expect("Unicode data should contain valid code points"))
        .collect()
}

#[test]
fn test_conformance_keycap_bases() {
    let data = include_str!("../../data/emoji-sequences.txt");
    let mut keycap_bases: Vec<u32> = Vec::new();

    for line in data.lines() {
        if let Some((cps, type_field)) = parse_sequence_line(line)
            && type_field == "Emoji_Keycap_Sequence"
        {
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
fn test_conformance_rgi_keycaps_are_fully_qualified() {
    let data = include_str!("../../data/emoji-sequences.txt");

    for line in data.lines() {
        if let Some((cps, type_field)) = parse_sequence_line(line)
            && type_field == "Emoji_Keycap_Sequence"
        {
            assert_eq!(
                cps.len(),
                3,
                "RGI keycap sequence has wrong length: {cps:?}"
            );
            assert_eq!(
                cps[1], 0xFE0F,
                "RGI keycap sequence should use FE0F: {cps:?}"
            );
            assert_eq!(
                cps[2], 0x20E3,
                "RGI keycap sequence should end with U+20E3: {cps:?}"
            );
        }
    }
}

#[test]
fn test_conformance_zwj_structure() {
    let data = include_str!("../../data/emoji-zwj-sequences.txt");
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
    let data = include_str!("../../data/emoji-zwj-sequences.txt");
    let mut checked = 0;

    for line in data.lines() {
        if let Some((cps, type_field)) = parse_sequence_line(line) {
            if type_field != "RGI_Emoji_ZWJ_Sequence" {
                continue;
            }

            let mut i = 0;
            while i < cps.len() {
                let base_cp = cps[i];
                if base_cp == 0x200D {
                    i += 1;
                    continue;
                }

                let next = cps.get(i + 1).copied().unwrap_or(0);
                let has_modifier = char::from_u32(next).is_some_and(unicode::is_emoji_modifier);

                if let Some(ch) = char::from_u32(base_cp)
                    && unicode::is_text_default(ch)
                    && unicode::has_variation_sequence(ch)
                    && !has_modifier
                {
                    assert_eq!(
                        cps.get(i + 1).copied(),
                        Some(0xFE0F),
                        "text-default U+{base_cp:04X} in ZWJ without FE0F: {cps:?}"
                    );
                    checked += 1;
                }

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
fn test_conformance_all_sequences_scan_as_one_canonical_item() {
    // Keycap sequences
    let seq_data = include_str!("../../data/emoji-sequences.txt");
    for line in seq_data.lines() {
        if let Some((cps, type_field)) = parse_sequence_line(line) {
            if type_field != "Emoji_Keycap_Sequence" {
                continue;
            }
            let s = code_point_string(&cps);
            let items = scan(&s);
            assert_eq!(
                items.len(),
                1,
                "keycap sequence didn't scan as single item: {cps:?} → {items:?}"
            );
            assert_eq!(items[0].raw, s);
            assert_eq!(items[0].span, 0..s.len());
            assert!(
                matches!(items[0].kind, ScanKind::EmojiSequence(_)),
                "keycap sequence scanned as wrong kind: {:?} -> {:?}",
                cps,
                items[0].kind
            );
            assert_eq!(format_text(&s, &default_policy()), FormatResult::Unchanged);
        }
    }

    // ZWJ sequences
    let zwj_data = include_str!("../../data/emoji-zwj-sequences.txt");
    let mut zwj_count = 0;
    for line in zwj_data.lines() {
        if let Some((cps, type_field)) = parse_sequence_line(line) {
            if type_field != "RGI_Emoji_ZWJ_Sequence" {
                continue;
            }
            let s = code_point_string(&cps);
            let items = scan(&s);
            assert_eq!(
                items.len(),
                1,
                "ZWJ sequence didn't scan as single item: {cps:X?} → {items:#?}"
            );
            assert_eq!(items[0].raw, s);
            assert_eq!(items[0].span, 0..s.len());
            assert!(
                matches!(items[0].kind, ScanKind::EmojiSequence(_)),
                "ZWJ sequence scanned as wrong kind: {:X?} -> {:?}",
                cps,
                items[0].kind
            );
            assert_eq!(format_text(&s, &default_policy()), FormatResult::Unchanged);
            zwj_count += 1;
        }
    }

    assert!(zwj_count > 100, "too few ZWJ sequences found: {zwj_count}");
}
