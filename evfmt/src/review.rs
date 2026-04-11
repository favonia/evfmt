//! Policy-aware review findings for scanned emoji variation structures.
//!
//! This module implements the policy and fixed-rule review steps from the
//! conceptual formatting algorithm. It produces reviewable findings with valid
//! replacement decisions and precomputed replacements, so callers can choose a
//! replacement without re-reading policy for the same item.
//! Interactive callers normally use this module directly: `review` computes
//! reasonableness, applies [`Policy`] only where policy is relevant, and stores
//! the valid replacements in each [`ReviewFinding`].
//!
//! - [`crate::scanner`] decides structural item boundaries
//! - `review` turns policy-neutral reasonableness into findings and replacement choices

use std::ops::Range;

use crate::policy::{Policy, SingletonRule};
use crate::scanner::{self, KEYCAP_CAP, ScanItem, ScanKind, VariationSelector, ZWJ};
use crate::unicode::{self, DefaultSide};

/// Category describing how a scanned item violates canonical form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ViolationKind {
    /// Standalone, extra, or otherwise unsanctioned variation selector.
    ///
    /// Extra or unsanctioned variation selectors inside a keycap or ZWJ sequence
    /// are reported as [`Self::NotFullyQualifiedEmojiSequence`] instead.
    IllegalVariationSelector,
    /// Wrong or missing variation selectors in a keycap or ZWJ sequence.
    NotFullyQualifiedEmojiSequence,
    /// Sanctioned variation selector that matches the bare side on a bare-preferred
    /// character. The selector is valid, but the formatting policy prefers bare form.
    RedundantVariationSelector,
    /// Bare variation-sequence character that the formatting policy wants to resolve
    /// with a variation selector. The bare form is valid, but the policy prefers an
    /// explicit presentation selector.
    BareNeedsResolution,
}

/// Replacement-producing decisions available for a review finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReplacementDecision {
    /// Apply the unambiguous formatter repair.
    Fix,
    /// Resolve a bare standalone slot as text presentation.
    Text,
    /// Resolve a bare standalone slot as emoji presentation.
    Emoji,
}

/// Valid replacement decision set and precomputed replacements for a review finding.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReviewPlan {
    /// Finding with one unambiguous repair.
    Repair {
        /// Why the item is non-canonical.
        violation: ViolationKind,
        /// Replacement for [`ReplacementDecision::Fix`].
        fix_replacement: String,
    },
    /// Bare variation-sequence character that must be resolved explicitly.
    ResolvePresentation {
        /// The presentation selected by batch formatting.
        default: VariationSelector,
        /// Replacement for [`ReplacementDecision::Text`].
        text_replacement: String,
        /// Replacement for [`ReplacementDecision::Emoji`].
        emoji_replacement: String,
    },
}

impl ReviewPlan {
    const REPAIR_CHOICES: [ReplacementDecision; 1] = [ReplacementDecision::Fix];
    const PRESENTATION_CHOICES: [ReplacementDecision; 2] =
        [ReplacementDecision::Text, ReplacementDecision::Emoji];

    /// Why the reviewed item is non-canonical.
    #[must_use]
    const fn violation(&self) -> ViolationKind {
        match self {
            Self::Repair { violation, .. } => *violation,
            Self::ResolvePresentation { .. } => ViolationKind::BareNeedsResolution,
        }
    }

    /// Valid replacement decisions for this plan.
    #[must_use]
    const fn choices(&self) -> &'static [ReplacementDecision] {
        match self {
            Self::Repair { .. } => &Self::REPAIR_CHOICES,
            Self::ResolvePresentation { .. } => &Self::PRESENTATION_CHOICES,
        }
    }

    /// The replacement decision batch formatting applies by default.
    #[must_use]
    const fn default_decision(&self) -> ReplacementDecision {
        match self {
            Self::Repair { .. } => ReplacementDecision::Fix,
            Self::ResolvePresentation { default, .. } => match default {
                VariationSelector::Text => ReplacementDecision::Text,
                VariationSelector::Emoji => ReplacementDecision::Emoji,
            },
        }
    }

    /// The replacement batch formatting applies by default.
    #[must_use]
    fn default_replacement(&self) -> &str {
        match self {
            Self::Repair {
                fix_replacement, ..
            } => fix_replacement,
            Self::ResolvePresentation {
                default,
                text_replacement,
                emoji_replacement,
            } => match default {
                VariationSelector::Text => text_replacement,
                VariationSelector::Emoji => emoji_replacement,
            },
        }
    }
}

