//! Slot-level analysis for emoji variation handling.
//!
//! Boundary:
//! this module owns the intermediate slot model between sequence-aware
//! scanning and higher-level policy decisions.
//!
//! It reduces scanned structure into slot kinds, records which selector
//! states are reasonable in that slot, and resolves selector state for a slot
//! once policy is applied.
//!
//! It does not classify user-visible violations and it does not rewrite text.
//! Those responsibilities live in [`crate::classify()`] and
//! [`crate::formatter`] respectively.

use std::ops::Range;

use crate::expr::Expr;
use crate::formatter::Policy;
use crate::scanner::{self, ScanItem, ScanKind, VS_EMOJI, VS_TEXT};
use crate::unicode::{self, DefaultSide};

/// The selector state currently present at a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectorState {
    /// No explicit variation selector.
    Bare,
    /// Explicit text presentation selector (`FE0E`).
    Text,
    /// Explicit emoji presentation selector (`FE0F`).
    Emoji,
}

impl SelectorState {
    /// Convert an optional selector character into a selector state.
    #[must_use]
    pub const fn from_selector(selector: Option<char>) -> Self {
        match selector {
            Some(VS_TEXT) => Self::Text,
            Some(VS_EMOJI) => Self::Emoji,
            _ => Self::Bare,
        }
    }

    /// Convert a selector state back to its character form, if explicit.
    #[must_use]
    pub const fn as_selector(self) -> Option<char> {
        match self {
            Self::Bare => None,
            Self::Text => Some(VS_TEXT),
            Self::Emoji => Some(VS_EMOJI),
        }
    }
}

/// A compact bitmap of which selector states are reasonable for a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReasonableSet(u8);

impl ReasonableSet {
    const NONE_BIT: u8 = 0b001;
    const TEXT_BIT: u8 = 0b010;
    const EMOJI_BIT: u8 = 0b100;

    /// Create an empty set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create a set from booleans for `bare`, `FE0E`, and `FE0F`.
    #[must_use]
    pub const fn new(bare: bool, text: bool, emoji: bool) -> Self {
        Self(
            (if bare { Self::NONE_BIT } else { 0 })
                | (if text { Self::TEXT_BIT } else { 0 })
                | (if emoji { Self::EMOJI_BIT } else { 0 }),
        )
    }

    /// Return whether the set contains the given selector state.
    #[must_use]
    pub const fn contains(self, state: SelectorState) -> bool {
        let bit = match state {
            SelectorState::Bare => Self::NONE_BIT,
            SelectorState::Text => Self::TEXT_BIT,
            SelectorState::Emoji => Self::EMOJI_BIT,
        };
        self.0 & bit != 0
    }

    /// Count how many selector states remain reasonable.
    #[must_use]
    pub const fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Return the sole selector state if the set has exactly one member.
    #[must_use]
    pub const fn only(self) -> Option<SelectorState> {
        match self.0 {
            Self::NONE_BIT => Some(SelectorState::Bare),
            Self::TEXT_BIT => Some(SelectorState::Text),
            Self::EMOJI_BIT => Some(SelectorState::Emoji),
            _ => None,
        }
    }
}

/// Slot kinds after context-aware classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// A standalone variation-sequence base with optional selector.
    StandaloneEvs,
    /// A `[0-9#*] _ 20E3` keycap slot.
    Keycap,
    /// A `Emoji_Modifier_Base FE0F Emoji_Modifier` legacy defect slot.
    ModifierDefect,
    /// The terminal selector-bearing position of a ZWJ component.
    ZwjTerminal,
    /// Not a policy-bearing slot.
    NotASlot,
}

/// Slot analysis result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotAnalysis<'a> {
    /// Byte range in the original input.
    pub span: Range<usize>,
    /// The classified slot kind.
    pub slot_kind: SlotKind,
    /// The selector state present in the input.
    pub current_state: SelectorState,
    /// Which selector states are reasonable in this context.
    pub reasonable_states: ReasonableSet,
    /// Whether the source contains extra trailing selectors beyond the first.
    pub has_extra_selectors: bool,
    /// The raw slice for this slot.
    pub raw: &'a str,
    /// The base code point when this slot has one.
    pub base: Option<char>,
}

