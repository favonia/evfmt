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
//!
//! Use this module when callers need to inspect or override repairs
//! item-by-item; otherwise [`crate::format_text`] is the shorter path.
//!
//! # Examples
//!
//! ```rust
//! use evfmt::{Policy, scan};
//! use evfmt::findings::analyze_scan_item;
//!
//! let policy = Policy::default();
//! let input = "A\u{FE0F}\u{00A9}";
//!
//! let repaired = scan(input)
//!     .map(|item| {
//!         analyze_scan_item(&item, &policy).map_or_else(
//!             || item.raw.to_owned(),
//!             |finding| finding.default_replacement().to_owned(),
//!         )
//!     })
//!     .collect::<String>();
//!
//! assert_eq!(repaired, "A\u{00A9}\u{FE0F}");
//! ```

use std::ops::{Add, AddAssign, Range};

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

// --- Public non-canonicality / decision types ---

/// Count summary for why a scanned item is non-canonical.
///
/// These axes are compositional rather than mutually exclusive. A finding may
/// simultaneously include selector cleanup, deterministic sequence defects,
/// redundant selectors, and policy-driven bare-base resolution.
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, scan};
/// use evfmt::findings::{NonCanonicality, analyze_scan_item};
///
/// let policy = Policy::default();
/// let finding = scan("A\u{FE0F}")
///     .find_map(|item| analyze_scan_item(&item, &policy))
///     .unwrap();
///
/// assert_eq!(
///     finding.non_canonicality(),
///     NonCanonicality::new(1, 0, 0, 0)
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NonCanonicality {
    /// Count of presentation selectors removed as unsanctioned cleanup.
    pub unsanctioned_selectors: usize,
    /// Count of deterministic sequence defects that need repair but do not
    /// expose a policy choice.
    pub defective_sequences: usize,
    /// Count of sanctioned selectors dropped because the active policy prefers
    /// bare form.
    pub redundant_selectors: usize,
    /// Count of bare base slots that the active policy asks callers to resolve.
    pub bases_to_resolve: usize,
}

impl Default for NonCanonicality {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

impl NonCanonicality {
    const DEFECTIVE: Self = Self::new(0, 1, 0, 0);
    const REDUNDANT: Self = Self::new(0, 0, 1, 0);
    const RESOLVE: Self = Self::new(0, 0, 0, 1);

    /// Create an explicit non-canonicality summary.
    #[must_use]
    pub const fn new(
        unsanctioned_selectors: usize,
        defective_sequences: usize,
        redundant_selectors: usize,
        bases_to_resolve: usize,
    ) -> Self {
        Self {
            unsanctioned_selectors,
            defective_sequences,
            redundant_selectors,
            bases_to_resolve,
        }
    }

    const fn unsanctioned(count: usize) -> Self {
        Self::new(count, 0, 0, 0)
    }

    const fn is_empty(self) -> bool {
        self.unsanctioned_selectors == 0
            && self.defective_sequences == 0
            && self.redundant_selectors == 0
            && self.bases_to_resolve == 0
    }
}

impl Add for NonCanonicality {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            unsanctioned_selectors: self.unsanctioned_selectors + rhs.unsanctioned_selectors,
            defective_sequences: self.defective_sequences + rhs.defective_sequences,
            redundant_selectors: self.redundant_selectors + rhs.redundant_selectors,
            bases_to_resolve: self.bases_to_resolve + rhs.bases_to_resolve,
        }
    }
}