/// A single non-canonical scanned item with its valid replacement decisions and replacements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFinding<'a> {
    /// Byte range in the original input.
    pub span: Range<usize>,
    /// Original raw source slice for the item.
    pub raw: &'a str,
    /// Valid replacement decision set and precomputed replacements for this finding.
    plan: ReviewPlan,
}

impl ReviewFinding<'_> {
    /// Why the reviewed item is non-canonical.
    #[must_use]
    pub const fn violation(&self) -> ViolationKind {
        self.plan.violation()
    }

    /// Valid replacement decisions for this finding.
    #[must_use]
    pub const fn choices(&self) -> &'static [ReplacementDecision] {
        self.plan.choices()
    }

    /// The replacement decision batch formatting applies by default.
    #[must_use]
    pub const fn default_decision(&self) -> ReplacementDecision {
        self.plan.default_decision()
    }

    /// The replacement text for the decision batch formatting applies by default.
    ///
    /// This is the infallible borrowed form of:
    ///
    /// ```text
    /// finding.replacement(finding.default_decision()).unwrap()
    /// ```
    ///
    /// The default decision is always one of this finding's valid replacement
    /// choices. Call [`str::to_owned`] when the replacement must be stored as
    /// an owned [`String`].
    #[must_use]
    pub fn default_replacement(&self) -> &str {
        self.plan.default_replacement()
    }

    /// Return the replacement text for a valid replacement decision.
    ///
    /// Valid replacement decisions return the replacement computed when this
    /// finding was created, so callers do not need to consult [`Policy`] again.
    /// The returned string is borrowed from this finding. Returns `None` when
    /// the decision is not one of this finding's valid replacement choices.
    /// Callers that want to skip a finding can keep [`ReviewFinding::raw`].
    #[must_use]
    pub fn replacement(&self, decision: ReplacementDecision) -> Option<&str> {
        match (&self.plan, decision) {
            (
                ReviewPlan::Repair {
                    fix_replacement, ..
                },
                ReplacementDecision::Fix,
            ) => Some(fix_replacement),
            (
                ReviewPlan::ResolvePresentation {
                    text_replacement, ..
                },
                ReplacementDecision::Text,
            ) => Some(text_replacement),
            (
                ReviewPlan::ResolvePresentation {
                    emoji_replacement, ..
                },
                ReplacementDecision::Emoji,
            ) => Some(emoji_replacement),
            (ReviewPlan::Repair { .. }, ReplacementDecision::Text | ReplacementDecision::Emoji)
            | (ReviewPlan::ResolvePresentation { .. }, ReplacementDecision::Fix) => None,
        }
    }
}

fn unambiguous_finding<'a>(
    item: &ScanItem<'a>,
    violation: ViolationKind,
    fix_replacement: String,
) -> ReviewFinding<'a> {
    ReviewFinding {
        span: item.span.clone(),
        raw: item.raw,
        plan: ReviewPlan::Repair {
            violation,
            fix_replacement,
        },
    }
}

