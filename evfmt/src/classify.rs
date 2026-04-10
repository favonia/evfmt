//! Policy-aware diagnostics for scanned emoji variation structures.
//!
//! It sits above [`crate::scanner`] and [`crate::slot`]:
//!
//! - [`crate::scanner`] decides structural item boundaries
//! - [`crate::slot`] provides slot-level analysis for policy-bearing positions
//! - `classify` turns that structural information into violation categories

use crate::formatter::Policy;
use crate::scanner::{self, ScanItem, ScanKind};
use crate::slot::{self, SelectorState};

/// Classification of how a scanned item violates canonical form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViolationKind {
    /// Standalone, extra, or unsanctioned selector.
    IllegalSelector,
    /// Selector matching the bare side on a bare-preferred character.
    RedundantSelector,
    /// Bare variation-sequence character without a selector when bare is not preferred.
    BareNeedsResolution,
    /// Wrong or missing FE0F in a keycap or ZWJ sequence.
    SequenceSelectorViolation,
}

/// Classify a scanned item under the current formatter policy.
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, ViolationKind, classify, scan};
///
/// let policy = Policy::default();
/// let items = scan("#\u{FE0E}");
///
/// assert_eq!(
///     classify(&items[0], &policy),
///     Some(ViolationKind::RedundantSelector)
/// );
/// ```
#[must_use]
pub fn classify(item: &ScanItem<'_>, policy: &Policy) -> Option<ViolationKind> {
    classify_with_view(item, &policy.as_view())
}

fn classify_with_view(item: &ScanItem<'_>, policy: &slot::PolicyView<'_>) -> Option<ViolationKind> {
    match &item.kind {
        ScanKind::Passthrough => None,
        ScanKind::StandaloneSelectors(_) => Some(ViolationKind::IllegalSelector),
        ScanKind::Singleton { .. } => violation_for_singleton(item, policy),
        ScanKind::Keycap { .. } | ScanKind::Zwj(_) => violation_for_sequence(item),
    }
}

fn violation_for_singleton(
    item: &ScanItem<'_>,
    policy: &slot::PolicyView<'_>,
) -> Option<ViolationKind> {
    let analysis = slot::analyze_scan_item(item);

    if analysis.has_extra_selectors || !analysis.reasonable_states.contains(analysis.current_state)
    {
        return Some(ViolationKind::IllegalSelector);
    }

    let canonical = slot::canonical_state_with_view(&analysis, policy)?;

    if analysis.current_state == canonical {
        return None;
    }

    match (analysis.current_state, canonical) {
        (SelectorState::Bare, SelectorState::Text | SelectorState::Emoji) => {
            Some(ViolationKind::BareNeedsResolution)
        }
        (SelectorState::Text | SelectorState::Emoji, SelectorState::Bare) => {
            Some(ViolationKind::RedundantSelector)
        }
        _ => None,
    }
}

fn violation_for_sequence(item: &ScanItem<'_>) -> Option<ViolationKind> {
    let analysis = slot::analyze_scan_item(item);

    if analysis.has_extra_selectors {
        return Some(ViolationKind::SequenceSelectorViolation);
    }

    // For keycap/ZWJ, check per-component selectors.
    if let ScanKind::Zwj(sequence) = &item.kind {
        if zwj_has_noncanonical_component_selector(sequence) {
            return Some(ViolationKind::SequenceSelectorViolation);
        }
        return None;
    }

    // Keycap / ModifierDefect / ZwjTerminal: single canonical state.
    let canonical = analysis.reasonable_states.only()?;
    if analysis.current_state == canonical {
        None
    } else {
        Some(ViolationKind::SequenceSelectorViolation)
    }
}

