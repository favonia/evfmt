//! Policy-aware analysis for scanned emoji variation structures.
//!
//! This module implements the policy and fixed-rule analysis steps from the
//! conceptual formatting algorithm. It produces findings with variable
//! replacement choices, so callers can choose a replacement
//! without re-reading policy for the same item.
//! Interactive callers normally use this module directly: [`analyze_scan_item`]
//! computes reasonableness, applies [`Policy`] only where policy is relevant,
//! and stores valid replacement choices in each [`Finding`].
//!
//! - [`crate::scanner`] decides structural item boundaries
//! - [`analyze_scan_item`] turns policy-neutral reasonableness into findings and replacement choices
//!
//! Use this module when callers need to inspect or override repairs
//! item-by-item; otherwise [`crate::format_text`] is the shorter path.
//!
//! # Examples
//!
//! ```rust
//! use evfmt::{Policy, scan};
//! use evfmt::analysis::analyze_scan_item;
//!
//! let policy = Policy::default();
//! let input = "A\u{FE0F}\u{00A9}";
//!
//! let repaired = scan(input)
//!     .map(|item| {
//!         analyze_scan_item(&item, &policy).map_or_else(
//!             || item.raw.to_owned(),
//!             |finding| finding.default_replacement(),
//!         )
//!     })
//!     .collect::<String>();
//!
//! assert_eq!(repaired, "A\u{00A9}\u{FE0F}");
//! ```

use crate::policy::{Policy, SingletonRule};
use crate::presentation::Presentation;
use crate::scanner::{
    EmojiLike, EmojiModification, EmojiSequence, EmojiStem, ScanItem, ScanKind, ZwjJoinedEmoji,
    ZwjLink,
};
use crate::unicode;

mod render;
mod types;

use render::{render_flag, render_singleton};
use types::ReplacementAnalysis;
pub use types::{Finding, NonCanonicality, ReplacementChoice};

#[cfg(test)]
mod tests;

// --- Public API ---

/// Analyze a scanned item under the current formatter policy.
///
/// # Examples
///
/// ```rust
/// use evfmt::{Policy, scan};
/// use evfmt::analysis::{NonCanonicality, analyze_scan_item};
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
///     NonCanonicality::new(1, 0, 0, 0, 0)
/// );
/// assert_eq!(finding.replacement(&[]).unwrap(), "");
/// ```
#[must_use]
pub fn analyze_scan_item<'a>(item: &ScanItem<'a>, policy: &Policy) -> Option<Finding<'a>> {
    match &item.kind {
        ScanKind::Passthrough => None,
        ScanKind::UnsanctionedPresentationSelectors(selectors) => Some(Finding::fixed(
            item,
            NonCanonicality::unsanctioned(selectors.len()),
            String::new(),
        )),
        ScanKind::EmojiSequence(sequence) => match sequence {
            // The scanner preserves malformed ZWJ-like shapes such as leading,
            // consecutive, or trailing ZWJ links. Item analysis keeps that
            // non-selector structure intact: these paths only remove or
            // normalize presentation selectors.
            //
            // `LinksOnly` contributes only link cleanup. `EmojiHeaded` uses
            // the same accumulation path regardless of whether the scanner
            // found one component or a joined chain: analyze each component
            // with the same component-local policy/fixed cleanup it would use
            // outside surrounding ZWJ links, then stitch the literal ZWJ links
            // back into the replacement elements in source order.
            //
            // This is the analysis-side implementation of the ZWJ-related
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

/// Analyze one scanner item whose ZWJ-related structure begins with an
/// emoji-like component.
///
/// Inputs are already structurally grouped by the scanner: `first` is the
/// leading component, `joined` are complete ZWJ-plus-component pairs, and
/// `trailing_links` are literal ZWJ links without a following component. This
/// function preserves that non-selector structure in source order. It delegates
/// component-local selector policy/cleanup to [`analyze_component`] and counts
/// only selector cleanup attached to ZWJ links itself.
fn analyze_emoji_headed_sequence<'a>(
    item: &ScanItem<'a>,
    first: &EmojiLike,
    joined: &[ZwjJoinedEmoji],
    trailing_links: &[ZwjLink],
    policy: &Policy,
) -> Option<Finding<'a>> {
    let mut analysis = ReplacementAnalysis::empty();
    analysis += analyze_component(first, policy);

    for joined in joined {
        analysis += analyze_link(&joined.link);
        analysis += analyze_component(&joined.emoji, policy);
    }

    for link in trailing_links {
        analysis += analyze_link(link);
    }

    if analysis.is_empty() {
        None
    } else {
        Some(Finding::new(item, analysis))
    }
}