impl AddAssign for NonCanonicality {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

/// One selector-bearing presentation choice in a replacement decision vector.
///
/// A complete replacement decision is a slice of these choices, one per
/// [`DecisionSlot`] reported by a finding. Fixed repairs have no presentation
/// slots, so their complete decision vector is empty. In particular, choosing
/// bare form is represented as a fixed repair rather than as a public decision
/// slot.
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, scan};
/// use evfmt::findings::{ReplacementDecision, analyze_scan_item};
///
/// let policy = Policy::default();
/// let finding = scan("\u{00A9}")
///     .find_map(|item| analyze_scan_item(&item, &policy))
///     .unwrap();
///
/// assert_eq!(
///     finding.replacement(&[ReplacementDecision::Text]).unwrap(),
///     "\u{00A9}\u{FE0E}"
/// );
/// assert_eq!(
///     finding.replacement(&[ReplacementDecision::Emoji]).unwrap(),
///     "\u{00A9}\u{FE0F}"
/// );
/// ```
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
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, scan};
/// use evfmt::findings::{ReplacementDecision, analyze_scan_item};
///
/// let policy = Policy::default();
/// let finding = scan("\u{00A9}")
///     .find_map(|item| analyze_scan_item(&item, &policy))
///     .unwrap();
///
/// let slot = &finding.decision_slots()[0];
/// assert_eq!(slot.choices(), &[ReplacementDecision::Text, ReplacementDecision::Emoji]);
/// assert_eq!(slot.default_decision(), ReplacementDecision::Emoji);
/// ```
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
    non_canonicality: NonCanonicality,
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
    const fn non_canonicality(&self) -> NonCanonicality {
        self.non_canonicality
    }

    /// Presentation slots in this finding's replacement decision vector.
    #[must_use]
    fn decision_slots(&self) -> &[DecisionSlot] {
        &self.decision_slots
    }

    /// The replacement decision batch formatting applies by default.
    #[must_use]
    fn default_decisions(&self) -> Vec<ReplacementDecision> {
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
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, scan};
/// use evfmt::findings::{ReplacementDecision, analyze_scan_item};
///
/// let policy = Policy::default();
/// let finding = scan("\u{00A9}")
///     .find_map(|item| analyze_scan_item(&item, &policy))
///     .unwrap();
///
/// assert_eq!(finding.raw, "\u{00A9}");
/// assert_eq!(finding.default_replacement(), "\u{00A9}\u{FE0F}");
/// assert_eq!(
///     finding.replacement(&[ReplacementDecision::Text]).unwrap(),
///     "\u{00A9}\u{FE0E}"
/// );
/// ```
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
    pub const fn non_canonicality(&self) -> NonCanonicality {
        self.replacement_plan.non_canonicality()
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
    pub fn default_decisions(&self) -> Vec<ReplacementDecision> {
        self.replacement_plan.default_decisions()
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
    /// finding.replacement(&finding.default_decisions()).unwrap();
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
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, scan};
/// use evfmt::findings::{NonCanonicality, analyze_scan_item};
///
/// let policy = Policy::default();
///
/// assert!(scan("plain text")
///     .all(|item| analyze_scan_item(&item, &policy).is_none()));
///
/// let selector_item = scan("\u{FE0F}").next().unwrap();
/// let finding = analyze_scan_item(&selector_item, &policy).unwrap();
/// assert_eq!(
///     finding.non_canonicality(),
///     NonCanonicality::new(1, 0, 0, 0)
/// );
/// assert_eq!(finding.replacement(&[]).unwrap(), "");
/// ```
#[must_use]
pub fn analyze_scan_item<'a>(item: &ScanItem<'a>, policy: &Policy) -> Option<Finding<'a>> {
    match &item.kind {
        ScanKind::Passthrough => None,
        ScanKind::UnsanctionedPresentationSelectors(_) => Some(unambiguous_finding(
            item,
            NonCanonicality::unsanctioned(count_presentation_selectors_in_item(item)),
            String::new(),
        )),
        ScanKind::EmojiSequence(sequence) => match sequence {
            // The scanner preserves malformed ZWJ-like shapes such as leading,
            // consecutive, or trailing ZWJ links. Finding analysis keeps that
            // non-selector structure intact: these paths only remove or
            // normalize presentation selectors.
            //
            // `LinksOnly` contributes only link cleanup. `EmojiHeaded` uses
            // the same accumulation path regardless of whether the scanner
            // found one component or a joined chain: analyze each component
            // with the same component-local policy/fixed cleanup it would use
            // outside surrounding ZWJ links, then stitch the literal ZWJ links
            // back into the replacement plan in source order.
            //
            // This is the findings-side implementation of the ZWJ-related
            // sequence contract in
            // `docs/designs/features/sequence-handling.markdown`.
            EmojiSequence::LinksOnly(links) => analyze_links_only_zwj_sequence(item, links),
            EmojiSequence::EmojiHeaded {
                first,
                joined,
                trailing_links,
            } => analyze_emoji_headed_sequence(item, first, joined, trailing_links, policy),
        },
    }
}

