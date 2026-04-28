use std::ops::{Add, AddAssign, Range};

use crate::presentation::Presentation;

/// Count summary for why a scanned item is non-canonical.
///
/// These axes are compositional rather than mutually exclusive. A finding may
/// simultaneously include selector cleanup, deterministic sequence defects,
/// redundant selectors, deterministic selector insertion, and policy-driven
/// bare-base resolution.
///
/// The scalar-length effect of a finding's default replacement is derived from
/// these selector-level counters:
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

/// One fixed or caller-selectable part of a finding's replacement.
///
/// These replacement types are the analysis API's render representation after
/// the semantic formatter model has already resolved selector contexts and
/// policy positions. They should not be treated as the design-spec vocabulary
/// for selector classification; see `docs/designs/core/formatting-model.markdown`
/// for that model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReplacementElement<D> {
    Fixed(String),
    Choice(ReplacementChoice<D>),
}

/// One caller-selectable replacement choice.
///
/// Each valid decision selects a complete replacement string for this choice.
/// That string may include surrounding characters needed to keep the choice
/// renderable after local cleanup.
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
/// let choice = finding.replacement_choices().next().unwrap();
/// assert_eq!(
///     choice.decisions().collect::<Vec<_>>(),
///     [Presentation::Text, Presentation::Emoji]
/// );
/// assert_eq!(choice.default_decision(), Presentation::Emoji);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacementChoice<D> {
    pub(super) default: D,
    pub(super) options: Vec<ReplacementOption<D>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReplacementOption<D> {
    pub(super) decision: D,
    pub(super) replacement: String,
}

impl<D: Copy> ReplacementChoice<D> {
    /// Valid decisions for this replacement choice.
    #[must_use]
    pub fn decisions(&self) -> impl ExactSizeIterator<Item = D> + '_ {
        self.options.iter().map(|option| option.decision)
    }

    /// The decision batch formatting applies to this choice by default.
    #[must_use]
    pub const fn default_decision(&self) -> D {
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
    fn default_replacement(&self) -> &str {
        self.replacement(&self.default)
            .expect("replacement choice constructor validates its default decision")
    }
}

/// Replacement fragment and non-canonicality accounting before source location
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

    pub(super) const fn is_empty(&self) -> bool {
        self.non_canonicality.is_empty()
    }

    pub(super) fn push_fixed(&mut self, text: String) {
        self.elements.push(ReplacementElement::Fixed(text));
    }
}

impl Add for ReplacementAnalysis {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for ReplacementAnalysis {
    fn add_assign(&mut self, rhs: Self) {
        self.non_canonicality += rhs.non_canonicality;
        self.elements.extend(rhs.elements);
    }
}

/// A single non-canonical scanned item with its valid replacement choices.
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
/// assert_eq!(finding.default_replacement(), "\u{00A9}\u{FE0F}");
/// assert_eq!(
///     finding.replacement(&[Presentation::Text]).unwrap(),
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

    /// Caller-selectable replacement choices in this finding's decision vector.
    ///
    /// A fixed repair has no replacement choices. Call [`Finding::replacement`]
    /// with an empty decision slice to apply that repair.
    pub fn replacement_choices(
        &self,
    ) -> impl Iterator<Item = &ReplacementChoice<Presentation>> + '_ {
        self.analysis
            .elements
            .iter()
            .filter_map(|element| match element {
                ReplacementElement::Fixed(_) => None,
                ReplacementElement::Choice(choice) => Some(choice),
            })
    }

    /// The replacement text for the decision batch formatting applies by default.
    ///
    /// This is infallible because each replacement choice stores its own default
    /// decision next to the options it may select. The default is validated when
    /// constructing the choice, so there is no separate default decision
    /// vector for callers to keep in sync.
    #[must_use]
    pub fn default_replacement(&self) -> String {
        let mut out = String::new();
        for element in &self.analysis.elements {
            match element {
                ReplacementElement::Fixed(text) => out.push_str(text),
                ReplacementElement::Choice(choice) => out.push_str(choice.default_replacement()),
            }
        }
        out
    }

    /// Return the replacement text for a valid replacement decision vector.
    ///
    /// The decision slice must contain exactly one choice for each
    /// [`ReplacementChoice`] returned by [`Finding::replacement_choices`]. Fixed
    /// repairs have no choices and therefore use an empty decision slice.
    ///
    /// Returns `None` when the decision vector has the wrong length or contains
    /// a choice that is not valid for its replacement choice. Callers that want
    /// to skip a finding can keep [`Finding::raw`].
    #[must_use]
    pub fn replacement(&self, decisions: &[Presentation]) -> Option<String> {
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

impl<'a> Finding<'a> {
    pub(super) fn new(item: &crate::scanner::ScanItem<'a>, analysis: ReplacementAnalysis) -> Self {
        assert!(
            !analysis.is_empty(),
            "finding construction requires a non-empty replacement analysis"
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
