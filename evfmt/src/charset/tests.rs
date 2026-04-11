use super::*;

#[test]
fn test_contains_all() {
    let set = CharSet::all();
    assert!(set.contains('#'));
    assert!(set.contains('\u{00A9}'));
    assert!(!set.contains('A'));
}

#[test]
fn test_contains_none() {
    let set = CharSet::none();
    assert!(!set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_named_ascii() {
    let set = CharSet::named(NamedSetId::Ascii);
    assert!(set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
    assert!(!set.contains('A'));
}

#[test]
fn test_named_emoji_defaults() {
    let set = CharSet::named(NamedSetId::EmojiDefaults);
    assert!(set.contains('\u{2728}'));
    assert!(!set.contains('\u{00A9}'));
    assert!(!set.contains('#'));
    assert!(!set.contains('A'));
}

#[test]
fn test_named_rights_marks() {
    let set = CharSet::named(NamedSetId::RightsMarks);
    assert!(set.contains('\u{00A9}'));
    assert!(set.contains('\u{00AE}'));
    assert!(set.contains('\u{2122}'));
    assert!(!set.contains('\u{2660}'));
}

#[test]
fn test_named_arrows() {
    let set = CharSet::named(NamedSetId::Arrows);
    assert!(set.contains('\u{2194}'));
    assert!(set.contains('\u{27A1}'));
    assert!(set.contains('\u{2B05}'));
    assert!(!set.contains('\u{2660}'));
}

#[test]
fn test_named_card_suits() {
    let set = CharSet::named(NamedSetId::CardSuits);
    assert!(set.contains('\u{2660}'));
    assert!(set.contains('\u{2663}'));
    assert!(set.contains('\u{2665}'));
    assert!(set.contains('\u{2666}'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_remove_ascii_from_all() {
    let set = CharSet::all() - CharSet::named(NamedSetId::Ascii);
    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_remove_multiple_named_sets() {
    let set = CharSet::all()
        - CharSet::named(NamedSetId::Ascii)
        - CharSet::named(NamedSetId::EmojiDefaults);
    assert!(!set.contains('#'));
    assert!(!set.contains('\u{2728}'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_add_singletons() {
    let set = CharSet::singleton('#') | CharSet::singleton('*');
    assert!(set.contains('#'));
    assert!(set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_singleton_ignores_non_universe_chars() {
    assert!(CharSet::singleton('#').contains('#'));
    assert!(!CharSet::singleton('A').contains('A'));
}

#[test]
fn test_add_none_is_identity() {
    let set = CharSet::none() | CharSet::named(NamedSetId::Ascii);
    assert!(set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_remove_all_clears_set() {
    let set = CharSet::named(NamedSetId::Ascii) - CharSet::all();
    assert!(!set.contains('#'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_operator_not_complements_within_universe() {
    let set = !CharSet::named(NamedSetId::Ascii);

    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
    assert!(!set.contains('A'));
}

#[test]
fn test_operator_union() {
    let set = CharSet::singleton('#') | CharSet::singleton('*');

    assert!(set.contains('#'));
    assert!(set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_operator_intersection() {
    let set = CharSet::named(NamedSetId::Ascii) & CharSet::singleton('#');

    assert!(set.contains('#'));
    assert!(!set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_operator_symmetric_difference() {
    let set = (CharSet::singleton('#') | CharSet::singleton('*'))
        ^ (CharSet::singleton('*') | CharSet::singleton('\u{00A9}'));

    assert!(set.contains('#'));
    assert!(!set.contains('*'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_operator_difference() {
    let set = CharSet::all() - CharSet::named(NamedSetId::Ascii);

    assert!(!set.contains('#'));
    assert!(set.contains('\u{00A9}'));
}

#[test]
fn test_operator_assignments() {
    let mut set = CharSet::singleton('#');
    set |= CharSet::singleton('*');
    set &= CharSet::named(NamedSetId::Ascii);
    set ^= CharSet::singleton('#');
    set -= CharSet::singleton('\u{00A9}');

    assert!(!set.contains('#'));
    assert!(set.contains('*'));
    assert!(!set.contains('\u{00A9}'));
}

#[test]
fn test_display_examples() {
    assert_eq!(CharSet::none().to_string(), "none");
    assert_eq!(CharSet::all().to_string(), "all");
    assert_eq!(CharSet::singleton('#').to_string(), "u(0023)");
}

#[test]
fn test_named_set_display_names() {
    assert_eq!(NamedSetId::Ascii.to_string(), "ascii");
    assert_eq!(NamedSetId::EmojiDefaults.to_string(), "emoji-defaults");
    assert_eq!(NamedSetId::RightsMarks.to_string(), "rights-marks");
    assert_eq!(NamedSetId::Arrows.to_string(), "arrows");
    assert_eq!(NamedSetId::CardSuits.to_string(), "card-suits");
}

#[test]
fn test_named_set_matches_reject_nonmembers() {
    assert!(!NamedSetId::Ascii.matches('\u{00A9}'));
    assert!(!NamedSetId::EmojiDefaults.matches('\u{00A9}'));
    assert!(!NamedSetId::RightsMarks.matches('#'));
    assert!(!NamedSetId::Arrows.matches('\u{2660}'));
    assert!(!NamedSetId::CardSuits.matches('\u{2194}'));
}

#[test]
fn test_default_is_empty() {
    assert_eq!(CharSet::default(), CharSet::none());
}

#[test]
fn test_display_multiple_code_points_in_table_order() {
    let set = CharSet::singleton('*') | CharSet::singleton('#');

    assert_eq!(set.to_string(), "u(0023),u(002A)");
}