fn analyze_emoji_headed_sequence<'a>(
    item: &ScanItem<'a>,
    first: &EmojiLike,
    joined: &[ZwjJoinedEmoji],
    trailing_links: &[ZwjLink],
    policy: &Policy,
) -> Option<Finding<'a>> {
    // Emoji-headed sequences now use one accumulation path regardless of
    // whether the scanner found only one component or a full joined chain.
    let mut builder = FindingBuilder::new();
    builder.extend_component(component_outcome(first, policy));

    for joined in joined {
        builder.extend_link(&joined.link);
        builder.extend_component(component_outcome(&joined.emoji, policy));
    }

    for link in trailing_links {
        builder.extend_link(link);
    }

    builder.build(item)
}

// --- ZWJ sequence analysis helpers ---

fn analyze_links_only_zwj_sequence<'a>(
    item: &ScanItem<'a>,
    links: &[ZwjLink],
) -> Option<Finding<'a>> {
    // Links-only sequences participate in the same builder flow as
    // emoji-headed sequences: they simply contribute link cleanup and literal
    // ZWJ output, but no emoji components.
    let mut builder = FindingBuilder::new();
    for link in links {
        builder.extend_link(link);
    }
    builder.build(item)
}

/// Outcome for one emoji component before any surrounding ZWJ links are
/// stitched back in.
///
/// This keeps component-local repair decisions compositional: callers can
/// accumulate component outcomes and link cleanup in source order without
/// re-analyzing policy.
struct ComponentOutcome {
    pieces: Vec<ComponentReplacementPiece>,
    slots: Vec<ReplacementSlotPlan>,
    non_canonicality: NonCanonicality,
}

enum ComponentReplacementPiece {
    Literal(String),
    Slot(usize),
}

fn component_outcome(emoji: &EmojiLike, policy: &Policy) -> ComponentOutcome {
    match &emoji.stem {
        EmojiStem::SingletonBase {
            base,
            presentation_selectors_after_base,
        } => singleton_component_outcome(
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
        } => ComponentOutcome {
            pieces: vec![ComponentReplacementPiece::Literal(
                FixedEmojiLike::flag(*first_ri, *second_ri, &emoji.modifiers).render_to_string(),
            )],
            slots: vec![],
            non_canonicality: NonCanonicality::unsanctioned(
                presentation_selectors_after_first_ri.len()
                    + presentation_selectors_after_second_ri.len()
                    + count_trailing_modification_presentation_selectors(&emoji.modifiers),
            ),
        },
    }
}

