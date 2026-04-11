use super::*;
use crate::analyze_text;
use crate::charset::{CharSet, NamedSetId};

#[test]
fn reasonable_set_contains_distinguishes_membership() {
    let set = ReasonableSet::new(true, false, true);

    assert!(set.contains(SelectorState::Bare));
    assert!(!set.contains(SelectorState::Text));
    assert!(set.contains(SelectorState::Emoji));
}

#[test]
fn reasonable_set_only_reports_each_singleton() {
    assert_eq!(ReasonableSet::empty().only(), None);
    assert_eq!(
        ReasonableSet::new(true, false, false).only(),
        Some(SelectorState::Bare)
    );
    assert_eq!(
        ReasonableSet::new(false, true, false).only(),
        Some(SelectorState::Text)
    );
    assert_eq!(
        ReasonableSet::new(false, false, true).only(),
        Some(SelectorState::Emoji)
    );
    assert_eq!(ReasonableSet::new(true, true, false).only(), None);
}

#[test]
fn singleton_slot_keeps_three_reasonable_states() {
    let slots = analyze_text("#");
    assert_eq!(slots.len(), 1);
    let slot = &slots[0];
    assert_eq!(slot.slot_kind, SlotKind::StandaloneEvs);
    assert_eq!(slot.reasonable_states.count(), 3);
    assert!(slot.reasonable_states.contains(SelectorState::Bare));
    assert!(slot.reasonable_states.contains(SelectorState::Text));
    assert!(slot.reasonable_states.contains(SelectorState::Emoji));
}

#[test]
fn singleton_slot_uses_unicode_reasonableness() {
    let base = '\u{00A9}';
    let text = base.to_string();
    let slots = analyze_text(&text);
    assert_eq!(slots.len(), 1);
    let slot = &slots[0];
    assert_eq!(slot.slot_kind, SlotKind::StandaloneEvs);
    assert!(slot.reasonable_states.contains(SelectorState::Bare));
    #[allow(clippy::unwrap_used)]
    let info = unicode::variation_sequence_info(base).unwrap();
    assert!(slot.reasonable_states.contains(SelectorState::Bare));
    assert_eq!(
        slot.reasonable_states.contains(SelectorState::Text),
        info.has_text_vs
    );
    assert_eq!(
        slot.reasonable_states.contains(SelectorState::Emoji),
        info.has_emoji_vs
    );
    assert_eq!(
        slot.reasonable_states.count(),
        1 + usize::from(info.has_text_vs) + usize::from(info.has_emoji_vs)
    );
}

#[test]
fn keycap_slot_only_allows_emoji_selector() {
    let slots = analyze_text("#\u{20E3}");
    assert_eq!(slots.len(), 1);
    let slot = &slots[0];
    assert_eq!(slot.slot_kind, SlotKind::Keycap);
    assert_eq!(slot.reasonable_states.count(), 1);
    assert!(slot.reasonable_states.contains(SelectorState::Emoji));
    assert!(!slot.has_extra_selectors);
}

#[test]
fn keycap_slot_reports_extra_selectors() {
    let slots = analyze_text("#\u{FE0F}\u{FE0E}\u{20E3}");
    assert_eq!(slots.len(), 1);
    let slot = &slots[0];
    assert_eq!(slot.slot_kind, SlotKind::Keycap);
    assert!(slot.has_extra_selectors);
}

#[test]
fn modifier_defect_slot_only_allows_no_selector() {
    let item = ScanItem {
        raw: "\u{1F44D}\u{FE0F}\u{1F3FB}\u{200D}\u{1F525}",
        span: 0..18,
        kind: ScanKind::Zwj(scanner::ZwjSequence::Joined {
            head: scanner::ZwjComponent {
                base: '\u{1F44D}',
                selectors_after_base: vec![],
                emoji_modifier: Some('\u{1F3FB}'),
                selectors_after_modifier: vec![VS_EMOJI],
            },
            link: scanner::ZwjLink { selectors: vec![] },
            tail: Box::new(scanner::ZwjSequence::Terminal(scanner::ZwjComponent {
                base: '\u{1F525}',
                selectors_after_base: vec![],
                emoji_modifier: None,
                selectors_after_modifier: vec![],
            })),
        }),
    };
    let analysis = analyze_scan_item(&item);
    assert_eq!(analysis.slot_kind, SlotKind::ModifierDefect);
    assert_eq!(analysis.reasonable_states.count(), 1);
    assert!(analysis.reasonable_states.contains(SelectorState::Bare));
}

