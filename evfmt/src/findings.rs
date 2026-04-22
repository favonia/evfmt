//! Policy-aware findings for scanned emoji variation structures.
//!
//! This module implements the policy and fixed-rule analysis steps from the
//! conceptual formatting algorithm. It produces findings with valid replacement
//! decisions and precomputed replacements, so callers can choose a replacement
//! without re-reading policy for the same item.
//! Interactive callers normally use this module directly: [`analyze_scan_item`]
//! computes reasonableness, applies [`Policy`] only where policy is relevant,
//! and stores the valid replacements in each [`Finding`].
//!
//! - [`crate::scanner`] decides structural item boundaries
//! - [`analyze_scan_item`] turns policy-neutral reasonableness into findings and replacement choices

use std::ops::Range;

use crate::policy::{Policy, SingletonRule};
use crate::scanner::{
    EmojiLike, EmojiModification, EmojiSequence, EmojiStem, Presentation, ScanItem, ScanKind,
    ZwjJoinedEmoji, ZwjLink,
};
use crate::unicode;

mod fixed;

use fixed::FixedEmojiLike;

#[cfg(test)]
mod tests;

// --- Public violation / decision types ---

/// Reason a scanned item is non-canonical.
///
/// Unsanctioned presentation selectors are tracked as a separate axis because
/// they can be the whole problem, or they can appear alongside a primary
/// sequence/policy violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Violation {
    /// The finding only contains unsanctioned presentation selectors.
    ///
    /// This can happen either as an isolated selector run or inside an
    /// otherwise canonical emoji-shaped item.
    UnsanctionedSelectorsOnly,
    /// The finding has a primary violation, and may also contain unsanctioned
    /// presentation selectors.
    Primary(PrimaryViolation),
}

impl Violation {
    const fn primary(kind: PrimaryViolationKind, has_unsanctioned_selectors: bool) -> Self {
        Self::Primary(PrimaryViolation::new(kind, has_unsanctioned_selectors))
    }

    const fn with_unsanctioned_selectors(self) -> Self {
        match self {
            Self::UnsanctionedSelectorsOnly => Self::UnsanctionedSelectorsOnly,
            Self::Primary(primary) => Self::Primary(PrimaryViolation {
                has_unsanctioned_selectors: true,
                ..primary
            }),
        }
    }
}

/// A primary sequence or policy violation, plus any attached selector cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PrimaryViolation {
    /// The primary reason this item is non-canonical.
    pub kind: PrimaryViolationKind,
    /// Whether the same item also contains unsanctioned presentation selectors.
    pub has_unsanctioned_selectors: bool,
}

impl PrimaryViolation {
    const fn new(kind: PrimaryViolationKind, has_unsanctioned_selectors: bool) -> Self {
        Self {
            kind,
            has_unsanctioned_selectors,
        }
    }
}

/// Primary sequence or policy category for a non-canonical scanned item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PrimaryViolationKind {
    /// Wrong or missing presentation selectors in a keycap or ZWJ sequence.
    NotFullyQualifiedSequence,
    /// Sanctioned presentation selector that matches the bare side on a bare-preferred
    /// character. The requested presentation is valid, but the formatting policy
    /// prefers bare form.
    RedundantSelector,
    /// Bare variation-sequence character that the formatting policy wants to resolve
    /// with a presentation selector. The bare form is valid, but the policy prefers an
    /// explicit presentation selector.
    BareNeedsResolution,
}

/// Replacement-producing decisions available for a finding.
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

/// Valid replacement decisions and precomputed replacement strings for a finding.
///
/// This is a plan in the narrow sense: it interprets a later
/// [`ReplacementDecision`] into the already-computed replacement text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplacementPlan {
    /// Finding with one unambiguous repair.
    Repair {
        /// Why the item is non-canonical.
        violation: Violation,
        /// Replacement for [`ReplacementDecision::Fix`].
        fix_replacement: String,
    },
    /// Bare variation-sequence character that must be resolved explicitly.
    ResolvePresentation {
        /// Why the item is non-canonical.
        violation: Violation,
        /// The presentation selected by batch formatting.
        default: Presentation,
        /// Replacement for [`ReplacementDecision::Text`].
        text_replacement: String,
        /// Replacement for [`ReplacementDecision::Emoji`].
        emoji_replacement: String,
    },
}

