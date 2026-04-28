use super::*;

const fn is_keycap_base(ch: char) -> bool {
    ch == '#' || ch == '*' || ch.is_ascii_digit()
}

#[test]
fn test_number_sign_has_variation_sequence() {
    assert!(has_variation_sequence('#'));
    assert!(is_text_default('#'));
    assert!(!is_emoji_default('#'));
}

#[test]
fn test_watch_is_emoji_default() {
    assert!(is_emoji_default('\u{231A}'));
    assert!(!is_text_default('\u{231A}'));
}

#[test]
fn test_copyright_is_text_default() {
    assert!(is_text_default('\u{00A9}'));
    assert!(!is_emoji_default('\u{00A9}'));
}

#[test]
fn test_letter_a_has_no_variation_sequence() {
    assert!(!has_variation_sequence('A'));
    assert!(!is_text_default('A'));
    assert!(!is_emoji_default('A'));
}

#[test]
fn test_sparkles_is_emoji_default() {
    assert!(is_emoji_default('\u{2728}'));
    assert!(!is_text_default('\u{2728}'));
}

#[test]
fn test_emoji_only_character_has_no_default_side() {
    // 😀 has Emoji_Presentation, but it is not listed in
    // emoji-variation-sequences.txt. The default-side predicates classify only
    // valid presentation-sequence bases, not every emoji-presentation character.
    assert!(is_emoji('\u{1F600}'));
    assert!(!has_variation_sequence('\u{1F600}'));
    assert!(!is_text_default('\u{1F600}'));
    assert!(!is_emoji_default('\u{1F600}'));
}

#[test]
fn test_variation_sequence_chars_contains_known_entries() {
    let chars: Vec<char> = variation_sequence_chars().collect();
    assert!(chars.contains(&'#'));
    assert!(chars.contains(&'\u{00A9}'));
    assert!(!chars.contains(&'A'));
}

#[test]
fn in_char_table_finds_first_middle_and_last_entries() {
    assert!(in_char_table(&['a', 'm', 'z'], 'a'));
    assert!(in_char_table(&['a', 'm', 'z'], 'm'));
    assert!(!in_char_table(&['a', 'm', 'z'], '\0'));
}

#[test]
fn test_emoji_modifier_property() {
    assert!(is_emoji_modifier('\u{1F3FB}'));
    assert!(is_emoji_modifier('\u{1F3FF}'));
    assert!(!is_emoji_modifier('\u{1F468}'));
    assert!(!is_emoji_modifier('A'));
}

/// Drift watch for modification handling: emoji modifiers and the enclosing
/// keycap are sequence suffixes, not bases with their own VS15/VS16 policies.
#[test]
fn test_modification_suffixes_have_no_variation_sequences() {
    for &modifier in &EMOJI_MODIFIERS {
        assert!(
            !has_variation_sequence(modifier),
            "emoji modifier U+{:04X} unexpectedly has a variation sequence",
            modifier as u32,
        );
    }

    assert!(
        !has_variation_sequence(COMBINING_ENCLOSING_KEYCAP),
        "combining enclosing keycap unexpectedly has a variation sequence",
    );
}

#[test]
fn test_emoji_property() {
    // Keycap bases
    assert!(is_emoji('#'));
    assert!(is_emoji('*'));
    assert!(is_emoji('0'));
    assert!(is_emoji('9'));
    // Common emoji
    assert!(is_emoji('\u{231A}')); // watch
    assert!(is_emoji('\u{2728}')); // sparkles
    assert!(is_emoji('\u{1F600}')); // grinning face
    // Non-emoji
    assert!(!is_emoji('A'));
    assert!(!is_emoji('\u{0041}'));
}

#[test]
fn test_ri_property() {
    assert!(is_ri('\u{1F1E6}')); // Regional Indicator A
    assert!(is_ri('\u{1F1FF}')); // Regional Indicator Z
    assert!(!is_ri('\u{1F1E5}')); // just before range
    assert!(!is_ri('\u{1F200}')); // just after range
    assert!(!is_ri('A'));
}

#[test]
fn test_keycap_base() {
    assert!(is_keycap_base('#'));
    assert!(is_keycap_base('*'));
    assert!(is_keycap_base('0'));
    assert!(is_keycap_base('9'));
    assert!(!is_keycap_base('A'));
}

/// Drift watch for the singleton analysis rule cascade: keycap handling
/// assumes every keycap base has a sanctioned VS15/VS16 variation sequence
/// and is text-default (so rule 5 does not fire on it). If a future Unicode
/// upgrade changes either property the cascade needs updating.
#[test]
fn test_keycap_bases_have_text_default_variation_sequences() {
    for ch in ['#', '*', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'] {
        assert!(is_keycap_base(ch), "expected {ch:?} to be a keycap base");
        assert!(
            has_variation_sequence(ch),
            "keycap base {ch:?} missing standardized variation sequence",
        );
        assert!(
            is_text_default(ch),
            "keycap base {ch:?} expected to be text-default",
        );
        assert!(
            !is_emoji_default(ch),
            "keycap base {ch:?} expected to not be emoji-default",
        );
    }
}

#[test]
fn test_tag() {
    assert!(is_tag('\u{E0020}')); // tag space
    assert!(is_tag('\u{E007F}')); // cancel tag
    assert!(is_tag('\u{E0061}')); // tag lowercase a
    assert!(!is_tag('\u{E001F}')); // just before range
    assert!(!is_tag('\u{E0080}')); // just after range
}

