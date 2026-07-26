use std::ops::{Add, AddAssign, Range};
use std::slice;

use crate::presentation::Presentation;

/// Count summary for why a scanned item is non-canonical.
///
/// These axes are compositional rather than mutually exclusive. A finding may
/// simultaneously include unsanctioned selectors, modifier-related defective
/// selectors, tag-context selector cleanup, policy-redundant selectors, and
/// policy-driven presentation decisions.
///
/// The categories describe how non-canonical selector usage is repaired or
/// exposed to callers:
///
/// - unsanctioned selector usage is removed
/// - sanctioned modifier-defective selectors are removed when a base with
///   `Emoji_Modifier_Base` precedes an emoji modifier
/// - additional defective selectors are sanctioned selectors removed by the
///   formatter's narrow extension to variation-sequence emoji bases without
///   `Emoji_Modifier_Base`
/// - an intervening selector on a base without a sanctioned variation
///   sequence is removed as unsanctioned selector usage
/// - tag-context selectors are removed, replaced, or supplied by tag-specific
///   cleanup
/// - policy-redundant selectors are removed when the active policy prefers the
///   bare form
/// - ambiguous selector slots become caller-resolvable presentation decisions
///
/// Tag-context counters are separate because tag-sequence presentation has a
/// different Unicode history from ordinary policy resolution: current UTS #51
/// admits broad base-and-tag spellings, while RGI tag sequences deliberately
/// use an emoji-default base.
///
/// The scalar-length effect of a finding's default canonical replacement is
/// derived from these counters. Each current category changes the default
/// replacement by one presentation-selector scalar per count:
///
/// ```text
/// replacement.chars().count() - raw.chars().count()
///   = tag_forced_presentations + presentation_decisions
///   - unsanctioned_selectors - modifier_defective_selectors
///   - additional_defective_selectors - tag_conflicting_selectors
///   - tag_redundant_selectors - policy_redundant_selectors
/// ```
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, scan};
/// use evfmt::analysis::{NonCanonicality, analyze_scan_item};
///
/// let policy = Policy::default();
/// let finding = scan("A\u{FE0F}")
///     .find_map(|item| analyze_scan_item(&item, &policy))
///     .unwrap();
///
/// assert_eq!(
///     finding.non_canonicality(),
///     NonCanonicality::new(1, 0, 0, 0, 0, 0, 0, 0)
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NonCanonicality {
    /// Count of presentation selectors removed as unsanctioned selector usage.
    pub unsanctioned_selectors: usize,
    /// Count of sanctioned `FE0F` selectors removed in the UTS #51 defective
    /// modifier context where an `Emoji_Modifier_Base` character precedes an
    /// `Emoji_Modifier`.
    ///
    /// An intervening `FE0F` on an `Emoji_Modifier_Base` character without a
    /// sanctioned variation sequence counts as [`Self::unsanctioned_selectors`].
    pub modifier_defective_selectors: usize,
    /// Count of sanctioned `FE0F` selectors removed before an emoji modifier
    /// when the recognized variation-sequence emoji base lacks
    /// `Emoji_Modifier_Base`.
    ///
    /// This is an `evfmt` formatter classification. It extends the modifier
    /// cleanup to the narrow recognized shape `Emoji FE0F Emoji_Modifier`; it
    /// keeps Unicode conformance status governed by UTS #51 and the
    /// `Emoji_Modifier_Base` boundary.
    pub additional_defective_selectors: usize,
    /// Count of sanctioned tag-context selectors whose requested presentation
    /// differs from the canonical base presentation in tag context.
    pub tag_conflicting_selectors: usize,
    /// Count of tag-context selector slots where canonical output supplies
    /// emoji presentation that was not already present in the source.
    pub tag_forced_presentations: usize,
    /// Count of sanctioned tag-context selectors whose requested emoji
    /// presentation is already carried by the canonical bare base in tag
    /// context.
    pub tag_redundant_selectors: usize,
    /// Count of sanctioned selectors dropped because the active policy chooses
    /// bare form as canonical.
    pub policy_redundant_selectors: usize,
    /// Count of policy presentation decisions callers may resolve.
    pub presentation_decisions: usize,
}

