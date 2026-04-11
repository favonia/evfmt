use proptest::prelude::*;
use proptest::sample::select;

use super::super::*;
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
    let input = "Hello #\u{FE0F}\u{20E3} world \u{00A9}\u{FE0E} \u{2764}\u{FE0F}\u{200D}\u{1F525}";
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
#[allow(clippy::panic)]
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
#[allow(clippy::panic)]
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
#[allow(clippy::panic)]
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
#[allow(clippy::panic)]
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
#[allow(clippy::expect_used, clippy::panic)]
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
#[allow(clippy::panic)]
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
//   - All official sequences scan as the expected kind
//     (test_conformance_all_sequences_classifiable)
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
        // Return range as individual entries — but for our purposes,
        // Basic_Emoji ranges are single code points, not sequences.
        // We'll handle them specially in the test.
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

#[test]
fn test_conformance_keycap_bases() {
    // Verify keycap bases are exactly #*0-9.
    let data = include_str!("../../../data/emoji-sequences.txt");
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
    let data = include_str!("../../../data/emoji-sequences.txt");

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
    let data = include_str!("../../../data/emoji-zwj-sequences.txt");
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
    let data = include_str!("../../../data/emoji-zwj-sequences.txt");
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
    let seq_data = include_str!("../../../data/emoji-sequences.txt");
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
    let zwj_data = include_str!("../../../data/emoji-zwj-sequences.txt");
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
