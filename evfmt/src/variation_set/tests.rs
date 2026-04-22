use super::*;

#[test]
fn test_contains_all() {
    let set = VariationSet::all();
    assert!(set.contains('#'));
    assert!(set.contains_keycap('#'));
    assert!(set.contains('\u{00A9}'));
    assert!(set.contains_keycap('\u{00A9}'));
    assert!(!set.contains('A'));
    assert!(!set.contains_keycap('A'));
}

#[test]
fn test_all_matches_singleton_union_for_full_universe() {
    let mut set = VariationSet::none();
    for index in 0..unicode::VARIATION_ENTRY_COUNT {
        set |= VariationSet::singleton(unicode::variation_entry(index));
        set |= VariationSet::singleton_keycap(unicode::variation_entry(index));
    }

    assert_eq!(set, VariationSet::all());
}

#[test]
fn test_all_matches_named_domains() {
    assert_eq!(VariationSet::all(), NON_KEYCAP_CHARS | KEYCAP_CHARS);
}

#[test]
fn test_contains_none() {
    let set = VariationSet::none();
    assert!(!set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_named_ascii() {
    let set = ASCII;
    assert!(set.contains('#'));
    assert!(!set.contains_keycap('#'));
    assert!(!set.contains('\u{00A9}'));
    assert!(!set.contains('A'));
}

#[test]
fn test_named_keycap_domains() {
    assert!(NON_KEYCAP_CHARS.contains('#'));
    assert!(!NON_KEYCAP_CHARS.contains_keycap('#'));
    assert!(!KEYCAP_CHARS.contains('#'));
    assert!(KEYCAP_CHARS.contains_keycap('#'));
    assert!(KEYCAP_CHARS.contains_keycap('\u{00A9}'));

    assert!(!KEYCAP_EMOJIS.contains('#'));
    assert!(KEYCAP_EMOJIS.contains_keycap('#'));
    assert!(KEYCAP_EMOJIS.contains_keycap('*'));
    assert!(KEYCAP_EMOJIS.contains_keycap('0'));
    assert!(KEYCAP_EMOJIS.contains_keycap('9'));
    assert!(!KEYCAP_EMOJIS.contains_keycap('\u{00A9}'));
}

#[test]
fn test_named_text_defaults() {
    let set = TEXT_DEFAULTS;
    assert!(set.contains('\u{00A9}'));
    assert!(set.contains('#'));
    assert!(!set.contains('\u{2728}'));
    assert!(!set.contains('A'));
}

#[test]
fn test_named_emoji_defaults() {
    let set = EMOJI_DEFAULTS;
    assert!(set.contains('\u{2728}'));
    assert!(!set.contains('\u{00A9}'));
    assert!(!set.contains('#'));
    assert!(!set.contains('A'));
}

#[test]
fn test_named_rights_marks() {
    let set = RIGHTS_MARKS;
    assert!(set.contains('\u{00A9}'));
    assert!(set.contains('\u{00AE}'));
    assert!(set.contains('\u{2122}'));
    assert!(!set.contains('\u{2660}'));
}

#[test]
fn test_named_arrows() {
    let set = ARROWS;
    assert!(set.contains('\u{2194}'));
    assert!(set.contains('\u{27A1}'));
    assert!(set.contains('\u{2B05}'));
    assert!(!set.contains('\u{2660}'));
}

#[test]
fn test_named_card_suits() {
    let set = CARD_SUITS;
    assert!(set.contains('\u{2660}'));
    assert!(set.contains('\u{2663}'));
    assert!(set.contains('\u{2665}'));
    assert!(set.contains('\u{2666}'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_remove_ascii_from_all() {
    let set = VariationSet::all() - ASCII;
    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_remove_multiple_named_sets() {
    let set = VariationSet::all() - ASCII - EMOJI_DEFAULTS;
    assert!(!set.contains('#'));
    assert!(!set.contains('\u{2728}'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_add_singletons() {
    let set = VariationSet::singleton('#') | VariationSet::singleton('*');
    assert!(set.contains('#'));
    assert!(set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_singleton_ignores_non_universe_chars() {
    assert!(VariationSet::singleton('#').contains('#'));
    assert!(!VariationSet::singleton('A').contains('A'));
    assert!(VariationSet::singleton_keycap('#').contains_keycap('#'));
    assert!(!VariationSet::singleton_keycap('A').contains_keycap('A'));
}

#[test]
fn test_add_none_is_identity() {
    let set = VariationSet::none() | ASCII;
    assert!(set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_remove_all_clears_set() {
    let set = ASCII - VariationSet::all();
    assert!(!set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_operator_not_complements_within_universe() {
    let set = !ASCII;

    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
    assert!(!set.contains('A'));
}

#[test]
fn test_operator_union() {
    let set = VariationSet::singleton('#') | VariationSet::singleton('*');

    assert!(set.contains('#'));
    assert!(set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_operator_intersection() {
    let set = ASCII & VariationSet::singleton('#');

    assert!(set.contains('#'));
    assert!(!set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_operator_symmetric_difference() {
    let set = (VariationSet::singleton('#') | VariationSet::singleton('*'))
        ^ (VariationSet::singleton('*') | VariationSet::singleton('\u{00A9}'));

    assert!(set.contains('#'));
    assert!(!set.contains('*'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_operator_difference() {
    let set = VariationSet::all() - ASCII;

    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_operator_assignments() {
    let mut set = VariationSet::singleton('#');
    set |= VariationSet::singleton('*');
    set &= ASCII;
    set ^= VariationSet::singleton('#');
    set -= VariationSet::singleton('\u{00A9}');

    assert!(!set.contains('#'));
    assert!(set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_bitand_assign_intersects_in_place() {
    let mut set = VariationSet::singleton('#') | VariationSet::singleton('\u{00A9}');
    set &= ASCII;

    assert!(set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_sub_assign_removes_in_place() {
    let mut set = VariationSet::singleton('#') | VariationSet::singleton('\u{00A9}');
    set -= VariationSet::singleton('#');

    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_display_examples() {
    assert_eq!(VariationSet::none().to_string(), "none");
    assert_eq!(VariationSet::all().to_string(), "all");
    assert_eq!(VariationSet::singleton('#').to_string(), "u(0023)");
    assert_eq!(VariationSet::singleton_keycap('#').to_string(), "k(0023)");
    assert_eq!(
        (VariationSet::singleton('#') | VariationSet::singleton_keycap('#')).to_string(),
        "u(0023),k(0023)"
    );
}

#[test]
fn test_named_set_matches_reject_nonmembers() {
    assert!(!ASCII.contains('\u{00A9}'));
    assert!(!TEXT_DEFAULTS.contains('\u{2728}'));
    assert!(!EMOJI_DEFAULTS.contains('\u{00A9}'));
    assert!(!RIGHTS_MARKS.contains('#'));
    assert!(!ARROWS.contains('\u{2660}'));
    assert!(!CARD_SUITS.contains('\u{2194}'));
}

#[test]
fn test_default_is_empty() {
    assert_eq!(VariationSet::default(), VariationSet::none());
}

#[test]
fn test_all_bits_matches_public_all_set() {
    let bits = all_bits();

    assert_eq!(bits, VariationSet::all().chars.bits);
    assert_eq!(bits, VariationSet::all().keycap_chars.bits);

    let used_bits = unicode::VARIATION_ENTRY_COUNT % WORD_BITS;
    let expected_last_word = if used_bits == 0 {
        u64::MAX
    } else {
        (1u64 << used_bits) - 1
    };
    assert_eq!(bits[CHARSET_WORDS - 1], expected_last_word);
}

#[test]
fn test_named_bits_matches_public_named_sets() {
    assert_eq!(named_bits(NamedSet::Ascii), ASCII.chars.bits);
    assert_eq!(named_bits(NamedSet::TextDefaults), TEXT_DEFAULTS.chars.bits);
    assert_eq!(
        named_bits(NamedSet::EmojiDefaults),
        EMOJI_DEFAULTS.chars.bits
    );
    assert_eq!(named_bits(NamedSet::RightsMarks), RIGHTS_MARKS.chars.bits);
    assert_eq!(named_bits(NamedSet::Arrows), ARROWS.chars.bits);
    assert_eq!(named_bits(NamedSet::CardSuits), CARD_SUITS.chars.bits);
    assert_eq!(
        named_bits(NamedSet::KeycapEmojis),
        KEYCAP_EMOJIS.keycap_chars.bits
    );
}

#[test]
fn test_named_entry_matches_each_named_set() {
    assert!(named_entry_matches(NamedSet::Ascii, '#'));
    assert!(!named_entry_matches(NamedSet::Ascii, '\u{00A9}'));

    assert!(named_entry_matches(NamedSet::TextDefaults, '\u{00A9}'));
    assert!(!named_entry_matches(NamedSet::TextDefaults, '\u{2728}'));

    assert!(named_entry_matches(NamedSet::EmojiDefaults, '\u{2728}'));
    assert!(!named_entry_matches(NamedSet::EmojiDefaults, '\u{00A9}'));

    assert!(named_entry_matches(NamedSet::RightsMarks, '\u{00A9}'));
    assert!(!named_entry_matches(NamedSet::RightsMarks, '\u{2660}'));

    assert!(named_entry_matches(NamedSet::Arrows, '\u{2194}'));
    assert!(!named_entry_matches(NamedSet::Arrows, '\u{2660}'));

    assert!(named_entry_matches(NamedSet::CardSuits, '\u{2660}'));
    assert!(!named_entry_matches(NamedSet::CardSuits, '\u{00A9}'));

    assert!(named_entry_matches(NamedSet::KeycapEmojis, '#'));
    assert!(named_entry_matches(NamedSet::KeycapEmojis, '0'));
    assert!(!named_entry_matches(NamedSet::KeycapEmojis, '\u{00A9}'));
}

#[test]
fn test_display_multiple_code_points_in_table_order() {
    let set = VariationSet::singleton('*') | VariationSet::singleton('#');

    assert_eq!(set.to_string(), "u(0023),u(002A)");
}