impl Default for NonCanonicality {
    fn default() -> Self {
        Self::new(0, 0, 0, 0, 0, 0, 0, 0)
    }
}

impl NonCanonicality {
    pub(super) const MODIFIER_DEFECTIVE_SELECTOR: Self = Self::new(0, 1, 0, 0, 0, 0, 0, 0);
    pub(super) const ADDITIONAL_DEFECTIVE_SELECTOR: Self = Self::new(0, 0, 1, 0, 0, 0, 0, 0);
    pub(super) const TAG_CONFLICTING_SELECTOR: Self = Self::new(0, 0, 0, 1, 0, 0, 0, 0);
    pub(super) const TAG_FORCED_PRESENTATION: Self = Self::new(0, 0, 0, 0, 1, 0, 0, 0);
    pub(super) const TAG_REDUNDANT_SELECTOR: Self = Self::new(0, 0, 0, 0, 0, 1, 0, 0);
    pub(super) const POLICY_REDUNDANT_SELECTOR: Self = Self::new(0, 0, 0, 0, 0, 0, 1, 0);
    pub(super) const PRESENTATION_DECISION: Self = Self::new(0, 0, 0, 0, 0, 0, 0, 1);

    /// Create an explicit non-canonicality summary.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    // The public constructor mirrors the eight independent accounting axes.
    pub const fn new(
        unsanctioned_selectors: usize,
        modifier_defective_selectors: usize,
        additional_defective_selectors: usize,
        tag_conflicting_selectors: usize,
        tag_forced_presentations: usize,
        tag_redundant_selectors: usize,
        policy_redundant_selectors: usize,
        presentation_decisions: usize,
    ) -> Self {
        Self {
            unsanctioned_selectors,
            modifier_defective_selectors,
            additional_defective_selectors,
            tag_conflicting_selectors,
            tag_forced_presentations,
            tag_redundant_selectors,
            policy_redundant_selectors,
            presentation_decisions,
        }
    }

    pub(super) const fn unsanctioned(count: usize) -> Self {
        Self::new(count, 0, 0, 0, 0, 0, 0, 0)
    }

    pub(super) const fn is_empty(self) -> bool {
        self.unsanctioned_selectors == 0
            && self.modifier_defective_selectors == 0
            && self.additional_defective_selectors == 0
            && self.tag_conflicting_selectors == 0
            && self.tag_forced_presentations == 0
            && self.tag_redundant_selectors == 0
            && self.policy_redundant_selectors == 0
            && self.presentation_decisions == 0
    }
}

impl Add for NonCanonicality {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            unsanctioned_selectors: self.unsanctioned_selectors + rhs.unsanctioned_selectors,
            modifier_defective_selectors: self.modifier_defective_selectors
                + rhs.modifier_defective_selectors,
            additional_defective_selectors: self.additional_defective_selectors
                + rhs.additional_defective_selectors,
            tag_conflicting_selectors: self.tag_conflicting_selectors
                + rhs.tag_conflicting_selectors,
            tag_forced_presentations: self.tag_forced_presentations + rhs.tag_forced_presentations,
            tag_redundant_selectors: self.tag_redundant_selectors + rhs.tag_redundant_selectors,
            policy_redundant_selectors: self.policy_redundant_selectors
                + rhs.policy_redundant_selectors,
            presentation_decisions: self.presentation_decisions + rhs.presentation_decisions,
        }
    }
}

impl AddAssign for NonCanonicality {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

/// One fixed or caller-selectable replacement assembly piece.
///
/// These elements are a private renderer representation after the semantic
/// formatter model has already resolved selector contexts and policy
/// keys. They should not be treated as the design-spec vocabulary for
/// selector classification; see `docs/designs/core/formatting.markdown`
/// for that model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReplacementElement<D> {
    Fixed(String),
    Choice(ReplacementChoice<D>),
}

/// One internally caller-selectable replacement assembly option.
///
/// Each valid decision selects a complete replacement string for this assembly
/// piece. That string may include surrounding characters needed to keep the
/// whole finding renderable after local cleanup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReplacementChoice<D> {
    pub(super) default: D,
    pub(super) options: Vec<ReplacementOption<D>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReplacementOption<D> {
    pub(super) decision: D,
    pub(super) replacement: String,
}

