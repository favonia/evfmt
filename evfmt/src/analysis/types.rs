use std::ops::{Add, AddAssign, Range};
use std::slice;

use crate::presentation::Presentation;

/// Count summary for why a scanned item is non-canonical.
///
/// These axes are compositional rather than mutually exclusive. A finding may
/// simultaneously include selector cleanup, deterministic sequence defects,
/// redundant selectors, deterministic selector insertion, and policy-driven
/// bare-base resolution.
///
/// The categories describe how non-canonical selector usage is repaired or
/// exposed to callers:
///
/// - unsanctioned or structurally broken selector usage is removed
/// - fixed-cleanup sequence defects are repaired without policy
/// - redundant sanctioned selectors are removed when the active policy prefers
///   the bare form
/// - missing required presentation selectors are inserted by deterministic
///   cleanup rather than by policy choice
/// - genuinely ambiguous bare bases become caller-resolvable policy decisions
///
/// A redundant variation selector is not unsanctioned. It belongs to a
/// sanctioned Unicode structure but is non-canonical under the active
/// formatter policy because the same context canonically stays bare. Likewise,
/// a missing required selector is separate from a defective sequence: inserting
/// `FE0F` for a deterministic tag context is fixed cleanup, not a UTS #51
/// defective emoji modifier repair.
///
/// The scalar-length effect of a finding's default canonical replacement is
/// derived from these selector-level counters:
///
/// ```text
/// replacement.chars().count() - raw.chars().count()
///   = missing_required_selectors + bases_to_resolve
///   - unsanctioned_selectors - defective_sequences - redundant_selectors
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
///     NonCanonicality::new(1, 0, 0, 0, 0)
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NonCanonicality {
    /// Count of presentation selectors removed as unsanctioned cleanup.
    pub unsanctioned_selectors: usize,
    /// Count of deterministic sequence defects, such as a UTS #51 defective
    /// emoji modifier sequence, that need repair but do not expose a policy
    /// choice.
    pub defective_sequences: usize,
    /// Count of sanctioned selectors dropped because the active policy prefers
    /// bare form.
    pub redundant_selectors: usize,
    /// Count of required presentation selectors inserted by deterministic
    /// cleanup rather than policy choice.
    pub missing_required_selectors: usize,
    /// Count of bare bases that the active policy asks callers to resolve.
    pub bases_to_resolve: usize,
}

impl Default for NonCanonicality {
    fn default() -> Self {
        Self::new(0, 0, 0, 0, 0)
    }
}

impl NonCanonicality {
    pub(super) const DEFECTIVE: Self = Self::new(0, 1, 0, 0, 0);
    pub(super) const REDUNDANT: Self = Self::new(0, 0, 1, 0, 0);
    pub(super) const MISSING_REQUIRED: Self = Self::new(0, 0, 0, 1, 0);
    pub(super) const RESOLVE: Self = Self::new(0, 0, 0, 0, 1);

    /// Create an explicit non-canonicality summary.
    #[must_use]
    pub const fn new(
        unsanctioned_selectors: usize,
        defective_sequences: usize,
        redundant_selectors: usize,
        missing_required_selectors: usize,
        bases_to_resolve: usize,
    ) -> Self {
        Self {
            unsanctioned_selectors,
            defective_sequences,
            redundant_selectors,
            missing_required_selectors,
            bases_to_resolve,
        }
    }

    pub(super) const fn unsanctioned(count: usize) -> Self {
        Self::new(count, 0, 0, 0, 0)
    }

    pub(super) const fn is_empty(self) -> bool {
        self.unsanctioned_selectors == 0
            && self.defective_sequences == 0
            && self.redundant_selectors == 0
            && self.missing_required_selectors == 0
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
            missing_required_selectors: self.missing_required_selectors
                + rhs.missing_required_selectors,
            bases_to_resolve: self.bases_to_resolve + rhs.bases_to_resolve,
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
/// positions. They should not be treated as the design-spec vocabulary for
/// selector classification; see `docs/designs/core/formatting-model.markdown`
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
/// assert_eq!(finding.default_canonical_replacement(), "\u{00A9}\u{FE0F}");
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
    /// Each decision slot is one ambiguous selector context in source order
    /// within the scanned item. Every current slot accepts
    /// [`Presentation::Text`] or [`Presentation::Emoji`]. Fixed cleanup
    /// contributes no decision slot.
    ///
    /// The iterator length is equal to
    /// [`NonCanonicality::bases_to_resolve`] for this finding.
    #[must_use]
    pub fn default_decisions(&self) -> impl ExactSizeIterator<Item = Presentation> + '_ {
        DefaultDecisions {
            elements: self.analysis.elements.iter(),
            remaining: self.analysis.decision_count(),
        }
    }

    /// The canonical replacement using the default decision vector.
    ///
    /// This is infallible because each decision slot stores its own default
    /// next to the options it may select. The default is validated when
    /// constructing the slot.
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
    /// Each decision slot is one ambiguous selector context in source order
    /// within the scanned item. Every current slot accepts
    /// [`Presentation::Text`] or [`Presentation::Emoji`]. Fixed cleanup is
    /// included in the whole replacement but contributes no decision slot.
    ///
    /// Returns `None` when the decision vector has the wrong length or contains
    /// a decision value that is not valid for its slot. That `None` reports
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
            analysis.non_canonicality.bases_to_resolve,
            "finding decision count must match bases_to_resolve"
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