// -------------------------------------------------------------------
// Phase 2: Generated-data conformance tests
//
// Independently verify the generated runtime tables against the pinned
// Unicode 17.0 source data. These tests do NOT use the build.rs code
// path — they parse the source files with independent logic and compare
// the results against the generated tables.
// -------------------------------------------------------------------

use std::collections::BTreeSet;

/// Independently parse `emoji-variation-sequences.txt` and return
/// (`text_vs_set`, `emoji_vs_set`) of code points.
// Malformed pinned Unicode data is a test fixture failure.
#[allow(clippy::expect_used)]
fn parse_variation_sequences_independently() -> (BTreeSet<u32>, BTreeSet<u32>) {
    let data = std::fs::read_to_string("data/emoji-variation-sequences.txt")
        .expect("failed to read emoji-variation-sequences.txt");

    let mut text_vs: BTreeSet<u32> = BTreeSet::new();
    let mut emoji_vs: BTreeSet<u32> = BTreeSet::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Format: "0023 FE0E  ; text style;  # ..."
        let Some((before, _after)) = line.split_once(';') else {
            continue;
        };
        let mut parts = before.split_whitespace();
        let Some(cp_hex) = parts.next() else { continue };
        let Some(selector) = parts.next() else {
            continue;
        };
        if parts.next().is_some() {
            continue;
        }
        let cp = u32::from_str_radix(cp_hex, 16).expect("bad codepoint");
        match selector {
            "FE0E" => {
                text_vs.insert(cp);
            }
            "FE0F" => {
                emoji_vs.insert(cp);
            }
            _ => {}
        }
    }

    (text_vs, emoji_vs)
}

/// Independently parse a UCD-format property file and return code points
/// matching `wanted_property`.
// Malformed pinned Unicode data is a test fixture failure.
#[allow(clippy::expect_used)] // Test fixtures are required to be readable pinned Unicode data.
fn parse_ucd_property_independently(path: &str, wanted_property: &str) -> BTreeSet<u32> {
    let data = std::fs::read_to_string(path).expect("failed to read pinned Unicode data file");

    let mut matching: BTreeSet<u32> = BTreeSet::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let entry: ucd_parse::EmojiProperty = match line.parse() {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.property != wanted_property {
            continue;
        }
        for cp in entry.codepoints {
            matching.insert(cp.value());
        }
    }

    matching
}

#[test]
fn test_conformance_variation_sequences() {
    let (text_vs, emoji_vs) = parse_variation_sequences_independently();

    // The variation-sequence set should be the union of text_vs and emoji_vs.
    let variation_sequence_set: BTreeSet<u32> = text_vs.union(&emoji_vs).copied().collect();

    // The build script asserts text_vs == emoji_vs; verify independently.
    assert_eq!(
        text_vs, emoji_vs,
        "expected every variation-sequence character to have both text and emoji entries"
    );

    // Verify the generated table has exactly the right set of code points.
    let generated_cps: BTreeSet<u32> = variation_sequence_chars().map(|ch| ch as u32).collect();

    assert_eq!(
        variation_sequence_set, generated_cps,
        "variation-sequence set mismatch between source data and generated table"
    );
}

#[test]
fn test_conformance_emoji_presentation() {
    let emoji_pres = parse_ucd_property_independently("data/emoji-data.txt", "Emoji_Presentation");

    // Every variation-sequence char's default presentation should match.
    for ch in variation_sequence_chars() {
        let cp = ch as u32;
        let expected_emoji = emoji_pres.contains(&cp);
        assert_eq!(
            is_emoji_default(ch),
            expected_emoji,
            "default presentation mismatch for U+{cp:04X}: \
             is_emoji_default={}, emoji-data.txt={}",
            is_emoji_default(ch),
            expected_emoji
        );
    }
}

#[test]
fn test_conformance_emoji_modifier() {
    let emoji_modifiers = parse_ucd_property_independently("data/emoji-data.txt", "Emoji_Modifier");

    let generated_modifiers: BTreeSet<u32> = EMOJI_MODIFIERS.iter().map(|&ch| ch as u32).collect();

    assert_eq!(
        generated_modifiers, emoji_modifiers,
        "Emoji_Modifier set mismatch between source data and generated table"
    );
}

#[test]
fn test_conformance_emoji() {
    let emoji_chars = parse_ucd_property_independently("data/emoji-data.txt", "Emoji");

    // Check every code point in the source data is recognized.
    for &cp in &emoji_chars {
        if let Some(ch) = char::from_u32(cp) {
            assert!(
                is_emoji(ch),
                "U+{cp:04X} has Emoji=Yes in source data but is_emoji returned false"
            );
        }
    }

    // Spot-check some non-emoji code points.
    assert!(!is_emoji('A'));
    assert!(!is_emoji('\u{0000}'));
}

#[test]
fn test_conformance_ri() {
    let ri_chars = parse_ucd_property_independently("data/PropList.txt", "Regional_Indicator");

    for &cp in &ri_chars {
        if let Some(ch) = char::from_u32(cp) {
            assert!(
                is_ri(ch),
                "U+{cp:04X} has Regional_Indicator in source data but is_ri returned false"
            );
        }
    }

    // Check boundaries.
    assert!(!is_ri('\u{1F1E5}'));
    assert!(!is_ri('\u{1F200}'));
}