/// Analyze a single scanner item into a slot-level representation.
#[must_use]
pub fn analyze_scan_item<'a>(item: &ScanItem<'a>) -> SlotAnalysis<'a> {
    match &item.kind {
        ScanKind::Singleton { base, selectors } => {
            let reasonable_states = unicode::variation_sequence_info(*base)
                .map_or(ReasonableSet::empty(), |info| {
                    ReasonableSet::new(true, info.has_text_vs, info.has_emoji_vs)
                });
            SlotAnalysis {
                span: item.span.clone(),
                slot_kind: SlotKind::StandaloneEvs,
                current_state: SelectorState::from_selector(scanner::effective_selector(selectors)),
                reasonable_states,
                has_extra_selectors: selectors.len() > 1,
                raw: item.raw,
                base: Some(*base),
            }
        }
        ScanKind::Keycap { selectors, .. } => SlotAnalysis {
            span: item.span.clone(),
            slot_kind: SlotKind::Keycap,
            current_state: SelectorState::from_selector(scanner::effective_selector(selectors)),
            reasonable_states: ReasonableSet::new(false, false, true),
            has_extra_selectors: selectors.len() > 1,
            raw: item.raw,
            base: None,
        },
        ScanKind::Zwj(sequence) => {
            let has_modifier_defect = zwj_any_component(sequence, &|component| {
                component.emoji_modifier.is_some()
                    && scanner::zwj_component_effective_selector(component) == Some(VS_EMOJI)
            });
            let current_state = zwj_find_component_selector(sequence)
                .map_or(SelectorState::Bare, |selector| {
                    SelectorState::from_selector(Some(selector))
                });
            SlotAnalysis {
                span: item.span.clone(),
                slot_kind: if has_modifier_defect {
                    SlotKind::ModifierDefect
                } else {
                    SlotKind::ZwjTerminal
                },
                current_state,
                reasonable_states: if has_modifier_defect {
                    ReasonableSet::new(true, false, false)
                } else {
                    ReasonableSet::new(false, false, true)
                },
                has_extra_selectors: zwj_has_extra_selectors(sequence),
                raw: item.raw,
                base: None,
            }
        }
        ScanKind::Passthrough | ScanKind::StandaloneSelectors(_) => SlotAnalysis {
            span: item.span.clone(),
            slot_kind: SlotKind::NotASlot,
            current_state: SelectorState::Bare,
            reasonable_states: ReasonableSet::empty(),
            has_extra_selectors: false,
            raw: item.raw,
            base: None,
        },
    }
}

fn zwj_any_component(
    sequence: &scanner::ZwjSequence,
    predicate: &dyn Fn(&scanner::ZwjComponent) -> bool,
) -> bool {
    match sequence {
        scanner::ZwjSequence::Terminal(component) => predicate(component),
        scanner::ZwjSequence::Joined { head, tail, .. } => {
            predicate(head) || zwj_any_component(tail, predicate)
        }
    }
}

fn zwj_find_component_selector(sequence: &scanner::ZwjSequence) -> Option<char> {
    match sequence {
        scanner::ZwjSequence::Terminal(component) => {
            scanner::zwj_component_effective_selector(component)
        }
        scanner::ZwjSequence::Joined { head, tail, .. } => {
            scanner::zwj_component_effective_selector(head)
                .or_else(|| zwj_find_component_selector(tail))
        }
    }
}

fn zwj_has_extra_selectors(sequence: &scanner::ZwjSequence) -> bool {
    match sequence {
        scanner::ZwjSequence::Terminal(component) => {
            (component.emoji_modifier.is_some() && !component.selectors_after_base.is_empty())
                || scanner::zwj_component_terminal_selectors(component).len() > 1
        }
        scanner::ZwjSequence::Joined { head, link, tail } => {
            (head.emoji_modifier.is_some() && !head.selectors_after_base.is_empty())
                || scanner::zwj_component_terminal_selectors(head).len() > 1
                || !link.selectors.is_empty()
                || zwj_has_extra_selectors(tail)
        }
    }
}

/// Scan and analyze an entire input string.
#[must_use]
pub fn analyze_text(input: &str) -> Vec<SlotAnalysis<'_>> {
    let items = scanner::scan(input);
    items.iter().map(analyze_scan_item).collect()
}