impl ReplacementPlan {
    const REPAIR_CHOICES: [ReplacementDecision; 1] = [ReplacementDecision::Fix];
    const PRESENTATION_CHOICES: [ReplacementDecision; 2] =
        [ReplacementDecision::Text, ReplacementDecision::Emoji];

    /// Why the analyzed item is non-canonical.
    #[must_use]
    const fn violation(&self) -> Violation {
        match self {
            Self::Repair { violation, .. } | Self::ResolvePresentation { violation, .. } => {
                *violation
            }
        }
    }

    /// Valid replacement decisions for this finding.
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
            Self::ResolvePresentation { default, .. } => match *default {
                Presentation::Text => ReplacementDecision::Text,
                Presentation::Emoji => ReplacementDecision::Emoji,
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
                ..
            } => match *default {
                Presentation::Text => text_replacement,
                Presentation::Emoji => emoji_replacement,
            },
        }
    }
}

/// A single non-canonical scanned item with its valid replacement decisions and replacements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding<'a> {
    /// Byte range in the original input.
    pub span: Range<usize>,
    /// Original raw source slice for the item.
    pub raw: &'a str,
    /// How valid replacement decisions map to replacement text for this finding.
    replacement_plan: ReplacementPlan,
}

impl Finding<'_> {
    /// Why the analyzed item is non-canonical.
    #[must_use]
    pub const fn violation(&self) -> Violation {
        self.replacement_plan.violation()
    }

    /// Valid replacement decisions for this finding.
    #[must_use]
    pub const fn choices(&self) -> &'static [ReplacementDecision] {
        self.replacement_plan.choices()
    }

    /// The replacement decision batch formatting applies by default.
    #[must_use]
    pub const fn default_decision(&self) -> ReplacementDecision {
        self.replacement_plan.default_decision()
    }

    /// The replacement text for the decision batch formatting applies by default.
    ///
    /// This is the infallible borrowed form of:
    ///
    /// ```rust
    /// use evfmt::Policy;
    /// use evfmt::findings::analyze_scan_item;
    /// use evfmt::scanner::scan;
    ///
    /// let policy = Policy::default();
    /// let finding = scan("A\u{FE0F}")
    ///     .find_map(|item| analyze_scan_item(&item, &policy))
    ///     .unwrap();
    ///
    /// finding.replacement(finding.default_decision()).unwrap();
    /// ```
    ///
    /// The default decision is always one of this finding's valid replacement
    /// choices. Call [`str::to_owned`] when the replacement must be stored as
    /// an owned [`String`].
    #[must_use]
    pub fn default_replacement(&self) -> &str {
        self.replacement_plan.default_replacement()
    }

    /// Return the replacement text for a valid replacement decision.
    ///
    /// Valid replacement decisions return the replacement computed when this
    /// finding was created, so callers do not need to consult [`Policy`] again.
    /// The returned string is borrowed from this finding. Returns `None` when
    /// the decision is not one of this finding's valid replacement choices.
    /// Callers that want to skip a finding can keep [`Finding::raw`].
    #[must_use]
    pub fn replacement(&self, decision: ReplacementDecision) -> Option<&str> {
        match (&self.replacement_plan, decision) {
            (
                ReplacementPlan::Repair {
                    fix_replacement, ..
                },
                ReplacementDecision::Fix,
            ) => Some(fix_replacement),
            (
                ReplacementPlan::ResolvePresentation {
                    text_replacement, ..
                },
                ReplacementDecision::Text,
            ) => Some(text_replacement),
            (
                ReplacementPlan::ResolvePresentation {
                    emoji_replacement, ..
                },
                ReplacementDecision::Emoji,
            ) => Some(emoji_replacement),
            (
                ReplacementPlan::Repair { .. },
                ReplacementDecision::Text | ReplacementDecision::Emoji,
            )
            | (ReplacementPlan::ResolvePresentation { .. }, ReplacementDecision::Fix) => None,
        }
    }
}

// --- Public API ---