#[test]
fn zwj_with_modifier_but_no_selector_is_not_modifier_defect() {
    let slots = analyze_text("\u{1F468}\u{1F3FB}\u{200D}\u{1F466}");
    assert_eq!(slots.len(), 1);
    let slot = &slots[0];
    assert_eq!(slot.slot_kind, SlotKind::ZwjTerminal);
    assert_eq!(slot.current_state, SelectorState::Bare);
    assert!(!slot.has_extra_selectors);
}

#[test]
fn zwj_current_state_can_come_from_tail_component() {
    let item = ScanItem {
        raw: "\u{2764}\u{200D}\u{1F525}\u{FE0F}",
        span: 0..10,
        kind: ScanKind::Zwj(scanner::ZwjSequence::Joined {
            head: scanner::ZwjComponent {
                base: '\u{2764}',
                selectors_after_base: vec![],
                emoji_modifier: None,
                selectors_after_modifier: vec![],
            },
            link: scanner::ZwjLink { selectors: vec![] },
            tail: Box::new(scanner::ZwjSequence::Terminal(scanner::ZwjComponent {
                base: '\u{1F525}',
                selectors_after_base: vec![VS_EMOJI],
                emoji_modifier: None,
                selectors_after_modifier: vec![],
            })),
        }),
    };

    let analysis = analyze_scan_item(&item);
    assert_eq!(analysis.slot_kind, SlotKind::ZwjTerminal);
    assert_eq!(analysis.current_state, SelectorState::Emoji);
    assert!(!analysis.has_extra_selectors);
}

#[test]
fn zwj_terminal_extra_selectors_detect_terminal_run_only() {
    let item = ScanItem {
        raw: "\u{1F525}\u{FE0F}\u{FE0E}",
        span: 0..9,
        kind: ScanKind::Zwj(scanner::ZwjSequence::Terminal(scanner::ZwjComponent {
            base: '\u{1F525}',
            selectors_after_base: vec![VS_EMOJI, VS_TEXT],
            emoji_modifier: None,
            selectors_after_modifier: vec![],
        })),
    };

    let analysis = analyze_scan_item(&item);
    assert_eq!(analysis.slot_kind, SlotKind::ZwjTerminal);
    assert_eq!(analysis.current_state, SelectorState::Emoji);
    assert!(analysis.has_extra_selectors);
}

#[test]
fn zwj_extra_selectors_detect_head_terminal_run_only() {
    let item = ScanItem {
        raw: "\u{2764}\u{FE0F}\u{FE0E}\u{200D}\u{1F525}",
        span: 0..13,
        kind: ScanKind::Zwj(scanner::ZwjSequence::Joined {
            head: scanner::ZwjComponent {
                base: '\u{2764}',
                selectors_after_base: vec![VS_EMOJI, VS_TEXT],
                emoji_modifier: None,
                selectors_after_modifier: vec![],
            },
            link: scanner::ZwjLink { selectors: vec![] },
            tail: Box::new(scanner::ZwjSequence::Terminal(scanner::ZwjComponent {
                base: '\u{1F525}',
                selectors_after_base: vec![],
                emoji_modifier: None,
                selectors_after_modifier: vec![],
            })),
        }),
    };

    let analysis = analyze_scan_item(&item);
    assert_eq!(analysis.slot_kind, SlotKind::ZwjTerminal);
    assert_eq!(analysis.current_state, SelectorState::Emoji);
    assert!(analysis.has_extra_selectors);
}