fn singleton_component_outcome(
    base: char,
    presentation_selectors_after_base: &[Presentation],
    modifications: &[EmojiModification],
    policy: &Policy,
) -> ComponentOutcome {
    match singleton_analysis_outcome(
        base,
        presentation_selectors_after_base,
        modifications,
        policy,
    ) {
        SingletonAnalysisOutcome::Canonical => {
            let presentation = presentation_selectors_after_base.first().copied();
            ComponentOutcome {
                pieces: vec![ComponentReplacementPiece::Literal(render_singleton(
                    base,
                    presentation,
                    modifications,
                ))],
                slots: vec![],
                non_canonicality: NonCanonicality::default(),
            }
        }
        SingletonAnalysisOutcome::Repair {
            canonical_presentation,
            non_canonicality,
        } => ComponentOutcome {
            pieces: vec![ComponentReplacementPiece::Literal(render_singleton(
                base,
                canonical_presentation,
                modifications,
            ))],
            slots: vec![],
            non_canonicality,
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
            ComponentOutcome {
                pieces: vec![ComponentReplacementPiece::Slot(0)],
                slots: vec![slot],
                non_canonicality: NonCanonicality::RESOLVE,
            }
        }
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
        non_canonicality: NonCanonicality,
    },
    /// A bare standalone slot remains policy-ambiguous and needs an explicit
    /// text/emoji choice from the caller.
    ResolvePresentation { default: Presentation },
}

/// Decide how a singleton-base emoji-like unit should be analyzed.
///
/// The fixed-cleanup precedence lives in
/// `docs/designs/features/sequence-handling.markdown`. This helper still uses
/// an internal outcome enum because singleton analysis needs to distinguish
/// three replacement shapes before `ComponentOutcome` is built:
///
/// - already canonical
/// - deterministic repair with a fixed rendered base presentation
/// - policy-driven text/emoji resolution with a public decision slot
///
/// Rule 5 is the only path where policy may be consulted.
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
        non_canonicality: fixed_cleanup_non_canonicality(
            presentation_selectors_after_base,
            canonical_presentation,
            modifications,
        ),
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
                non_canonicality: NonCanonicality::unsanctioned(
                    presentation_selectors_after_base.len(),
                ),
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
                non_canonicality: NonCanonicality::new(
                    presentation_selectors_after_base.len() - 1,
                    0,
                    1,
                    0,
                ),
            }
        }
        // More than one presentation selector but the first is meaningful
        // under this rule: the first selector is canonical, so the only
        // violation is the unsanctioned selector cleanup after it.
        (_, &[current_presentation, _, ..]) => SingletonAnalysisOutcome::Repair {
            canonical_presentation: Some(current_presentation),
            non_canonicality: NonCanonicality::unsanctioned(
                presentation_selectors_after_base.len() - 1,
            ),
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
                non_canonicality: NonCanonicality::REDUNDANT,
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

fn count_presentation_selectors_in_item(item: &ScanItem<'_>) -> usize {
    match &item.kind {
        ScanKind::UnsanctionedPresentationSelectors(selectors) => selectors.len(),
        _ => 0,
    }
}

fn count_modification_presentation_selectors(m: &EmojiModification) -> usize {
    match m {
        EmojiModification::EmojiModifier {
            presentation_selectors_after_modifier,
            ..
        } => presentation_selectors_after_modifier.len(),
        EmojiModification::EnclosingKeycap {
            presentation_selectors_after_keycap,
        } => presentation_selectors_after_keycap.len(),
        EmojiModification::TagModifier(runs) => runs
            .iter()
            .map(|run| run.presentation_selectors_after_tag.len())
            .sum(),
    }
}

fn count_trailing_modification_presentation_selectors(
    modifications: &[EmojiModification],
) -> usize {
    modifications
        .iter()
        .map(count_modification_presentation_selectors)
        .sum()
}

fn fixed_cleanup_non_canonicality(
    presentation_selectors_after_base: &[Presentation],
    canonical_presentation: Option<Presentation>,
    modifications: &[EmojiModification],
) -> NonCanonicality {
    let mut non_canonicality = NonCanonicality::unsanctioned(
        count_trailing_modification_presentation_selectors(modifications),
    );
    let canonical = canonical_presentation.as_slice();

    if presentation_selectors_after_base == canonical {
        return non_canonicality;
    }

    if presentation_selectors_after_base.starts_with(canonical) {
        non_canonicality.unsanctioned_selectors += presentation_selectors_after_base
            .len()
            .saturating_sub(canonical.len());
    } else {
        non_canonicality += NonCanonicality::DEFECTIVE;
    }

    non_canonicality
}

fn modification_has_trailing_presentation_selectors(m: &EmojiModification) -> bool {
    count_modification_presentation_selectors(m) != 0
}

fn has_trailing_modification_presentation_selectors(modifications: &[EmojiModification]) -> bool {
    modifications
        .iter()
        .any(modification_has_trailing_presentation_selectors)
}

// --- Finding builders ---

fn unambiguous_finding<'a>(
    item: &ScanItem<'a>,
    non_canonicality: NonCanonicality,
    fix_replacement: String,
) -> Finding<'a> {
    let mut builder = FindingBuilder::with_non_canonicality(non_canonicality);
    builder.push_literal(fix_replacement);
    builder.finish(item)
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