/// Review a scanned item under the current formatter policy.
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, ViolationKind, review_item, scan};
///
/// let policy = Policy::default();
/// let items = scan("#\u{FE0E}");
///
/// assert_eq!(
///     review_item(&items[0], &policy).map(|finding| finding.violation()),
///     Some(ViolationKind::RedundantVariationSelector),
/// );
/// ```
#[must_use]
pub fn review_item<'a>(item: &ScanItem<'a>, policy: &Policy) -> Option<ReviewFinding<'a>> {
    match &item.kind {
        ScanKind::Passthrough => None,
        ScanKind::StandaloneVariationSelectors(_) => Some(unambiguous_finding(
            item,
            ViolationKind::IllegalVariationSelector,
            String::new(),
        )),
        ScanKind::Singleton {
            base,
            variation_selectors,
        } => review_singleton(item, *base, variation_selectors, policy),
        ScanKind::Keycap {
            variation_selectors,
            ..
        } => review_keycap(item, variation_selectors),
        ScanKind::Zwj(sequence) => review_zwj(item, sequence),
    }
}

/// Review every non-canonical item in an input string.
///
/// This is the convenience batch wrapper around [`scanner::scan`] plus
/// [`review_item`]. It eagerly collects findings into a [`Vec`].
///
/// Use this for whole-input diagnostics. For interactive repair flows that
/// apply replacement decisions while walking scanned items, use [`scanner::scan`] together
/// with [`review_item`].
#[must_use]
pub fn review_text<'a>(input: &'a str, policy: &Policy) -> Vec<ReviewFinding<'a>> {
    scanner::scan(input)
        .into_iter()
        .filter_map(|item| review_item(&item, policy))
        .collect()
}

fn review_singleton<'a>(
    item: &ScanItem<'a>,
    base: char,
    variation_selectors: &[VariationSelector],
    policy: &Policy,
) -> Option<ReviewFinding<'a>> {
    if !unicode::has_variation_sequence(base) {
        return Some(singleton_illegal_variation_selector_finding(
            item, base, None,
        ));
    }

    match (policy.singleton_rule(base), variation_selectors) {
        (SingletonRule::TextToBare, &[VariationSelector::Text, _, ..])
        | (SingletonRule::EmojiToBare, &[VariationSelector::Emoji, _, ..]) => Some(
            singleton_illegal_variation_selector_finding(item, base, None),
        ),
        (_, &[current_selector, _, ..]) => Some(singleton_illegal_variation_selector_finding(
            item,
            base,
            Some(current_selector),
        )),
        (SingletonRule::BareToEmoji, &[]) => Some(resolve_presentation_finding(
            item,
            base,
            VariationSelector::Emoji,
        )),
        (SingletonRule::BareToText, &[]) => Some(resolve_presentation_finding(
            item,
            base,
            VariationSelector::Text,
        )),
        (SingletonRule::TextToBare, &[VariationSelector::Text])
        | (SingletonRule::EmojiToBare, &[VariationSelector::Emoji]) => Some(unambiguous_finding(
            item,
            ViolationKind::RedundantVariationSelector,
            render_singleton_selector(base, None),
        )),
        // Spell out all the remaining cases to make sure we did not miss any.
        (SingletonRule::TextToBare, &[VariationSelector::Emoji] | &[])
        | (SingletonRule::EmojiToBare, &[VariationSelector::Text] | &[])
        | (
            SingletonRule::BareToText | SingletonRule::BareToEmoji,
            &[VariationSelector::Text] | &[VariationSelector::Emoji],
        ) => None,
    }
}

fn resolve_presentation_finding<'a>(
    item: &ScanItem<'a>,
    base: char,
    default: VariationSelector,
) -> ReviewFinding<'a> {
    ReviewFinding {
        span: item.span.clone(),
        raw: item.raw,
        plan: ReviewPlan::ResolvePresentation {
            default,
            text_replacement: render_singleton_selector(base, Some(VariationSelector::Text)),
            emoji_replacement: render_singleton_selector(base, Some(VariationSelector::Emoji)),
        },
    }
}

fn singleton_illegal_variation_selector_finding<'a>(
    item: &ScanItem<'a>,
    base: char,
    fix_selector: Option<VariationSelector>,
) -> ReviewFinding<'a> {
    ReviewFinding {
        span: item.span.clone(),
        raw: item.raw,
        plan: ReviewPlan::Repair {
            violation: ViolationKind::IllegalVariationSelector,
            fix_replacement: render_singleton_selector(base, fix_selector),
        },
    }
}