/// Analyze a scanned item under the current formatter policy.
#[must_use]
pub fn analyze_scan_item<'a>(item: &ScanItem<'a>, policy: &Policy) -> Option<Finding<'a>> {
    match &item.kind {
        ScanKind::Passthrough => None,
        ScanKind::UnsanctionedPresentationSelectors(_) => Some(unambiguous_finding(
            item,
            Violation::UnsanctionedSelectorsOnly,
            String::new(),
        )),
        ScanKind::EmojiSequence(sequence) => match sequence {
            // The scanner preserves malformed ZWJ-like shapes such as leading,
            // consecutive, or trailing ZWJ links. Finding analysis keeps that
            // non-selector structure intact: these paths only remove or
            // normalize presentation selectors. The scanner's typed sequence
            // shape determines which presentation rules apply:
            //
            // - `LinksOnly`: only link-attached selectors can be cleaned
            // - single-component `EmojiHeaded`: use the ordinary
            //   singleton/flag rules for that component, and preserve any
            //   trailing ZWJ links in the same scanned item
            // - joined `EmojiHeaded`: use ZWJ fully-qualified component rules,
            //   never standalone policy
            //
            // This is the findings-side implementation of the ZWJ-related
            // sequence contract in
            // `docs/designs/features/sequence-handling.markdown`.
            EmojiSequence::LinksOnly(links) => analyze_links_only_zwj_sequence(item, links),
            EmojiSequence::EmojiHeaded {
                first,
                joined,
                trailing_links,
            } if joined.is_empty() => analyze_single_emoji(item, first, trailing_links, policy),
            EmojiSequence::EmojiHeaded {
                first,
                joined,
                trailing_links,
            } => analyze_multi_emoji_zwj_sequence(item, first, joined, trailing_links),
        },
    }
}

// --- Case: single emoji, either standalone or wrapped in trailing ZWJ links ---

fn analyze_single_emoji<'a>(
    item: &ScanItem<'a>,
    emoji: &EmojiLike,
    trailing_links: &[ZwjLink],
    policy: &Policy,
) -> Option<Finding<'a>> {
    match &emoji.stem {
        EmojiStem::SingletonBase {
            base,
            presentation_selectors_after_base,
        } => {
            let outcome = singleton_analysis_outcome(
                *base,
                presentation_selectors_after_base,
                &emoji.modifiers,
                policy,
            );
            single_emoji_singleton_finding(
                item,
                *base,
                presentation_selectors_after_base,
                &emoji.modifiers,
                trailing_links,
                outcome,
            )
        }
        EmojiStem::Flag {
            first_ri,
            presentation_selectors_after_first_ri,
            second_ri,
            presentation_selectors_after_second_ri,
        } => analyze_single_flag(
            item,
            *first_ri,
            presentation_selectors_after_first_ri,
            *second_ri,
            presentation_selectors_after_second_ri,
            &emoji.modifiers,
            trailing_links,
        ),
    }
}

/// Analyze a regional-indicator flag stem, with or without modifications.
///
/// Flag stems have no base-presentation slot. Regional indicators never carry
/// a sanctioned presentation sequence, so every presentation selector attached
/// to either RI is unsanctioned and removed under rule 1.
///
/// If greedy scanner grouping attaches modifiers, keycaps, or tags after a flag
/// stem, their own trailing presentation selectors are also rule-1 cleanup:
/// preserve the modification content and strip the unsanctioned selectors.
fn analyze_single_flag<'a>(
    item: &ScanItem<'a>,
    first_ri: char,
    presentation_selectors_after_first_ri: &[Presentation],
    second_ri: char,
    presentation_selectors_after_second_ri: &[Presentation],
    modifications: &[EmojiModification],
    trailing_links: &[ZwjLink],
) -> Option<Finding<'a>> {
    let has_ri_presentation_selectors = !presentation_selectors_after_first_ri.is_empty()
        || !presentation_selectors_after_second_ri.is_empty();
    let has_modification_presentation_selectors =
        has_trailing_modification_presentation_selectors(modifications);
    let has_link_presentation_selectors = trailing_links.iter().any(zwj_link_has_selectors);

    if !has_ri_presentation_selectors
        && !has_modification_presentation_selectors
        && !has_link_presentation_selectors
    {
        return None;
    }

    Some(unambiguous_finding(
        item,
        Violation::UnsanctionedSelectorsOnly,
        render_flag_with_links(first_ri, second_ri, modifications, trailing_links),
    ))
}

