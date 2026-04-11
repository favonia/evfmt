use super::*;

#[test]
fn test_number_sign_has_variation_sequence() {
    assert!(has_variation_sequence('#'));
    #[allow(clippy::unwrap_used)]
    let info = variation_sequence_info('#').unwrap();
    assert!(info.has_text_vs);
    assert!(info.has_emoji_vs);
    assert_eq!(info.default_side, DefaultSide::Text);
}

#[test]
fn test_watch_is_emoji_default() {
    #[allow(clippy::unwrap_used)]
    let info = variation_sequence_info('\u{231A}').unwrap();
    assert_eq!(info.default_side, DefaultSide::Emoji);
}

#[test]
fn test_copyright_is_text_default() {
    #[allow(clippy::unwrap_used)]
    let info = variation_sequence_info('\u{00A9}').unwrap();
    assert_eq!(info.default_side, DefaultSide::Text);
}

#[test]
fn test_letter_a_has_no_variation_sequence() {
    assert!(!has_variation_sequence('A'));
    assert!(variation_sequence_info('A').is_none());
}

#[test]
fn test_sparkles_is_emoji_default() {
    #[allow(clippy::unwrap_used)]
    let info = variation_sequence_info('\u{2728}').unwrap();
    assert_eq!(info.default_side, DefaultSide::Emoji);
}

#[test]
fn test_variation_sequence_chars_contains_known_entries() {
    let chars: Vec<char> = variation_sequence_chars().collect();
    assert!(chars.contains(&'#'));
    assert!(chars.contains(&'\u{00A9}'));
    assert!(!chars.contains(&'A'));
}

#[test]
fn test_emoji_modifier_property() {
    assert!(is_emoji_modifier('\u{1F3FB}'));
    assert!(is_emoji_modifier('\u{1F3FF}'));
    assert!(!is_emoji_modifier('\u{1F468}'));
    assert!(!is_emoji_modifier('A'));
}

// -------------------------------------------------------------------
// Phase 2: Generated-data conformance test
//
// Independently verify the generated runtime table against the pinned
// Unicode 16.0 source data. This test does NOT use the build.rs code
// path — it parses the source files with independent logic and compares
// the results against VARIATION_ENTRIES.
//
// Two independent parsers:
//   1. ucd-parse for emoji-data.txt → Emoji_Presentation property
//   2. Hand-written parser for emoji-variation-sequences.txt
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

/// Independently parse `emoji-data.txt` using `ucd-parse` and return
/// the set of code points with the `Emoji_Presentation` property.
fn parse_emoji_presentation_independently() -> BTreeSet<u32> {
    parse_emoji_property_independently("Emoji_Presentation")
}

fn parse_emoji_modifier_independently() -> BTreeSet<u32> {
    parse_emoji_property_independently("Emoji_Modifier")
}

// Malformed pinned Unicode data is a test fixture failure.
#[allow(clippy::expect_used)]
fn parse_emoji_property_independently(wanted_property: &str) -> BTreeSet<u32> {
    let data =
        std::fs::read_to_string("data/emoji-data.txt").expect("failed to read emoji-data.txt");

    let mut matching: BTreeSet<u32> = BTreeSet::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Use ucd-parse to parse the line.
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
    // Independently parse the source data.
    let (text_vs, emoji_vs) = parse_variation_sequences_independently();

    // The variation-sequence set should be the union of text_vs and emoji_vs.
    let variation_sequence_set: BTreeSet<u32> = text_vs.union(&emoji_vs).copied().collect();

    // Verify VARIATION_ENTRIES has exactly the right set of code points.
    let generated_cps: BTreeSet<u32> = VARIATION_ENTRIES
        .iter()
        .map(|e| e.code_point as u32)
        .collect();

    assert_eq!(
        variation_sequence_set, generated_cps,
        "variation-sequence set mismatch between source data and generated table"
    );

    // Verify each entry's has_text_vs and has_emoji_vs flags.
    for entry in VARIATION_ENTRIES {
        let cp = entry.code_point as u32;
        assert_eq!(
            entry.has_text_vs,
            text_vs.contains(&cp),
            "has_text_vs mismatch for U+{cp:04X}"
        );
        assert_eq!(
            entry.has_emoji_vs,
            emoji_vs.contains(&cp),
            "has_emoji_vs mismatch for U+{cp:04X}"
        );
    }
}

#[test]
fn test_conformance_emoji_presentation() {
    // Independently parse Emoji_Presentation from emoji-data.txt.
    let emoji_pres = parse_emoji_presentation_independently();

    // Verify each VARIATION_ENTRIES entry's default_emoji flag.
    for entry in VARIATION_ENTRIES {
        let cp = entry.code_point as u32;
        let expected_emoji = emoji_pres.contains(&cp);
        assert_eq!(
            entry.default_emoji, expected_emoji,
            "default_emoji mismatch for U+{cp:04X}: \
             generated={}, emoji-data.txt={}",
            entry.default_emoji, expected_emoji
        );
    }
}

#[test]
fn test_conformance_emoji_modifier() {
    let emoji_modifiers = parse_emoji_modifier_independently();

    let generated_modifiers: BTreeSet<u32> = EMOJI_MODIFIERS.iter().map(|&ch| ch as u32).collect();

    assert_eq!(
        generated_modifiers, emoji_modifiers,
        "Emoji_Modifier set mismatch between source data and generated table"
    );
}