// --- ZWJ sequence analysis helpers ---

/// Analyze one scanner item made only of ZWJ links.
///
/// Links-only items contain no emoji component and therefore no component-local
/// selector policy. The ZWJ code points are preserved literally; presentation
/// selectors after the links are counted by [`analyze_link`].
fn analyze_links_only_zwj_sequence<'a>(
    item: &ScanItem<'a>,
    links: &[ZwjLink],
) -> Option<Finding<'a>> {
    let mut analysis = ReplacementAnalysis::empty();
    for link in links {
        analysis += analyze_link(link);
    }
    if analysis.is_empty() {
        None
    } else {
        Some(Finding::new(item, analysis))
    }
}

/// Analyze one literal ZWJ link between or after components.
///
/// The ZWJ itself is preserved. Presentation selectors attached after the ZWJ
/// link are not owned by either neighboring component, so this counts them as
/// unsanctioned cleanup.
fn analyze_link(link: &ZwjLink) -> ReplacementAnalysis {
    ReplacementAnalysis::fixed(
        NonCanonicality::unsanctioned(link.presentation_selectors_after_link.len()),
        unicode::ZWJ.to_string(),
    )
}

/// Analyze one emoji-like component independent of surrounding ZWJ links.
///
/// A component is either a singleton base plus modifications or a regional
/// indicator flag plus modifications. The returned replacement elements render
/// only this component; callers are responsible for inserting any surrounding
/// ZWJ links.
fn analyze_component(emoji: &EmojiLike, policy: &Policy) -> ReplacementAnalysis {
    match &emoji.stem {
        EmojiStem::SingletonBase {
            base,
            presentation_selectors_after_base,
        } => analyze_singleton_component(
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
        } => ReplacementAnalysis::fixed(
            NonCanonicality::unsanctioned(
                presentation_selectors_after_first_ri.len()
                    + presentation_selectors_after_second_ri.len()
                    + count_selectors_after_modifications(&emoji.modifiers),
            ),
            render_flag(*first_ri, *second_ri, &emoji.modifiers),
        ),
    }
}

/// Analyze one singleton-base component, including its base selector run and
/// all modification suffixes.
///
/// The base selector run (`presentation_selectors_after_base`) decides the
/// replacement presentation for the base itself. The modification list is
/// rendered after that base, with presentation selectors after modifiers,
/// keycap marks, and tag characters stripped and counted as unsanctioned
/// cleanup. This function is the boundary where those two accounting streams
/// are combined into one [`ReplacementAnalysis`].
fn analyze_singleton_component(
    base: char,
    presentation_selectors_after_base: &[Presentation],
    modifications: &[EmojiModification],
    policy: &Policy,
) -> ReplacementAnalysis {
    let modification_selector_cleanup =
        NonCanonicality::unsanctioned(count_selectors_after_modifications(modifications));

    match analyze_singleton_base_selectors(
        base,
        presentation_selectors_after_base,
        modifications.first(),
        policy,
    ) {
        SingletonBaseSelectorOutcome::Deterministic {
            canonical_presentation,
            non_canonicality,
        } => ReplacementAnalysis::fixed(
            non_canonicality + modification_selector_cleanup,
            render_singleton(base, canonical_presentation, modifications),
        ),
        SingletonBaseSelectorOutcome::NeedsPresentationDecision {
            default,
            non_canonicality,
        } => {
            let choice = ReplacementChoice::from_replacements(
                default,
                [
                    (
                        Presentation::Text,
                        render_singleton(base, Some(Presentation::Text), modifications),
                    ),
                    (
                        Presentation::Emoji,
                        render_singleton(base, Some(Presentation::Emoji), modifications),
                    ),
                ],
            );
            ReplacementAnalysis::choice(non_canonicality + modification_selector_cleanup, choice)
        }
    }
}

// --- Singleton analysis planning ---

/// Analysis of the presentation selector run immediately after a singleton
/// base, before any cleanup from later modifications is added.
#[derive(Clone, Copy)]
enum SingletonBaseSelectorOutcome {
    /// The base selector run has one deterministic canonical form.
    ///
    /// This includes already-canonical input, fixed cleanup, and policy cases
    /// that do not expose a caller choice. `canonical_presentation` is only the
    /// selector state for the base itself.
    Deterministic {
        canonical_presentation: Option<Presentation>,
        non_canonicality: NonCanonicality,
    },
    /// The base selector run remains policy-ambiguous and needs an explicit
    /// text/emoji choice from the caller.
    NeedsPresentationDecision {
        default: Presentation,
        non_canonicality: NonCanonicality,
    },
}