fn analyze_links_only_zwj_sequence<'a>(
    item: &ScanItem<'a>,
    links: &[ZwjLink],
) -> Option<Finding<'a>> {
    let has_link_presentation_selectors = links.iter().any(zwj_link_has_selectors);
    if !has_link_presentation_selectors {
        return None;
    }

    Some(unambiguous_finding(
        item,
        Violation::UnsanctionedSelectorsOnly,
        render_zwj_links_only_sequence(links),
    ))
}

fn single_emoji_singleton_finding<'a>(
    item: &ScanItem<'a>,
    base: char,
    presentation_selectors_after_base: &[Presentation],
    modifications: &[EmojiModification],
    trailing_links: &[ZwjLink],
    outcome: SingletonAnalysisOutcome,
) -> Option<Finding<'a>> {
    let has_link_presentation_selectors = trailing_links.iter().any(zwj_link_has_selectors);

    match outcome {
        SingletonAnalysisOutcome::Canonical if !has_link_presentation_selectors => None,
        SingletonAnalysisOutcome::Canonical => {
            debug_assert!(
                presentation_selectors_after_base.len() <= 1,
                "canonical singleton base cannot carry multiple presentation selectors"
            );
            Some(unambiguous_finding(
                item,
                Violation::UnsanctionedSelectorsOnly,
                render_singleton_with_links(
                    base,
                    presentation_selectors_after_base.first().copied(),
                    modifications,
                    trailing_links,
                ),
            ))
        }
        SingletonAnalysisOutcome::Repair {
            canonical_presentation,
            violation,
        } => Some(unambiguous_finding(
            item,
            // Link-attached selectors are orthogonal cleanup. They should not
            // hide a primary singleton/keycap violation in the wrapped emoji.
            if has_link_presentation_selectors {
                violation.with_unsanctioned_selectors()
            } else {
                violation
            },
            render_singleton_with_links(
                base,
                canonical_presentation,
                modifications,
                trailing_links,
            ),
        )),
        SingletonAnalysisOutcome::ResolvePresentation { default } if trailing_links.is_empty() => {
            Some(resolve_presentation_finding(
                item,
                base,
                modifications,
                default,
            ))
        }
        SingletonAnalysisOutcome::ResolvePresentation { default } => {
            Some(resolve_zwj_wrapper_presentation_finding(
                item,
                base,
                modifications,
                trailing_links,
                default,
                has_link_presentation_selectors,
            ))
        }
    }
}

fn analyze_multi_emoji_zwj_sequence<'a>(
    item: &ScanItem<'a>,
    first: &EmojiLike,
    joined: &[ZwjJoinedEmoji],
    trailing_links: &[ZwjLink],
) -> Option<Finding<'a>> {
    let has_unsanctioned_selectors = joined
        .iter()
        .any(|joined| zwj_link_has_selectors(&joined.link))
        || trailing_links.iter().any(zwj_link_has_selectors);
    let needs_repair = !zwj_component_is_canonical(first)
        || joined
            .iter()
            .any(|joined| !zwj_component_is_canonical(&joined.emoji))
        || has_unsanctioned_selectors;

    if !needs_repair {
        return None;
    }

    Some(unambiguous_finding(
        item,
        Violation::primary(
            PrimaryViolationKind::NotFullyQualifiedSequence,
            has_unsanctioned_selectors,
        ),
        render_forced_emoji_zwj_sequence(first, joined, trailing_links),
    ))
}

// --- ZWJ sequence analysis helpers ---

fn zwj_link_has_selectors(link: &ZwjLink) -> bool {
    !link.presentation_selectors_after_link.is_empty()
}

fn zwj_component_is_canonical(emoji: &EmojiLike) -> bool {
    match &emoji.stem {
        EmojiStem::SingletonBase {
            base,
            presentation_selectors_after_base,
        } => {
            singleton_base_presentation_is_canonical(
                presentation_selectors_after_base,
                zwj_forced_singleton_presentation(*base, &emoji.modifiers),
            ) && !has_trailing_modification_presentation_selectors(&emoji.modifiers)
        }
        EmojiStem::Flag {
            presentation_selectors_after_first_ri,
            presentation_selectors_after_second_ri,
            ..
        } => {
            presentation_selectors_after_first_ri.is_empty()
                && presentation_selectors_after_second_ri.is_empty()
                && !emoji
                    .modifiers
                    .iter()
                    .any(modification_has_trailing_presentation_selectors)
        }
    }
}