fn zwj_has_noncanonical_component_selector(sequence: &scanner::ZwjSequence) -> bool {
    match sequence {
        scanner::ZwjSequence::Terminal(component) => {
            let current = scanner::zwj_component_effective_selector(component);
            let canonical = slot::canonical_zwj_component_selector(component);
            current != canonical
        }
        scanner::ZwjSequence::Joined { head, tail, .. } => {
            let current = scanner::zwj_component_effective_selector(head);
            let canonical = slot::canonical_zwj_component_selector(head);
            current != canonical || zwj_has_noncanonical_component_selector(tail)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::expr;
    use crate::find_violations;
    use crate::scanner::scan;

    fn default_policy() -> Policy {
        Policy {
            prefer_bare_for: expr::parse_expr_only("ascii").unwrap(),
            treat_bare_as_text_for: expr::parse_expr_only("ascii").unwrap(),
        }
    }

    #[test]
    fn test_classify_passthrough() {
        let items = scan("Hello");
        assert_eq!(classify(&items[0], &default_policy()), None);
    }

    #[test]
    fn test_classify_standalone_selector_run() {
        let items = scan("\u{FE0F}");
        assert_eq!(
            classify(&items[0], &default_policy()),
            Some(ViolationKind::IllegalSelector)
        );
    }

    #[test]
    fn test_classify_singleton_canonical() {
        let policy = default_policy();
        // # bare, bare-preferred → canonical
        let items = scan("#");
        assert_eq!(classify(&items[0], &policy), None);
    }

    #[test]
    fn test_classify_singleton_redundant() {
        let policy = default_policy();
        // # + FE0E, bare-preferred, bare side=text → FE0E redundant
        let items = scan("#\u{FE0E}");
        assert_eq!(
            classify(&items[0], &policy),
            Some(ViolationKind::RedundantSelector)
        );
    }

    #[test]
    fn test_classify_singleton_bare_needs_resolution() {
        let policy = default_policy();
        // ©️ bare, not bare-preferred → needs resolution
        let items = scan("\u{00A9}");
        assert_eq!(
            classify(&items[0], &policy),
            Some(ViolationKind::BareNeedsResolution)
        );
    }

    #[test]
    fn test_classify_singleton_with_extra_selector_is_illegal() {
        let policy = default_policy();
        let items = scan("#\u{FE0F}\u{FE0E}");
        assert_eq!(
            classify(&items[0], &policy),
            Some(ViolationKind::IllegalSelector)
        );
    }

    #[test]
    fn test_find_violations_reports_all_non_canonical_items() {
        let policy = default_policy();
        let findings = find_violations("A\u{FE0F} #\u{FE0E} \u{00A9}", &policy);
        assert_eq!(findings.len(), 3);
        assert_eq!(findings[0].violation, ViolationKind::IllegalSelector);
        assert_eq!(findings[0].raw, "\u{FE0F}");
        assert_eq!(findings[0].replacement, "");
        assert_eq!(findings[1].violation, ViolationKind::RedundantSelector);
        assert_eq!(findings[1].replacement, "#");
        assert_eq!(findings[2].violation, ViolationKind::BareNeedsResolution);
        assert_eq!(findings[2].replacement, "\u{00A9}\u{FE0F}");
    }

    #[test]
    fn test_classify_keycap_correct() {
        let items = scan("#\u{FE0F}\u{20E3}");
        assert_eq!(classify(&items[0], &default_policy()), None);
    }

    #[test]
    fn test_classify_keycap_missing_fe0f() {
        let items = scan("#\u{20E3}");
        assert_eq!(
            classify(&items[0], &default_policy()),
            Some(ViolationKind::SequenceSelectorViolation)
        );
    }

    #[test]
    fn test_classify_keycap_wrong_vs() {
        let items = scan("#\u{FE0E}\u{20E3}");
        assert_eq!(
            classify(&items[0], &default_policy()),
            Some(ViolationKind::SequenceSelectorViolation)
        );
    }

    #[test]
    fn test_classify_zwj_correct() {
        // ❤️ FE0F ZWJ 🔥 — ❤️ is text-default with FE0F → correct
        let items = scan("\u{2764}\u{FE0F}\u{200D}\u{1F525}");
        assert_eq!(classify(&items[0], &default_policy()), None);
    }

    #[test]
    fn test_classify_zwj_missing_fe0f() {
        // ❤️ ZWJ 🔥 — ❤️ is text-default, bare → violation
        let items = scan("\u{2764}\u{200D}\u{1F525}");
        assert_eq!(
            classify(&items[0], &default_policy()),
            Some(ViolationKind::SequenceSelectorViolation)
        );
    }

    #[test]
    fn test_classify_zwj_wrong_vs() {
        // ❤️ FE0E ZWJ 🔥 — ❤️ is text-default with FE0E → violation
        let items = scan("\u{2764}\u{FE0E}\u{200D}\u{1F525}");
        assert_eq!(
            classify(&items[0], &default_policy()),
            Some(ViolationKind::SequenceSelectorViolation)
        );
    }

    #[test]
    fn test_classify_zwj_non_eligible_selector_is_violation() {
        // 😀 FE0F ZWJ 🔥 — 😀 has no sanctioned variation sequence → violation
        let items = scan("\u{1F600}\u{FE0F}\u{200D}\u{1F525}");
        assert_eq!(
            classify(&items[0], &default_policy()),
            Some(ViolationKind::SequenceSelectorViolation)
        );
    }
}