impl<D: Copy> ReplacementChoice<D> {
    pub(super) const fn default_decision(&self) -> D {
        self.default
    }
}

impl<D: PartialEq> ReplacementChoice<D> {
    pub(super) fn new(default: D, options: Vec<ReplacementOption<D>>) -> Self {
        assert!(
            options.iter().any(|option| option.decision == default),
            "replacement choice default decision must be one of the options"
        );
        Self { default, options }
    }

    pub(super) fn from_replacements<const N: usize>(
        default: D,
        replacements: [(D, String); N],
    ) -> Self {
        Self::new(
            default,
            replacements
                .into_iter()
                .map(|(decision, replacement)| ReplacementOption {
                    decision,
                    replacement,
                })
                .collect(),
        )
    }

    pub(super) fn replacement(&self, decision: &D) -> Option<&str> {
        self.options
            .iter()
            .find(|option| option.decision == *decision)
            .map(|option| option.replacement.as_str())
    }

    #[allow(clippy::expect_used)] // ReplacementChoice::new validates that the default decision is one of the options.
    fn default_canonical_replacement(&self) -> &str {
        self.replacement(&self.default)
            .expect("replacement choice constructor validates its default decision")
    }
}

/// Replacement assembly and non-canonicality accounting before source location
/// is attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReplacementAnalysis {
    non_canonicality: NonCanonicality,
    elements: Vec<ReplacementElement<Presentation>>,
}

impl ReplacementAnalysis {
    pub(super) fn empty() -> Self {
        Self {
            non_canonicality: NonCanonicality::default(),
            elements: Vec::new(),
        }
    }

    pub(super) fn fixed(non_canonicality: NonCanonicality, replacement: String) -> Self {
        let mut analysis = Self {
            non_canonicality,
            elements: Vec::new(),
        };
        analysis.push_fixed(replacement);
        analysis
    }

    pub(super) fn choice(
        non_canonicality: NonCanonicality,
        choice: ReplacementChoice<Presentation>,
    ) -> Self {
        Self {
            non_canonicality,
            elements: vec![ReplacementElement::Choice(choice)],
        }
    }

    /// Whether this assembled analysis would leave the scanned item canonical.
    ///
    /// This is determined only by the non-canonicality counters. Replacement
    /// elements can still be present because sequence-level analysis may need
    /// them to preserve surrounding structure when another part of the same
    /// item is non-canonical.
    pub(super) const fn is_canonical(&self) -> bool {
        self.non_canonicality.is_empty()
    }

    fn decision_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|element| matches!(element, ReplacementElement::Choice(_)))
            .count()
    }

    pub(super) fn push_fixed(&mut self, text: String) {
        self.elements.push(ReplacementElement::Fixed(text));
    }
}

impl AddAssign for ReplacementAnalysis {
    fn add_assign(&mut self, rhs: Self) {
        self.non_canonicality += rhs.non_canonicality;
        self.elements.extend(rhs.elements);
    }
}

/// A single non-canonical scanned item with its valid replacement decisions.
///
/// `Finding` values are returned only for items that are non-canonical under
/// the policy passed to [`crate::analysis::analyze_scan_item`]. Their
/// [`NonCanonicality`] is guaranteed to be non-empty.
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, Presentation, scan};
/// use evfmt::analysis::analyze_scan_item;
///
/// let policy = Policy::default();
/// let finding = scan("\u{00A9}")
///     .find_map(|item| analyze_scan_item(&item, &policy))
///     .unwrap();
///
/// assert_eq!(finding.raw, "\u{00A9}");
/// assert_eq!(finding.default_canonical_replacement(), "\u{00A9}\u{FE0E}");
/// assert_eq!(
///     finding.canonical_replacement_with_decisions(&[Presentation::Text]).unwrap(),
///     "\u{00A9}\u{FE0E}"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding<'a> {
    /// Byte range in the original input.
    pub span: Range<usize>,
    /// Original raw source slice for the item.
    pub raw: &'a str,
    analysis: ReplacementAnalysis,
}