fn zwj_forced_singleton_presentation(
    base: char,
    modifications: &[EmojiModification],
) -> Option<Presentation> {
    // An emoji modifier that immediately follows the base still forces a bare
    // base. Otherwise, multi-component ZWJ context forces emoji presentation
    // for non-emoji-default bases when that presentation selector is sanctioned.
    if matches!(
        modifications.first(),
        Some(EmojiModification::EmojiModifier { .. })
    ) || unicode::is_emoji_default(base)
    {
        None
    } else {
        sanctioned_presentation(base, Presentation::Emoji)
    }
}

/// Render every component according to multi-component ZWJ forced-emoji rules.
fn render_forced_emoji_zwj_sequence(
    first: &EmojiLike,
    joined: &[ZwjJoinedEmoji],
    trailing_links: &[ZwjLink],
) -> String {
    let mut out = String::new();
    zwj_forced_emoji_like(first).render(&mut out);
    for joined in joined {
        out.push(unicode::ZWJ);
        zwj_forced_emoji_like(&joined.emoji).render(&mut out);
    }
    render_zwj_links(&mut out, trailing_links);
    out
}

fn render_zwj_links_only_sequence(links: &[ZwjLink]) -> String {
    let mut out = String::new();
    render_zwj_links(&mut out, links);
    out
}

fn render_zwj_links(out: &mut String, links: &[ZwjLink]) {
    for _ in links {
        out.push(unicode::ZWJ);
    }
}

fn zwj_forced_emoji_like(emoji: &EmojiLike) -> FixedEmojiLike<'_> {
    match &emoji.stem {
        EmojiStem::SingletonBase { base, .. } => FixedEmojiLike::singleton_base(
            *base,
            zwj_forced_singleton_presentation(*base, &emoji.modifiers),
            &emoji.modifiers,
        ),
        EmojiStem::Flag {
            first_ri,
            second_ri,
            ..
        } => FixedEmojiLike::flag(*first_ri, *second_ri, &emoji.modifiers),
    }
}

// --- Singleton analysis planning ---

#[derive(Clone, Copy)]
enum SingletonAnalysisOutcome {
    /// No finding is needed.
    Canonical,
    /// A deterministic repair is available.
    ///
    /// `canonical_presentation` is only the selector state for the base slot;
    /// rendering also preserves every modification after stripping their
    /// trailing presentation selectors.
    Repair {
        canonical_presentation: Option<Presentation>,
        violation: Violation,
    },
    /// A bare standalone slot remains policy-ambiguous and needs an explicit
    /// text/emoji choice from the caller.
    ResolvePresentation { default: Presentation },
}

/// Decide how a singleton-base emoji-like unit should be analyzed.
///
/// The fixed-cleanup precedence lives in
/// `docs/designs/features/sequence-handling.markdown`. Rule 5 is the only
/// path where policy may be consulted.
fn singleton_analysis_outcome(
    base: char,
    presentation_selectors_after_base: &[Presentation],
    modifications: &[EmojiModification],
    policy: &Policy,
) -> SingletonAnalysisOutcome {
    match modifications.first() {
        // Precedence 2: emoji modifiers force a bare base. Legacy FE0F before an
        // emoji modifier is removed.
        Some(EmojiModification::EmojiModifier { .. }) => {
            singleton_fixed_cleanup_outcome(presentation_selectors_after_base, modifications, None)
        }
        // Precedence 3: tag context forces emoji presentation for
        // non-emoji-default bases, subject to precedence 1's
        // sanctioned-presentation guard below.
        Some(EmojiModification::TagModifier(_)) => singleton_fixed_cleanup_outcome(
            presentation_selectors_after_base,
            modifications,
            if unicode::is_emoji_default(base) {
                None
            } else {
                sanctioned_presentation(base, Presentation::Emoji)
            },
        ),
        // Precedence 5: no fixed-cleanup context remains, so policy chooses the
        // canonical presentation. The policy analysis still rejects selector
        // choices unsupported by the base's variation-sequence data.
        first_modification => standalone_singleton_analysis_outcome(
            base,
            presentation_selectors_after_base,
            unicode::has_variation_sequence(base),
            matches!(
                first_modification,
                Some(EmojiModification::EnclosingKeycap { .. })
            ),
            policy,
        ),
    }
}

