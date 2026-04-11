use super::*;
use crate::scanner::{VS_EMOJI, ZwjComponent, ZwjLink, ZwjSequence};

#[test]
fn test_format_zwj_never_prefixes_first_component_with_zwj() {
    let mut output = String::new();
    let sequence = ZwjSequence::Joined {
        head: ZwjComponent {
            base: '\u{2764}',
            selectors_after_base: vec![],
            emoji_modifier: None,
            selectors_after_modifier: vec![],
        },
        link: ZwjLink { selectors: vec![] },
        tail: Box::new(ZwjSequence::Terminal(ZwjComponent {
            base: '\u{1F525}',
            selectors_after_base: vec![],
            emoji_modifier: None,
            selectors_after_modifier: vec![],
        })),
    };

    format_zwj(&mut output, &sequence);

    assert_eq!(output, "\u{2764}\u{FE0F}\u{200D}\u{1F525}");
}

#[test]
fn test_format_zwj_drops_selectors_on_join_edges() {
    let mut output = String::new();
    let sequence = ZwjSequence::Joined {
        head: ZwjComponent {
            base: '\u{1F525}',
            selectors_after_base: vec![],
            emoji_modifier: None,
            selectors_after_modifier: vec![],
        },
        link: ZwjLink {
            selectors: vec![VS_EMOJI],
        },
        tail: Box::new(ZwjSequence::Terminal(ZwjComponent {
            base: '\u{2764}',
            selectors_after_base: vec![],
            emoji_modifier: None,
            selectors_after_modifier: vec![],
        })),
    };

    format_zwj(&mut output, &sequence);

    assert_eq!(output, "\u{1F525}\u{200D}\u{2764}\u{FE0F}");
}

#[test]
fn test_format_zwj_keeps_modifier_adjacent_to_its_base() {
    let mut output = String::new();
    let sequence = ZwjSequence::Joined {
        head: ZwjComponent {
            base: '\u{1F468}',
            selectors_after_base: vec![],
            emoji_modifier: Some('\u{1F3FB}'),
            selectors_after_modifier: vec![VS_EMOJI],
        },
        link: ZwjLink { selectors: vec![] },
        tail: Box::new(ZwjSequence::Terminal(ZwjComponent {
            base: '\u{1F466}',
            selectors_after_base: vec![],
            emoji_modifier: None,
            selectors_after_modifier: vec![],
        })),
    };

    format_zwj(&mut output, &sequence);

    assert_eq!(output, "\u{1F468}\u{1F3FB}\u{200D}\u{1F466}");
}