fn render_singleton_selector(base: char, selector: Option<VariationSelector>) -> String {
    let mut output = String::new();
    output.push(base);
    if let Some(vs) = selector {
        output.push(vs.to_char());
    }
    output
}

fn review_keycap<'a>(
    item: &ScanItem<'a>,
    variation_selectors: &[VariationSelector],
) -> Option<ReviewFinding<'a>> {
    if variation_selectors.len() > 1
        || scanner::effective_selector(variation_selectors) != Some(VariationSelector::Emoji)
    {
        Some(unambiguous_finding(
            item,
            ViolationKind::NotFullyQualifiedEmojiSequence,
            render_sequence_fix(item)?,
        ))
    } else {
        None
    }
}

fn review_zwj<'a>(
    item: &ScanItem<'a>,
    sequence: &scanner::ZwjSequence,
) -> Option<ReviewFinding<'a>> {
    if zwj_needs_repair(sequence) {
        Some(unambiguous_finding(
            item,
            ViolationKind::NotFullyQualifiedEmojiSequence,
            render_sequence_fix(item)?,
        ))
    } else {
        None
    }
}

fn render_sequence_fix(item: &ScanItem<'_>) -> Option<String> {
    match &item.kind {
        ScanKind::Keycap { base, .. } => {
            let mut output = String::with_capacity(item.raw.len());
            output.push(*base);
            output.push(VariationSelector::Emoji.to_char());
            output.push(KEYCAP_CAP);
            Some(output)
        }
        ScanKind::Zwj(sequence) => {
            let mut output = String::with_capacity(item.raw.len());
            render_zwj(&mut output, sequence);
            Some(output)
        }
        ScanKind::Passthrough
        | ScanKind::StandaloneVariationSelectors(_)
        | ScanKind::Singleton { .. } => None,
    }
}

fn render_zwj(output: &mut String, sequence: &scanner::ZwjSequence) {
    match sequence {
        scanner::ZwjSequence::Terminal(component) => {
            render_zwj_component(output, component);
        }
        scanner::ZwjSequence::Joined { head, tail, .. } => {
            render_zwj_component(output, head);
            output.push(ZWJ);
            render_zwj(output, tail);
        }
    }
}

fn render_zwj_component(output: &mut String, component: &scanner::ZwjComponent) {
    output.push(component.base);
    if let Some(emoji_modifier) = component.emoji_modifier {
        output.push(emoji_modifier);
    }
    if let Some(vs) = canonical_zwj_component_selector(component) {
        output.push(vs.to_char());
    }
}

/// Return the canonical variation selector for a single ZWJ component.
///
/// This is a fixed cleanup rule from the conceptual algorithm, not a
/// user-configurable policy choice.
fn canonical_zwj_component_selector(comp: &scanner::ZwjComponent) -> Option<VariationSelector> {
    if let Some(info) = unicode::variation_sequence_info(comp.base) {
        if info.default_side == DefaultSide::Text && comp.emoji_modifier.is_none() {
            Some(VariationSelector::Emoji)
        } else {
            None
        }
    } else {
        None
    }
}

fn zwj_component_needs_repair(component: &scanner::ZwjComponent) -> bool {
    let has_extra = (component.emoji_modifier.is_some()
        && !component.variation_selectors_after_base.is_empty())
        || scanner::zwj_component_terminal_selectors(component).len() > 1;
    let current = scanner::zwj_component_effective_selector(component);
    let canonical = canonical_zwj_component_selector(component);
    has_extra || current != canonical
}

fn zwj_needs_repair(sequence: &scanner::ZwjSequence) -> bool {
    match sequence {
        scanner::ZwjSequence::Terminal(component) => zwj_component_needs_repair(component),
        scanner::ZwjSequence::Joined { head, link, tail } => {
            zwj_component_needs_repair(head)
                || !link.variation_selectors.is_empty()
                || zwj_needs_repair(tail)
        }
    }
}

#[cfg(test)]
mod tests;