fn sanctioned_presentation(base: char, presentation: Presentation) -> Option<Presentation> {
    unicode::has_variation_sequence(base).then_some(presentation)
}

fn singleton_fixed_cleanup_outcome(
    presentation_selectors_after_base: &[Presentation],
    modifications: &[EmojiModification],
    canonical_presentation: Option<Presentation>,
) -> SingletonAnalysisOutcome {
    // Fixed cleanup is canonical only when both the base slot is already in the
    // selected presentation state and every modification is free of trailing
    // presentation selectors.
    if singleton_base_presentation_is_canonical(
        presentation_selectors_after_base,
        canonical_presentation,
    ) && !has_trailing_modification_presentation_selectors(modifications)
    {
        return SingletonAnalysisOutcome::Canonical;
    }

    SingletonAnalysisOutcome::Repair {
        canonical_presentation,
        violation: Violation::UnsanctionedSelectorsOnly,
    }
}

/// Whether the base presentation selectors already match the selected
/// canonical base presentation.
///
/// This intentionally ignores trailing selectors after modifications; those
/// are checked separately. Keeping the two checks separate avoids using a
/// rendered string as a proxy for canonicality.
fn singleton_base_presentation_is_canonical(
    presentation_selectors_after_base: &[Presentation],
    presentation: Option<Presentation>,
) -> bool {
    match presentation {
        None => presentation_selectors_after_base.is_empty(),
        Some(presentation) => presentation_selectors_after_base == [presentation],
    }
}

fn standalone_singleton_analysis_outcome(
    base: char,
    presentation_selectors_after_base: &[Presentation],
    has_sanctioned_presentation: bool,
    is_keycap: bool,
    policy: &Policy,
) -> SingletonAnalysisOutcome {
    // This function is entered only after fixed-cleanup rules have been
    // exhausted. It therefore handles a true standalone base slot: no
    // modifier, tag, or missing-variation keycap context can force a
    // presentation before policy.
    if !has_sanctioned_presentation {
        // Base has no sanctioned variation sequence data. Any attached
        // presentation selector is unsanctioned and gets removed; absence of
        // presentation selectors means there is nothing to repair.
        return if presentation_selectors_after_base.is_empty() {
            SingletonAnalysisOutcome::Canonical
        } else {
            SingletonAnalysisOutcome::Repair {
                canonical_presentation: None,
                violation: Violation::UnsanctionedSelectorsOnly,
            }
        };
    }

    match (
        policy.singleton_rule(base, is_keycap),
        presentation_selectors_after_base,
    ) {
        // More than one presentation selector where the first matches the
        // bare-side of the rule: the primary violation is the redundant first
        // selector, and the extras are unsanctioned selector cleanup.
        (SingletonRule::TextToBare, &[Presentation::Text, _, ..])
        | (SingletonRule::EmojiToBare, &[Presentation::Emoji, _, ..]) => {
            SingletonAnalysisOutcome::Repair {
                canonical_presentation: None,
                violation: Violation::primary(PrimaryViolationKind::RedundantSelector, true),
            }
        }
        // More than one presentation selector but the first is meaningful
        // under this rule: the first selector is canonical, so the only
        // violation is the unsanctioned selector cleanup after it.
        (_, &[current_presentation, _, ..]) => SingletonAnalysisOutcome::Repair {
            canonical_presentation: Some(current_presentation),
            violation: Violation::UnsanctionedSelectorsOnly,
        },
        // Bare stem under a rule that resolves bare to a concrete presentation:
        // let the caller decide between text and emoji.
        (SingletonRule::BareToEmoji, &[]) => SingletonAnalysisOutcome::ResolvePresentation {
            default: Presentation::Emoji,
        },
        (SingletonRule::BareToText, &[]) => SingletonAnalysisOutcome::ResolvePresentation {
            default: Presentation::Text,
        },
        // Exactly one presentation selector that matches the bare-side of the
        // rule: the presentation selector is redundant, drop it.
        (SingletonRule::TextToBare, &[Presentation::Text])
        | (SingletonRule::EmojiToBare, &[Presentation::Emoji]) => {
            SingletonAnalysisOutcome::Repair {
                canonical_presentation: None,
                violation: Violation::primary(PrimaryViolationKind::RedundantSelector, false),
            }
        }
        // All remaining single-presentation-selector and no-presentation-selector
        // cases are already canonical under the active rule.
        (SingletonRule::TextToBare, &[Presentation::Emoji] | &[])
        | (SingletonRule::EmojiToBare, &[Presentation::Text] | &[])
        | (SingletonRule::BareToText | SingletonRule::BareToEmoji, &[_]) => {
            SingletonAnalysisOutcome::Canonical
        }
    }
}

