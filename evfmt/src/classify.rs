//! Policy-aware diagnostics for scanned emoji variation structures.
//!
//! It sits above [`crate::scanner`] and [`crate::slot`]:
//!
//! - [`crate::scanner`] decides structural item boundaries
//! - [`crate::slot`] provides slot-level analysis for policy-bearing positions
//! - `classify` turns that structural information into violation categories

use crate::formatter::Policy;
use crate::scanner::{self, ScanItem, ScanKind};
use crate::slot::{self, PolicyView, SelectorState};

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

fn classify_with_view(item: &ScanItem<'_>, policy: &PolicyView<'_>) -> Option<ViolationKind> {
    match &item.kind {
        ScanKind::Passthrough => None,
        ScanKind::StandaloneSelectors(_) => Some(ViolationKind::IllegalSelector),
        ScanKind::Singleton { .. } => violation_for_singleton(item, policy),
        ScanKind::Keycap { .. } | ScanKind::Zwj(_) => violation_for_sequence(item),
    }
}

fn violation_for_singleton(item: &ScanItem<'_>, policy: &PolicyView<'_>) -> Option<ViolationKind> {
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
mod tests;
