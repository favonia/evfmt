//! Private canonicalization helpers.
//!
//! Boundary:
//! this module owns text repair only. It takes already-scanned items and emits
//! their canonical serialized form under formatter policy.
//!
//! User-visible diagnostics intentionally do not live here; they are part of
//! [`crate::classify()`].

use crate::scanner::{self, KEYCAP_CAP, ScanItem, ScanKind, ZWJ, ZwjComponent, ZwjSequence};
use crate::slot::{self, PolicyView};

/// Canonicalize a single scanned item according to the formatter policy.
#[must_use]
pub(crate) fn canonicalize_item(item: &ScanItem<'_>, policy: &PolicyView<'_>) -> String {
    let mut output = String::with_capacity(item.raw.len());

    match &item.kind {
        ScanKind::Passthrough => {
            output.push_str(item.raw);
        }
        ScanKind::StandaloneSelectors(_) => {}
        ScanKind::Singleton { base, selectors } => {
            format_singleton(&mut output, *base, selectors, policy);
        }
        ScanKind::Keycap { base, .. } => {
            output.push(*base);
            output.push('\u{FE0F}');
            output.push(KEYCAP_CAP);
        }
        ScanKind::Zwj(sequence) => {
            format_zwj(&mut output, sequence);
        }
    }

    output
}

fn format_singleton(output: &mut String, base: char, selectors: &[char], policy: &PolicyView<'_>) {
    let state =
        slot::resolve_singleton_with_view(base, scanner::effective_selector(selectors), policy);

    output.push(base);
    if let Some(selector) = state.as_selector() {
        output.push(selector);
    }
}

fn format_zwj(output: &mut String, sequence: &ZwjSequence) {
    match sequence {
        ZwjSequence::Terminal(component) => {
            format_zwj_component(output, component);
        }
        ZwjSequence::Joined { head, tail, .. } => {
            format_zwj_component(output, head);
            output.push(ZWJ);
            format_zwj(output, tail);
        }
    }
}

fn format_zwj_component(output: &mut String, component: &ZwjComponent) {
    output.push(component.base);
    // UTS #51 requires an emoji modifier to immediately follow its base.
    // Our canonical ZWJ policy therefore never emits both a modifier and a
    // variation selector on the same component: if `emoji_modifier` is present,
    // `canonical_zwj_component_selector` must return `None`.
    if let Some(emoji_modifier) = component.emoji_modifier {
        output.push(emoji_modifier);
    }
    if let Some(sel) = slot::canonical_zwj_component_selector(component) {
        output.push(sel);
    }
}

#[cfg(test)]
mod tests {
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
}
