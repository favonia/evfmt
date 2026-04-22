//! Policy-aware findings for scanned emoji variation structures.
//!
//! This module implements the policy and fixed-rule analysis steps from the
//! conceptual formatting algorithm. It produces findings with valid replacement
//! decision slots and a render plan, so callers can choose a replacement
//! without re-reading policy for the same item.
//! Interactive callers normally use this module directly: [`analyze_scan_item`]
//! computes reasonableness, applies [`Policy`] only where policy is relevant,
//! and stores the valid replacement plan in each [`Finding`].
//!
//! - [`crate::scanner`] decides structural item boundaries
//! - [`analyze_scan_item`] turns policy-neutral reasonableness into findings and replacement slots

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
    /// Wrong or missing presentation selectors in a keycap or ZWJ-related sequence.
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

/// One selector-bearing presentation choice in a replacement decision vector.
///
/// A complete replacement decision is a slice of these choices, one per
/// [`DecisionSlot`] reported by a finding. Fixed repairs have no presentation
/// slots, so their complete decision vector is empty. In particular, choosing
/// bare form is represented as a fixed repair rather than as a public decision
/// slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReplacementDecision {
    /// Resolve this slot as text presentation.
    Text,
    /// Resolve this slot as emoji presentation.
    Emoji,
}

impl ReplacementDecision {
    const fn from_presentation(presentation: Presentation) -> Self {
        match presentation {
            Presentation::Text => Self::Text,
            Presentation::Emoji => Self::Emoji,
        }
    }
}

/// One presentation slot in a finding's replacement decision vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionSlot {
    choices: Vec<ReplacementDecision>,
    default: ReplacementDecision,
}

impl DecisionSlot {
    /// Valid choices for this slot.
    #[must_use]
    pub fn choices(&self) -> &[ReplacementDecision] {
        &self.choices
    }