/// Analyze the presentation selector run immediately after a singleton base.
///
/// Inputs:
/// - `base`: the singleton base character.
/// - `presentation_selectors_after_base`: the complete `FE0E`/`FE0F` run
///   immediately after `base`.
/// - `first_modification`: the first structural modification after that base
///   selector run, if any. It selects the fixed-cleanup context for the base
///   selector run; later modifications do not affect this decision.
///
/// Output: the canonical selector state for the base, plus the
/// [`NonCanonicality`] for this base-selector decision. That count may include
/// policy resolution of a bare base as well as cleanup of explicit base
/// selectors. This function does not inspect or count selectors after
/// modifier/keycap/tag characters; those belong to
/// [`analyze_singleton_component`].
///
/// The fixed-cleanup precedence lives in
/// `docs/designs/features/sequence-handling.markdown`.
///
/// Rule 6 is the only path where policy may be consulted.
fn analyze_singleton_base_selectors(
    base: char,
    presentation_selectors_after_base: &[Presentation],
    first_modification: Option<&EmojiModification>,
    policy: &Policy,
) -> SingletonBaseSelectorOutcome {
    // Precedence 1: if the base has no variation-sequence data, any explicit
    // base presentation would be unsanctioned. Use bare base presentation.
    if !unicode::has_variation_sequence(base) {
        return SingletonBaseSelectorOutcome::Deterministic {
            canonical_presentation: None,
            non_canonicality: NonCanonicality::unsanctioned(
                presentation_selectors_after_base.len(),
            ),
        };
    }

    // AI MAINTAINER NOTE: keep this precedence cascade as the single
    // executable copy of the fixed-cleanup table in the design document. Each
    // fixed-cleanup branch must construct the complete base outcome:
    // `canonical_presentation` and `NonCanonicality`. Do not move rule
    // dispatch into helper functions or split output selection from violation
    // accounting. Modification suffix cleanup belongs to
    // `analyze_singleton_component`, not to this cascade.
    match first_modification {
        // Precedence 2: a sanctioned FE0E remains attached to the base as
        // text presentation. The following modifier is preserved in source
        // order, but no longer forms an emoji modifier sequence.
        Some(EmojiModification::EmojiModifier { .. })
            if matches!(presentation_selectors_after_base, [Presentation::Text, ..]) =>
        {
            let [_text, rest @ ..] = presentation_selectors_after_base else {
                unreachable!("guard requires leading text presentation")
            };

            SingletonBaseSelectorOutcome::Deterministic {
                canonical_presentation: Some(Presentation::Text),
                non_canonicality: NonCanonicality::unsanctioned(rest.len()),
            }
        }
        // Precedence 3: with no leading sanctioned FE0E, the modifier attaches
        // to the bare base. Legacy FE0F in that position is the UTS #51
        // defective form and is removed.
        Some(EmojiModification::EmojiModifier { .. }) => {
            let non_canonicality = match presentation_selectors_after_base {
                [] => NonCanonicality::default(),
                [Presentation::Emoji, rest @ ..] => {
                    NonCanonicality::DEFECTIVE + NonCanonicality::unsanctioned(rest.len())
                }
                [Presentation::Text, ..] => {
                    unreachable!("text presentation before modifier is precedence 2")
                }
            };

            SingletonBaseSelectorOutcome::Deterministic {
                canonical_presentation: None,
                non_canonicality,
            }
        }
        // Precedence 4 and 5: tag context keeps emoji-default bases bare and
        // forces emoji presentation for other bases. Precedence 1 above has
        // already guaranteed that the explicit emoji presentation is
        // sanctioned when needed.
        Some(EmojiModification::TagModifier(_)) => {
            let canonical_presentation = if unicode::is_emoji_default(base) {
                None
            } else {
                Some(Presentation::Emoji)
            };
            let canonical = canonical_presentation.as_slice();

            let non_canonicality = if presentation_selectors_after_base == canonical {
                NonCanonicality::default()
            } else if presentation_selectors_after_base.starts_with(canonical) {
                NonCanonicality::unsanctioned(
                    presentation_selectors_after_base.len() - canonical.len(),
                )
            } else {
                let missing_required_selector = if canonical_presentation.is_some() {
                    NonCanonicality::MISSING_REQUIRED
                } else {
                    NonCanonicality::default()
                };

                missing_required_selector
                    + NonCanonicality::unsanctioned(presentation_selectors_after_base.len())
            };

            SingletonBaseSelectorOutcome::Deterministic {
                canonical_presentation,
                non_canonicality,
            }
        }
        // Precedence 6: no fixed-cleanup context remains. Ordinary and
        // keycap-character contexts use the matching policy domain.
        first_modification => analyze_policy_base_selectors(
            base,
            presentation_selectors_after_base,
            matches!(
                first_modification,
                Some(EmojiModification::EnclosingKeycap { .. })
            ),
            policy,
        ),
    }
}

