use super::*;

#[test]
fn test_contains_all() {
    let set = PolicyKeySet::all();
    assert!(set.contains('#'));
    assert!(set.contains_keycap('#'));
    assert!(set.contains('\u{00A9}'));
    assert!(set.contains_keycap('\u{00A9}'));
    assert!(!set.contains('A'));
    assert!(!set.contains_keycap('A'));
}

#[test]
fn test_all_matches_singleton_union_for_full_universe() {
    let mut set = PolicyKeySet::none();
    for index in 0..unicode::VARIATION_ENTRY_COUNT {
        set |= PolicyKeySet::singleton(unicode::variation_entry(index));
        set |= PolicyKeySet::singleton_keycap(unicode::variation_entry(index));
    }

    assert_eq!(set, PolicyKeySet::all());
}

#[test]
fn test_all_matches_named_domains() {
    assert_eq!(
        PolicyKeySet::all(),
        VARIATION_BASES | KEYCAP_VARIATION_BASES
    );
}

#[test]
fn test_contains_none() {
    let set = PolicyKeySet::none();
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
    assert!(VARIATION_BASES.contains('#'));
    assert!(!VARIATION_BASES.contains_keycap('#'));
    assert!(!KEYCAP_VARIATION_BASES.contains('#'));
    assert!(KEYCAP_VARIATION_BASES.contains_keycap('#'));
    assert!(KEYCAP_VARIATION_BASES.contains_keycap('\u{00A9}'));

    assert!(!KEYCAP_TEXT_DEFAULTS.contains('#'));
    assert!(KEYCAP_TEXT_DEFAULTS.contains_keycap('#'));
    assert!(KEYCAP_TEXT_DEFAULTS.contains_keycap('\u{00A9}'));
    assert!(!KEYCAP_TEXT_DEFAULTS.contains_keycap('\u{2728}'));

    assert!(!KEYCAP_EMOJI_DEFAULTS.contains('\u{2728}'));
    assert!(KEYCAP_EMOJI_DEFAULTS.contains_keycap('\u{2728}'));
    assert!(!KEYCAP_EMOJI_DEFAULTS.contains_keycap('\u{00A9}'));

    assert!(!KEYCAP_RGI.contains('#'));
    assert!(KEYCAP_RGI.contains_keycap('#'));
    assert!(KEYCAP_RGI.contains_keycap('*'));
    assert!(KEYCAP_RGI.contains_keycap('0'));
    assert!(KEYCAP_RGI.contains_keycap('9'));
    assert!(!KEYCAP_RGI.contains_keycap('\u{00A9}'));
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
fn test_remove_ascii_from_all() {
    let set = PolicyKeySet::all() - ASCII;
    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_remove_multiple_named_sets() {
    let set = PolicyKeySet::all() - ASCII - EMOJI_DEFAULTS;
    assert!(!set.contains('#'));
    assert!(!set.contains('\u{2728}'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_add_singletons() {
    let set = PolicyKeySet::singleton('#') | PolicyKeySet::singleton('*');
    assert!(set.contains('#'));
    assert!(set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_singleton_ignores_non_universe_chars() {
    assert!(PolicyKeySet::singleton('#').contains('#'));
    assert!(!PolicyKeySet::singleton('A').contains('A'));
    assert!(PolicyKeySet::singleton_keycap('#').contains_keycap('#'));
    assert!(!PolicyKeySet::singleton_keycap('A').contains_keycap('A'));
}

#[test]
fn test_add_none_is_identity() {
    let set = PolicyKeySet::none() | ASCII;
    assert!(set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_remove_all_clears_set() {
    let set = ASCII - PolicyKeySet::all();
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
    let set = PolicyKeySet::singleton('#') | PolicyKeySet::singleton('*');

    assert!(set.contains('#'));
    assert!(set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_operator_intersection() {
    let set = ASCII & PolicyKeySet::singleton('#');

    assert!(set.contains('#'));
    assert!(!set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_operator_symmetric_difference() {
    let set = (PolicyKeySet::singleton('#') | PolicyKeySet::singleton('*'))
        ^ (PolicyKeySet::singleton('*') | PolicyKeySet::singleton('\u{00A9}'));

    assert!(set.contains('#'));
    assert!(!set.contains('*'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_operator_difference() {
    let set = PolicyKeySet::all() - ASCII;

    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_operator_assignments() {
    let mut set = PolicyKeySet::singleton('#');
    set |= PolicyKeySet::singleton('*');
    set &= ASCII;
    set ^= PolicyKeySet::singleton('#');
    set -= PolicyKeySet::singleton('\u{00A9}');

    assert!(!set.contains('#'));
    assert!(set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_bitand_assign_intersects_in_place() {
    let mut set = PolicyKeySet::singleton('#') | PolicyKeySet::singleton('\u{00A9}');
    set &= ASCII;

    assert!(set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_sub_assign_removes_in_place() {
    let mut set = PolicyKeySet::singleton('#') | PolicyKeySet::singleton('\u{00A9}');
    set -= PolicyKeySet::singleton('#');

    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_display_examples() {
    assert_eq!(PolicyKeySet::none().to_string(), "none");
    assert_eq!(PolicyKeySet::all().to_string(), "all");
    assert_eq!(PolicyKeySet::singleton('#').to_string(), "u(0023)");
    assert_eq!(
        PolicyKeySet::singleton_keycap('#').to_string(),
        "keycap:u(0023)"
    );
    assert_eq!(
        (PolicyKeySet::singleton('#') | PolicyKeySet::singleton_keycap('#')).to_string(),
        "u(0023),keycap:u(0023)"
    );
}

#[test]
fn test_named_set_matches_reject_nonmembers() {
    assert!(!ASCII.contains('\u{00A9}'));
    assert!(!TEXT_DEFAULTS.contains('\u{2728}'));
    assert!(!EMOJI_DEFAULTS.contains('\u{00A9}'));
    assert!(!VARIATION_BASES.contains('A'));
    assert!(!KEYCAP_TEXT_DEFAULTS.contains_keycap('\u{2728}'));
    assert!(!KEYCAP_EMOJI_DEFAULTS.contains_keycap('\u{00A9}'));
    assert!(!KEYCAP_RGI.contains_keycap('\u{00A9}'));
}

#[test]
fn test_default_is_empty() {
    assert_eq!(PolicyKeySet::default(), PolicyKeySet::none());
}

#[test]
fn test_all_bits_matches_public_all_set() {
    let bits = all_bits();

    assert_eq!(bits, PolicyKeySet::all().chars.bits);
    assert_eq!(bits, PolicyKeySet::all().keycap_chars.bits);

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
    assert_eq!(
        named_bits(NamedSet::TextDefaults),
        KEYCAP_TEXT_DEFAULTS.keycap_chars.bits
    );
    assert_eq!(
        named_bits(NamedSet::EmojiDefaults),
        KEYCAP_EMOJI_DEFAULTS.keycap_chars.bits
    );
    assert_eq!(
        named_bits(NamedSet::KeycapRgi),
        KEYCAP_RGI.keycap_chars.bits
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

    assert!(named_entry_matches(NamedSet::KeycapRgi, '#'));
    assert!(named_entry_matches(NamedSet::KeycapRgi, '0'));
    assert!(!named_entry_matches(NamedSet::KeycapRgi, '\u{00A9}'));
}

#[test]
fn test_display_multiple_code_points_in_table_order() {
    let set = PolicyKeySet::singleton('*') | PolicyKeySet::singleton('#');

    assert_eq!(set.to_string(), "u(0023),u(002A)");
}