    /// The choice batch formatting applies to this slot by default.
    #[must_use]
    pub const fn default_decision(&self) -> ReplacementDecision {
        self.default
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotReplacement {
    decision: ReplacementDecision,
    replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplacementSlotPlan {
    public: DecisionSlot,
    replacements: Vec<SlotReplacement>,
}

impl ReplacementSlotPlan {
    fn replacement(&self, decision: ReplacementDecision) -> Option<&str> {
        self.replacements
            .iter()
            .find(|replacement| replacement.decision == decision)
            .map(|replacement| replacement.replacement.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplacementPiece {
    Literal(String),
    Slot(usize),
}

/// Valid replacement decision vector and render plan for a finding.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplacementPlan {
    /// Why the item is non-canonical.
    violation: Violation,
    /// Public presentation slots in this finding's decision vector.
    decision_slots: Vec<DecisionSlot>,
    /// Presentation slots callers must decide to select a non-default
    /// replacement. Empty means the finding is a fixed repair.
    slots: Vec<ReplacementSlotPlan>,
    /// Render plan for this finding's replacement. Slot references index into
    /// `slots`; literal pieces already include fixed cleanup.
    pieces: Vec<ReplacementPiece>,
    /// The replacement batch formatting applies by default.
    default_replacement: String,
}

impl ReplacementPlan {
    /// Why the analyzed item is non-canonical.
    #[must_use]
    const fn violation(&self) -> Violation {
        self.violation
    }

    /// Presentation slots in this finding's replacement decision vector.
    #[must_use]
    fn decision_slots(&self) -> &[DecisionSlot] {
        &self.decision_slots
    }

    /// The replacement decision batch formatting applies by default.
    #[must_use]
    fn default_decision(&self) -> Vec<ReplacementDecision> {
        self.slots
            .iter()
            .map(|slot| slot.public.default_decision())
            .collect()
    }

    /// The replacement batch formatting applies by default.
    #[must_use]
    fn default_replacement(&self) -> &str {
        &self.default_replacement
    }

    fn render_replacement(&self, decisions: &[ReplacementDecision]) -> Option<String> {
        if decisions.len() != self.slots.len() {
            return None;
        }

        let mut out = String::new();
        for piece in &self.pieces {
            match piece {
                ReplacementPiece::Literal(text) => out.push_str(text),
                ReplacementPiece::Slot(slot_index) => {
                    let decision = decisions.get(*slot_index).copied()?;
                    out.push_str(self.slots.get(*slot_index)?.replacement(decision)?);
                }
            }
        }
        Some(out)
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

    /// Presentation slots in this finding's replacement decision vector.
    ///
    /// A fixed repair has no presentation slots. Call [`Finding::replacement`]
    /// with an empty decision slice to apply that repair.
    #[must_use]
    pub fn decision_slots(&self) -> &[DecisionSlot] {
        self.replacement_plan.decision_slots()
    }

    /// The replacement decision batch formatting applies by default.
    #[must_use]
    pub fn default_decision(&self) -> Vec<ReplacementDecision> {
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
    /// finding.replacement(&finding.default_decision()).unwrap();
    /// ```
    ///
    /// The default decision vector is always valid for this finding.
    #[must_use]
    pub fn default_replacement(&self) -> &str {
        self.replacement_plan.default_replacement()
    }

    /// Return the replacement text for a valid replacement decision vector.
    ///
    /// The decision slice must contain exactly one choice for each
    /// [`DecisionSlot`] returned by [`Finding::decision_slots`]. Fixed repairs
    /// have no slots and therefore use an empty decision slice.
    ///
    /// Returns `None` when the decision vector has the wrong length or contains
    /// a choice that is not valid for its slot.
    /// Callers that want to skip a finding can keep [`Finding::raw`].
    #[must_use]
    pub fn replacement(&self, decision: &[ReplacementDecision]) -> Option<String> {
        self.replacement_plan.render_replacement(decision)
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
            // - joined `EmojiHeaded`: preserve ZWJ links but resolve each
            //   component with the same component-local policy/fixed cleanup
            //   it would use outside the surrounding ZWJ links
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
            } => analyze_multi_emoji_zwj_sequence(item, first, joined, trailing_links, policy),
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
    policy: &Policy,
) -> Option<Finding<'a>> {
    let mut builder = ReplacementPlanBuilder::new();
    let mut has_unsanctioned_selectors = false;
    let mut has_noncanonical_component = false;
    let mut has_resolution_slot = false;

    let first_outcome = zwj_component_outcome(first, policy);
    has_unsanctioned_selectors |= first_outcome.has_unsanctioned_selectors;
    has_noncanonical_component |= first_outcome.is_noncanonical;
    has_resolution_slot |= first_outcome.has_resolution_slot;
    builder.extend(first_outcome.pieces, first_outcome.slots);

    for joined in joined {
        if zwj_link_has_selectors(&joined.link) {
            has_unsanctioned_selectors = true;
        }
        builder.push_literal(unicode::ZWJ.to_string());

        let joined_outcome = zwj_component_outcome(&joined.emoji, policy);
        has_unsanctioned_selectors |= joined_outcome.has_unsanctioned_selectors;
        has_noncanonical_component |= joined_outcome.is_noncanonical;
        has_resolution_slot |= joined_outcome.has_resolution_slot;
        builder.extend(joined_outcome.pieces, joined_outcome.slots);
    }

    if trailing_links.iter().any(zwj_link_has_selectors) {
        has_unsanctioned_selectors = true;
    }
    for _ in trailing_links {
        builder.push_literal(unicode::ZWJ.to_string());
    }

    if !has_noncanonical_component && !has_unsanctioned_selectors {
        return None;
    }

    let violation = if has_resolution_slot {
        Violation::primary(
            PrimaryViolationKind::BareNeedsResolution,
            has_unsanctioned_selectors,
        )
    } else if has_noncanonical_component {
        Violation::primary(
            PrimaryViolationKind::NotFullyQualifiedSequence,
            has_unsanctioned_selectors,
        )
    } else {
        Violation::UnsanctionedSelectorsOnly
    };

    Some(finding_from_builder(item, violation, builder))
}

// --- ZWJ sequence analysis helpers ---

fn zwj_link_has_selectors(link: &ZwjLink) -> bool {
    !link.presentation_selectors_after_link.is_empty()
}

struct ZwjComponentOutcome {
    pieces: Vec<ComponentReplacementPiece>,
    slots: Vec<ReplacementSlotPlan>,
    is_noncanonical: bool,
    has_unsanctioned_selectors: bool,
    has_resolution_slot: bool,
}

enum ComponentReplacementPiece {
    Literal(String),
    Slot(usize),
}

fn zwj_component_outcome(emoji: &EmojiLike, policy: &Policy) -> ZwjComponentOutcome {
    match &emoji.stem {
        EmojiStem::SingletonBase {
            base,
            presentation_selectors_after_base,
        } => zwj_singleton_component_outcome(
            *base,
            presentation_selectors_after_base,
            &emoji.modifiers,
            policy,
        ),
        EmojiStem::Flag {
            first_ri,
            presentation_selectors_after_first_ri,
            second_ri,
            presentation_selectors_after_second_ri,
        } => {
            let has_unsanctioned_selectors = !presentation_selectors_after_first_ri.is_empty()
                || !presentation_selectors_after_second_ri.is_empty()
                || has_trailing_modification_presentation_selectors(&emoji.modifiers);
            ZwjComponentOutcome {
                pieces: vec![ComponentReplacementPiece::Literal(
                    FixedEmojiLike::flag(*first_ri, *second_ri, &emoji.modifiers)
                        .render_to_string(),
                )],
                slots: vec![],
                is_noncanonical: has_unsanctioned_selectors,
                has_unsanctioned_selectors,
                has_resolution_slot: false,
            }
        }
    }
}

fn zwj_singleton_component_outcome(
    base: char,
    presentation_selectors_after_base: &[Presentation],
    modifications: &[EmojiModification],
    policy: &Policy,
) -> ZwjComponentOutcome {
    match singleton_analysis_outcome(
        base,
        presentation_selectors_after_base,
        modifications,
        policy,
    ) {
        SingletonAnalysisOutcome::Canonical => {
            let presentation = presentation_selectors_after_base.first().copied();
            ZwjComponentOutcome {
                pieces: vec![ComponentReplacementPiece::Literal(render_singleton(
                    base,
                    presentation,
                    modifications,
                ))],
                slots: vec![],
                is_noncanonical: false,
                has_unsanctioned_selectors: false,
                has_resolution_slot: false,
            }
        }
        SingletonAnalysisOutcome::Repair {
            canonical_presentation,
            violation,
        } => ZwjComponentOutcome {
            pieces: vec![ComponentReplacementPiece::Literal(render_singleton(
                base,
                canonical_presentation,
                modifications,
            ))],
            slots: vec![],
            is_noncanonical: true,
            has_unsanctioned_selectors: matches!(violation, Violation::UnsanctionedSelectorsOnly)
                || matches!(
                    violation,
                    Violation::Primary(PrimaryViolation {
                        has_unsanctioned_selectors: true,
                        ..
                    })
                ),
            has_resolution_slot: false,
        },
        SingletonAnalysisOutcome::ResolvePresentation { default } => {
            let slot = presentation_slot(
                ReplacementDecision::from_presentation(default),
                [
                    (
                        ReplacementDecision::Text,
                        render_singleton(base, Some(Presentation::Text), modifications),
                    ),
                    (
                        ReplacementDecision::Emoji,
                        render_singleton(base, Some(Presentation::Emoji), modifications),
                    ),
                ],
            );
            ZwjComponentOutcome {
                pieces: vec![ComponentReplacementPiece::Slot(0)],
                slots: vec![slot],
                is_noncanonical: true,
                has_unsanctioned_selectors: false,
                has_resolution_slot: true,
            }
        }
    }
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
    presentation_selectors_after_base == presentation.as_slice()
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
    let mut builder = ReplacementPlanBuilder::new();
    builder.push_literal(fix_replacement);
    finding_from_builder(item, violation, builder)
}

fn resolve_presentation_finding<'a>(
    item: &ScanItem<'a>,
    base: char,
    modifications: &[EmojiModification],
    default: Presentation,
) -> Finding<'a> {
    let mut builder = ReplacementPlanBuilder::new();
    builder.push_slot(presentation_slot(
        ReplacementDecision::from_presentation(default),
        [
            (
                ReplacementDecision::Text,
                render_singleton(base, Some(Presentation::Text), modifications),
            ),
            (
                ReplacementDecision::Emoji,
                render_singleton(base, Some(Presentation::Emoji), modifications),
            ),
        ],
    ));
    finding_from_builder(
        item,
        Violation::primary(PrimaryViolationKind::BareNeedsResolution, false),
        builder,
    )
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
    let mut builder = ReplacementPlanBuilder::new();
    builder.push_slot(presentation_slot(
        ReplacementDecision::from_presentation(default),
        [
            (
                ReplacementDecision::Text,
                render_singleton_with_links(
                    base,
                    Some(Presentation::Text),
                    modifications,
                    trailing_links,
                ),
            ),
            (
                ReplacementDecision::Emoji,
                render_singleton_with_links(
                    base,
                    Some(Presentation::Emoji),
                    modifications,
                    trailing_links,
                ),
            ),
        ],
    ));
    finding_from_builder(
        item,
        Violation::primary(
            PrimaryViolationKind::BareNeedsResolution,
            has_unsanctioned_selectors,
        ),
        builder,
    )
}

fn presentation_slot<const N: usize>(
    default: ReplacementDecision,
    replacements: [(ReplacementDecision, String); N],
) -> ReplacementSlotPlan {
    let replacements: Vec<_> = replacements
        .into_iter()
        .map(|(decision, replacement)| SlotReplacement {
            decision,
            replacement,
        })
        .collect();
    let choices = replacements
        .iter()
        .map(|replacement| replacement.decision)
        .collect();
    ReplacementSlotPlan {
        public: DecisionSlot { choices, default },
        replacements,
    }
}

struct ReplacementPlanBuilder {
    slots: Vec<ReplacementSlotPlan>,
    pieces: Vec<ReplacementPiece>,
}

impl ReplacementPlanBuilder {
    const fn new() -> Self {
        Self {
            slots: Vec::new(),
            pieces: Vec::new(),
        }
    }

    fn push_literal(&mut self, literal: String) {
        self.pieces.push(ReplacementPiece::Literal(literal));
    }

    fn push_slot(&mut self, slot: ReplacementSlotPlan) {
        let slot_index = self.slots.len();
        self.slots.push(slot);
        self.pieces.push(ReplacementPiece::Slot(slot_index));
    }

    fn extend(&mut self, pieces: Vec<ComponentReplacementPiece>, slots: Vec<ReplacementSlotPlan>) {
        let slot_offset = self.slots.len();
        self.slots.extend(slots);
        self.pieces
            .extend(pieces.into_iter().map(|piece| match piece {
                ComponentReplacementPiece::Literal(literal) => ReplacementPiece::Literal(literal),
                ComponentReplacementPiece::Slot(slot_index) => {
                    ReplacementPiece::Slot(slot_offset + slot_index)
                }
            }));
    }

    fn build(self, violation: Violation) -> ReplacementPlan {
        let default_replacement = render_default_replacement(&self.slots, &self.pieces);
        let plan = ReplacementPlan {
            violation,
            decision_slots: self.slots.iter().map(|slot| slot.public.clone()).collect(),
            slots: self.slots,
            pieces: self.pieces,
            default_replacement,
        };
        debug_assert_eq!(
            plan.render_replacement(&plan.default_decision()),
            Some(plan.default_replacement.clone())
        );
        plan
    }
}

fn render_default_replacement(
    slots: &[ReplacementSlotPlan],
    pieces: &[ReplacementPiece],
) -> String {
    let mut out = String::new();
    for piece in pieces {
        match piece {
            ReplacementPiece::Literal(text) => out.push_str(text),
            ReplacementPiece::Slot(slot_index) => {
                if let Some(slot) = slots.get(*slot_index)
                    && let Some(replacement) = slot.replacement(slot.public.default_decision())
                {
                    out.push_str(replacement);
                }
            }
        }
    }
    out
}

fn finding_from_builder<'a>(
    item: &ScanItem<'a>,
    violation: Violation,
    builder: ReplacementPlanBuilder,
) -> Finding<'a> {
    Finding {
        span: item.span.clone(),
        raw: item.raw,
        replacement_plan: builder.build(violation),
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