/// Apply ordinary/keycap policy to a singleton base selector run.
///
/// This is Rule 6 of the fixed-cleanup table: all deterministic cleanup
/// contexts have already been ruled out. The function only classifies
/// `presentation_selectors_after_base` under the active policy domain and
/// returns the resulting base selector outcome. It does not handle modification
/// suffix cleanup.
fn analyze_policy_base_selectors(
    base: char,
    presentation_selectors_after_base: &[Presentation],
    is_keycap_context: bool,
    policy: &Policy,
) -> SingletonBaseSelectorOutcome {
    debug_assert!(unicode::has_variation_sequence(base));

    match (
        policy.singleton_rule(base, is_keycap_context),
        presentation_selectors_after_base,
    ) {
        // More than one presentation selector where the first matches the
        // bare-side of the rule: the primary violation is the redundant first
        // selector, and the extras are unsanctioned selector cleanup.
        (SingletonRule::TextToBare, &[Presentation::Text, _, ..])
        | (SingletonRule::EmojiToBare, &[Presentation::Emoji, _, ..]) => {
            SingletonBaseSelectorOutcome::Deterministic {
                canonical_presentation: None,
                non_canonicality: NonCanonicality::new(
                    presentation_selectors_after_base.len() - 1,
                    0,
                    1,
                    0,
                    0,
                ),
            }
        }
        // More than one presentation selector but the first is meaningful
        // under this rule: the first selector is canonical, so the only
        // violation is the unsanctioned selector cleanup after it.
        (_, &[current_presentation, _, ..]) => SingletonBaseSelectorOutcome::Deterministic {
            canonical_presentation: Some(current_presentation),
            non_canonicality: NonCanonicality::unsanctioned(
                presentation_selectors_after_base.len() - 1,
            ),
        },
        // Bare stem under a rule that resolves bare to a concrete presentation:
        // let the caller decide between text and emoji.
        (SingletonRule::BareToEmoji, &[]) => {
            SingletonBaseSelectorOutcome::NeedsPresentationDecision {
                default: Presentation::Emoji,
                non_canonicality: NonCanonicality::RESOLVE,
            }
        }
        (SingletonRule::BareToText, &[]) => {
            SingletonBaseSelectorOutcome::NeedsPresentationDecision {
                default: Presentation::Text,
                non_canonicality: NonCanonicality::RESOLVE,
            }
        }
        // Exactly one presentation selector that matches the bare-side of the
        // rule: the presentation selector is redundant, drop it.
        (SingletonRule::TextToBare, &[Presentation::Text])
        | (SingletonRule::EmojiToBare, &[Presentation::Emoji]) => {
            SingletonBaseSelectorOutcome::Deterministic {
                canonical_presentation: None,
                non_canonicality: NonCanonicality::REDUNDANT,
            }
        }
        // All remaining single-presentation-selector and no-presentation-selector
        // cases are already canonical under the active rule.
        (
            SingletonRule::TextToBare | SingletonRule::BareToText | SingletonRule::BareToEmoji,
            &[Presentation::Emoji],
        ) => SingletonBaseSelectorOutcome::Deterministic {
            canonical_presentation: Some(Presentation::Emoji),
            non_canonicality: NonCanonicality::default(),
        },
        (
            SingletonRule::EmojiToBare | SingletonRule::BareToText | SingletonRule::BareToEmoji,
            &[Presentation::Text],
        ) => SingletonBaseSelectorOutcome::Deterministic {
            canonical_presentation: Some(Presentation::Text),
            non_canonicality: NonCanonicality::default(),
        },
        (SingletonRule::TextToBare | SingletonRule::EmojiToBare, &[]) => {
            SingletonBaseSelectorOutcome::Deterministic {
                canonical_presentation: None,
                non_canonicality: NonCanonicality::default(),
            }
        }
    }
}

// --- Modification selector counting ---

/// Count presentation selectors attached after one modification suffix.
///
/// These selectors are not part of a singleton base selector run. They are
/// always stripped when the surrounding component is rendered.
fn count_selectors_after_modification(m: &EmojiModification) -> usize {
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

/// Count all presentation selectors attached after modification suffixes in
/// one emoji-like component.
///
/// The returned count is added as unsanctioned cleanup by the component-level
/// analyzer after the base selector run has been analyzed.
fn count_selectors_after_modifications(modifications: &[EmojiModification]) -> usize {
    modifications
        .iter()
        .map(count_selectors_after_modification)
        .sum()
}