impl Finding<'_> {
    /// Why the analyzed item is non-canonical.
    #[must_use]
    pub const fn non_canonicality(&self) -> NonCanonicality {
        self.analysis.non_canonicality
    }

    /// The decision vector formatting applies to this finding by default.
    ///
    /// Each presentation decision is one ambiguous selector slot in source order
    /// within the scanned item. Every current decision accepts
    /// [`Presentation::Text`] or [`Presentation::Emoji`]. Fixed cleanup
    /// contributes no presentation decision.
    ///
    /// The iterator length is equal to
    /// [`NonCanonicality::presentation_decisions`] for this finding.
    #[must_use]
    pub fn default_decisions(&self) -> impl ExactSizeIterator<Item = Presentation> + '_ {
        DefaultDecisions {
            elements: self.analysis.elements.iter(),
            remaining: self.analysis.decision_count(),
        }
    }

    /// Return the canonical replacement selected by this finding's default
    /// decision vector.
    ///
    /// Use this when accepting evfmt's default repair for the whole finding.
    /// Unlike [`Finding::canonical_replacement_with_decisions`], this method
    /// cannot fail because the decisions come from the finding itself.
    #[must_use]
    pub fn default_canonical_replacement(&self) -> String {
        let mut out = String::new();
        for element in &self.analysis.elements {
            match element {
                ReplacementElement::Fixed(text) => out.push_str(text),
                ReplacementElement::Choice(choice) => {
                    out.push_str(choice.default_canonical_replacement());
                }
            }
        }
        out
    }

    /// Return the canonical whole-item replacement for a valid decision vector.
    ///
    /// Each presentation decision is one ambiguous selector slot in source order
    /// within the scanned item. Every current decision accepts
    /// [`Presentation::Text`] or [`Presentation::Emoji`]. Fixed cleanup is
    /// included in the whole replacement but contributes no presentation decision.
    ///
    /// Returns `None` when the decision vector is invalid. Because the current
    /// API accepts both presentation decisions at every slot, the only invalid
    /// decision vectors are those with the wrong length. That `None` reports
    /// invalid caller input; it does not mean this finding is canonical.
    /// Callers that want to skip a finding can keep [`Finding::raw`].
    #[must_use]
    pub fn canonical_replacement_with_decisions(
        &self,
        decisions: &[Presentation],
    ) -> Option<String> {
        let mut decisions = decisions.iter();
        let mut out = String::new();

        for element in &self.analysis.elements {
            match element {
                ReplacementElement::Fixed(text) => out.push_str(text),
                ReplacementElement::Choice(choice) => {
                    let decision = decisions.next()?;
                    out.push_str(choice.replacement(decision)?);
                }
            }
        }

        if decisions.next().is_none() {
            Some(out)
        } else {
            None
        }
    }
}

struct DefaultDecisions<'a> {
    elements: slice::Iter<'a, ReplacementElement<Presentation>>,
    remaining: usize,
}

impl Iterator for DefaultDecisions<'_> {
    type Item = Presentation;

    fn next(&mut self) -> Option<Self::Item> {
        for element in self.elements.by_ref() {
            if let ReplacementElement::Choice(choice) = element {
                self.remaining -= 1;
                return Some(choice.default_decision());
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for DefaultDecisions<'_> {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<'a> Finding<'a> {
    pub(super) fn new(item: &crate::scanner::ScanItem<'a>, analysis: ReplacementAnalysis) -> Self {
        assert!(
            !analysis.is_canonical(),
            "finding construction requires non-empty non-canonicality"
        );
        assert_eq!(
            analysis.decision_count(),
            analysis.non_canonicality.presentation_decisions,
            "finding decision count must match presentation_decisions"
        );
        Self {
            span: item.span.clone(),
            raw: item.raw,
            analysis,
        }
    }

    pub(super) fn fixed(
        item: &crate::scanner::ScanItem<'a>,
        non_canonicality: NonCanonicality,
        replacement: String,
    ) -> Self {
        Self::new(
            item,
            ReplacementAnalysis::fixed(non_canonicality, replacement),
        )
    }
}