#[test]
fn zwj_extra_selectors_detect_joiner_selector_only() {
    let item = ScanItem {
        raw: "\u{2764}\u{200D}\u{FE0F}\u{1F525}",
        span: 0..10,
        kind: ScanKind::Zwj(scanner::ZwjSequence::Joined {
            head: scanner::ZwjComponent {
                base: '\u{2764}',
                selectors_after_base: vec![],
                emoji_modifier: None,
                selectors_after_modifier: vec![],
            },
            link: scanner::ZwjLink {
                selectors: vec![VS_EMOJI],
            },
            tail: Box::new(scanner::ZwjSequence::Terminal(scanner::ZwjComponent {
                base: '\u{1F525}',
                selectors_after_base: vec![],
                emoji_modifier: None,
                selectors_after_modifier: vec![],
            })),
        }),
    };

    let analysis = analyze_scan_item(&item);
    assert_eq!(analysis.slot_kind, SlotKind::ZwjTerminal);
    assert_eq!(analysis.current_state, SelectorState::Bare);
    assert!(analysis.has_extra_selectors);
}

#[test]
fn zwj_extra_selectors_detect_tail_recursion_only() {
    let item = ScanItem {
        raw: "\u{2764}\u{200D}\u{FE0F}\u{1F525}\u{FE0F}\u{FE0E}",
        span: 0..16,
        kind: ScanKind::Zwj(scanner::ZwjSequence::Joined {
            head: scanner::ZwjComponent {
                base: '\u{2764}',
                selectors_after_base: vec![],
                emoji_modifier: None,
                selectors_after_modifier: vec![],
            },
            link: scanner::ZwjLink { selectors: vec![] },
            tail: Box::new(scanner::ZwjSequence::Terminal(scanner::ZwjComponent {
                base: '\u{1F525}',
                selectors_after_base: vec![VS_EMOJI, VS_TEXT],
                emoji_modifier: None,
                selectors_after_modifier: vec![],
            })),
        }),
    };

    let analysis = analyze_scan_item(&item);
    assert_eq!(analysis.slot_kind, SlotKind::ZwjTerminal);
    assert_eq!(analysis.current_state, SelectorState::Emoji);
    assert!(analysis.has_extra_selectors);
}

#[test]
fn canonical_state_prefers_bare_when_policy_says_so() {
    let item = scanner::scan("#\u{FE0E}");
    let analysis = analyze_scan_item(&item[0]);
    let prefer_bare = CharSet::named(NamedSetId::Ascii);
    let bare_as_text = CharSet::named(NamedSetId::Ascii);
    let policy = PolicyView {
        prefer_bare: &prefer_bare,
        bare_as_text: &bare_as_text,
    };
    assert_eq!(
        canonical_state_with_view(&analysis, &policy),
        Some(SelectorState::Bare)
    );
}

#[test]
fn canonical_state_resolves_non_prefer_bare_slot_explicitly() {
    let item = scanner::scan("\u{00A9}");
    let analysis = analyze_scan_item(&item[0]);
    let prefer_bare = CharSet::named(NamedSetId::Ascii);
    let bare_as_text = CharSet::named(NamedSetId::Ascii);
    let policy = PolicyView {
        prefer_bare: &prefer_bare,
        bare_as_text: &bare_as_text,
    };
    // ©️ is non-ASCII -> not in bare_as_text -> bare resolves to emoji.
    assert_eq!(
        canonical_state_with_view(&analysis, &policy),
        Some(SelectorState::Emoji)
    );
}

#[test]
fn canonical_state_public_wrapper_resolves_singleton() {
    let analysis = analyze_text("\u{00A9}")[0].clone();
    let policy = Policy::default();

    assert_eq!(
        canonical_state(&analysis, &policy),
        Some(SelectorState::Emoji)
    );
}

#[test]
fn canonical_state_public_wrapper_resolves_keycap() {
    let analysis = analyze_text("#\u{20E3}")[0].clone();
    let policy = Policy::default();

    assert_eq!(
        canonical_state(&analysis, &policy),
        Some(SelectorState::Emoji)
    );
}

#[test]
fn canonical_state_returns_none_for_non_slots() {
    let analysis = analyze_text("Hello")[0].clone();
    let policy = Policy::default();

    assert_eq!(canonical_state(&analysis, &policy), None);
}