/// Borrowed view of the two policy predicates shared by internal resolution,
/// classification, and canonicalization code.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PolicyView<'a> {
    /// Expression for characters whose bare form is preferred.
    pub(crate) prefer_bare_for: &'a Expr,
    /// Expression for characters whose bare form means text presentation.
    pub(crate) treat_bare_as_text_for: &'a Expr,
}

/// Derive the formatter-resolved selector state for a slot.
///
/// Boundary:
/// this answers "which selector state should this slot resolve to?" for the
/// slot model. It does not assign violation categories and it does not render
/// repaired text.
///
/// Returns `None` when the item is not a real slot.
#[must_use]
pub fn canonical_state(analysis: &SlotAnalysis<'_>, policy: &Policy) -> Option<SelectorState> {
    canonical_state_with_view(analysis, &policy.as_view())
}

pub(crate) fn canonical_state_with_view(
    analysis: &SlotAnalysis<'_>,
    policy: &PolicyView<'_>,
) -> Option<SelectorState> {
    match analysis.slot_kind {
        SlotKind::StandaloneEvs => {
            let base = analysis.base?;
            Some(resolve_singleton_with_view(
                base,
                analysis.current_state.as_selector(),
                policy,
            ))
        }
        SlotKind::Keycap | SlotKind::ModifierDefect | SlotKind::ZwjTerminal => {
            analysis.reasonable_states.only()
        }
        SlotKind::NotASlot => None,
    }
}

/// Resolve the canonical selector state for a standalone variation-sequence
/// character
/// without requiring a full [`ScanItem`].
///
/// `effective_selector` is the first variation selector trailing the base, if
/// any. Returns the selector state the formatter should emit.
#[must_use]
pub fn resolve_singleton(
    base: char,
    effective_selector: Option<char>,
    policy: &Policy,
) -> SelectorState {
    resolve_singleton_with_view(base, effective_selector, &policy.as_view())
}

pub(crate) fn resolve_singleton_with_view(
    base: char,
    effective_selector: Option<char>,
    policy: &PolicyView<'_>,
) -> SelectorState {
    let reasonable_states = unicode::variation_sequence_info(base)
        .map_or(ReasonableSet::empty(), |info| {
            ReasonableSet::new(true, info.has_text_vs, info.has_emoji_vs)
        });
    let raw_state = SelectorState::from_selector(effective_selector);
    let current = if reasonable_states.contains(raw_state) {
        raw_state
    } else {
        SelectorState::Bare
    };
    let bare_side = if policy.treat_bare_as_text_for.matches(base) {
        SelectorState::Text
    } else {
        SelectorState::Emoji
    };

    if policy.prefer_bare_for.matches(base) {
        match current {
            SelectorState::Text | SelectorState::Emoji if current != bare_side => current,
            SelectorState::Bare | SelectorState::Text | SelectorState::Emoji => SelectorState::Bare,
        }
    } else {
        match current {
            SelectorState::Text | SelectorState::Emoji => current,
            SelectorState::Bare => bare_side,
        }
    }
}

/// Return the canonical selector for a single ZWJ component.
///
/// - Variation-sequence text-default components without an emoji modifier
///   canonically carry `FE0F`.
/// - Variation-sequence components that are emoji-default or carry an emoji modifier
///   canonically have no selector (the modifier or default implies emoji).
/// - Components without variation-sequence data canonically have no selector; unsupported
///   selectors in ZWJ context are removed.
#[must_use]
pub fn canonical_zwj_component_selector(comp: &scanner::ZwjComponent) -> Option<char> {
    if let Some(info) = unicode::variation_sequence_info(comp.base) {
        if info.default_side == DefaultSide::Text && comp.emoji_modifier.is_none() {
            Some(VS_EMOJI)
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

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
        let prefer_bare_for = crate::expr::parse_expr_only("ascii").unwrap();
        let bare_is_text_for = crate::expr::parse_expr_only("ascii").unwrap();
        let policy = PolicyView {
            prefer_bare_for: &prefer_bare_for,
            treat_bare_as_text_for: &bare_is_text_for,
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
        let prefer_bare_for = crate::expr::parse_expr_only("ascii").unwrap();
        let bare_is_text_for = crate::expr::parse_expr_only("ascii").unwrap();
        let policy = PolicyView {
            prefer_bare_for: &prefer_bare_for,
            treat_bare_as_text_for: &bare_is_text_for,
        };
        // ©️ is non-ASCII → not in bare_is_text_for → bare resolves to emoji.
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
}