// --- Shared selector predicates ---

fn modification_has_trailing_presentation_selectors(m: &EmojiModification) -> bool {
    match m {
        EmojiModification::EmojiModifier {
            presentation_selectors_after_modifier,
            ..
        } => !presentation_selectors_after_modifier.is_empty(),
        EmojiModification::EnclosingKeycap {
            presentation_selectors_after_keycap,
        } => !presentation_selectors_after_keycap.is_empty(),
        EmojiModification::TagModifier(runs) => runs
            .iter()
            .any(|r| !r.presentation_selectors_after_tag.is_empty()),
    }
}

fn has_trailing_modification_presentation_selectors(modifications: &[EmojiModification]) -> bool {
    modifications
        .iter()
        .any(modification_has_trailing_presentation_selectors)
}

// --- Finding builders ---

fn unambiguous_finding<'a>(
    item: &ScanItem<'a>,
    violation: Violation,
    fix_replacement: String,
) -> Finding<'a> {
    Finding {
        span: item.span.clone(),
        raw: item.raw,
        replacement_plan: ReplacementPlan::Repair {
            violation,
            fix_replacement,
        },
    }
}

fn resolve_presentation_finding<'a>(
    item: &ScanItem<'a>,
    base: char,
    modifications: &[EmojiModification],
    default: Presentation,
) -> Finding<'a> {
    Finding {
        span: item.span.clone(),
        raw: item.raw,
        replacement_plan: ReplacementPlan::ResolvePresentation {
            violation: Violation::primary(PrimaryViolationKind::BareNeedsResolution, false),
            default,
            text_replacement: render_singleton(base, Some(Presentation::Text), modifications),
            emoji_replacement: render_singleton(base, Some(Presentation::Emoji), modifications),
        },
    }
}

fn resolve_zwj_wrapper_presentation_finding<'a>(
    item: &ScanItem<'a>,
    base: char,
    modifications: &[EmojiModification],
    trailing_links: &[ZwjLink],
    default: Presentation,
    has_unsanctioned_selectors: bool,
) -> Finding<'a> {
    debug_assert!(
        !trailing_links.is_empty(),
        "ZWJ wrapper presentation resolution requires at least one trailing link"
    );
    Finding {
        span: item.span.clone(),
        raw: item.raw,
        replacement_plan: ReplacementPlan::ResolvePresentation {
            violation: Violation::primary(
                PrimaryViolationKind::BareNeedsResolution,
                has_unsanctioned_selectors,
            ),
            default,
            text_replacement: render_singleton_with_links(
                base,
                Some(Presentation::Text),
                modifications,
                trailing_links,
            ),
            emoji_replacement: render_singleton_with_links(
                base,
                Some(Presentation::Emoji),
                modifications,
                trailing_links,
            ),
        },
    }
}

// --- Fixed emoji-like rendering ---

fn render_singleton(
    base: char,
    presentation: Option<Presentation>,
    modifications: &[EmojiModification],
) -> String {
    FixedEmojiLike::singleton_base(base, presentation, modifications).render_to_string()
}

fn render_singleton_with_links(
    base: char,
    presentation: Option<Presentation>,
    modifications: &[EmojiModification],
    trailing_links: &[ZwjLink],
) -> String {
    let mut out =
        FixedEmojiLike::singleton_base(base, presentation, modifications).render_to_string();
    render_zwj_links(&mut out, trailing_links);
    out
}

fn render_flag_with_links(
    first_ri: char,
    second_ri: char,
    modifications: &[EmojiModification],
    trailing_links: &[ZwjLink],
) -> String {
    let mut out = FixedEmojiLike::flag(first_ri, second_ri, modifications).render_to_string();
    render_zwj_links(&mut out, trailing_links);
    out
}