struct FindingBuilder {
    non_canonicality: NonCanonicality,
    slots: Vec<ReplacementSlotPlan>,
    pieces: Vec<ReplacementPiece>,
}

impl FindingBuilder {
    fn new() -> Self {
        Self {
            non_canonicality: NonCanonicality::default(),
            slots: Vec::new(),
            pieces: Vec::new(),
        }
    }

    fn with_non_canonicality(non_canonicality: NonCanonicality) -> Self {
        Self {
            non_canonicality,
            slots: Vec::new(),
            pieces: Vec::new(),
        }
    }

    fn push_literal(&mut self, literal: String) {
        self.pieces.push(ReplacementPiece::Literal(literal));
    }

    // Component outcomes are already expressed in their own local slot space.
    // When they are appended to the whole finding, slot references in their
    // render pieces must be shifted to the current slot offset.
    fn extend_component(&mut self, outcome: ComponentOutcome) {
        self.non_canonicality += outcome.non_canonicality;
        let slot_offset = self.slots.len();
        self.slots.extend(outcome.slots);
        self.pieces
            .extend(outcome.pieces.into_iter().map(|piece| match piece {
                ComponentReplacementPiece::Literal(literal) => ReplacementPiece::Literal(literal),
                ComponentReplacementPiece::Slot(slot_index) => {
                    ReplacementPiece::Slot(slot_offset + slot_index)
                }
            }));
    }

    fn extend_link(&mut self, link: &ZwjLink) {
        self.non_canonicality +=
            NonCanonicality::unsanctioned(link.presentation_selectors_after_link.len());
        self.push_literal(unicode::ZWJ.to_string());
    }

    // Empty builders correspond to fully canonical items, so they do not
    // produce findings.
    fn build<'a>(self, item: &ScanItem<'a>) -> Option<Finding<'a>> {
        if self.non_canonicality.is_empty() {
            None
        } else {
            Some(self.finish(item))
        }
    }

    fn finish<'a>(self, item: &ScanItem<'a>) -> Finding<'a> {
        debug_assert!(
            !self.non_canonicality.is_empty(),
            "finding builder finish requires a non-empty non-canonicality summary"
        );
        Finding {
            span: item.span.clone(),
            raw: item.raw,
            replacement_plan: self.build_replacement_plan(),
        }
    }

    fn build_replacement_plan(self) -> ReplacementPlan {
        let default_replacement = render_default_replacement(&self.slots, &self.pieces);
        let plan = ReplacementPlan {
            non_canonicality: self.non_canonicality,
            decision_slots: self.slots.iter().map(|slot| slot.public.clone()).collect(),
            slots: self.slots,
            pieces: self.pieces,
            default_replacement,
        };
        debug_assert_eq!(
            plan.render_replacement(&plan.default_decisions()),
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

// --- Fixed emoji-like rendering ---

fn render_singleton(
    base: char,
    presentation: Option<Presentation>,
    modifications: &[EmojiModification],
) -> String {
    FixedEmojiLike::singleton_base(base, presentation, modifications).render_to_string()
}
