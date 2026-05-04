use super::*;
use crate::analysis::{Finding, analyze_scan_item};
use crate::scanner::scan;
use crate::unicode;
use crate::variation_set::VariationSet;
use proptest::prelude::*;

fn default_policy() -> Policy {
    Policy::default()
}

fn bool_variation_set(matches: bool) -> VariationSet {
    if matches {
        VariationSet::all()
    } else {
        VariationSet::none()
    }
}

fn formatted_output(input: &str, policy: &Policy) -> String {
    match format_text(input, policy) {
        FormatResult::Unchanged => input.to_owned(),
        FormatResult::Changed(output) => output,
    }
}

#[test]
fn plain_ascii_is_unchanged() {
    assert_eq!(
        format_text("Hello, world!", &default_policy()),
        FormatResult::Unchanged
    );
}

#[test]
fn standalone_ascii_default_policy_prefers_bare_text_side() {
    let policy = default_policy();

    assert_eq!(format_text("#", &policy), FormatResult::Unchanged);
    assert_eq!(
        format_text("#\u{FE0E}", &policy),
        FormatResult::Changed("#".to_owned())
    );
    assert_eq!(format_text("#\u{FE0F}", &policy), FormatResult::Unchanged);
}

#[test]
fn standalone_text_default_non_ascii_default_policy_resolves_bare_to_emoji() {
    let policy = default_policy();

    assert_eq!(
        format_text("\u{00A9}", &policy),
        FormatResult::Changed("\u{00A9}\u{FE0F}".to_owned())
    );
    assert_eq!(
        format_text("\u{00A9}\u{FE0E}", &policy),
        FormatResult::Unchanged
    );
    assert_eq!(
        format_text("\u{00A9}\u{FE0F}", &policy),
        FormatResult::Unchanged
    );
}

#[test]
fn standalone_emoji_default_non_ascii_default_policy_prefers_bare_emoji_side() {
    let policy = default_policy();

    assert_eq!(format_text("\u{2728}", &policy), FormatResult::Unchanged);
    assert_eq!(
        format_text("\u{2728}\u{FE0E}", &policy),
        FormatResult::Unchanged
    );
    assert_eq!(
        format_text("\u{2728}\u{FE0F}", &policy),
        FormatResult::Changed("\u{2728}".to_owned())
    );
}

#[test]
fn unsanctioned_selectors_are_removed() {
    let policy = default_policy();

    assert_eq!(
        format_text("A\u{FE0F}", &policy),
        FormatResult::Changed("A".to_owned())
    );
    assert_eq!(
        format_text("\u{FE0F}hello", &policy),
        FormatResult::Changed("hello".to_owned())
    );
}

#[test]
fn extra_selector_after_meaningful_selector_is_removed() {
    assert_eq!(
        format_text("#\u{FE0F}\u{FE0E}", &default_policy()),
        FormatResult::Changed("#\u{FE0F}".to_owned())
    );
}

#[test]
fn keycap_cleanup_follows_current_sequence_contract() {
    let policy = default_policy();

    assert_eq!(
        format_text("#\u{FE0F}\u{20E3}", &policy),
        FormatResult::Unchanged
    );
    assert_eq!(
        format_text("#\u{20E3}", &policy),
        FormatResult::Changed("#\u{FE0E}\u{20E3}".to_owned())
    );
    // ZWJ context no longer overrides the component's own keycap policy.
    assert_eq!(
        format_text("#\u{FE0E}\u{20E3}", &policy),
        FormatResult::Unchanged
    );
    assert_eq!(
        format_text("#\u{FE0E}\u{20E3}\u{200D}\u{1F525}", &policy),
        FormatResult::Unchanged
    );
    assert_eq!(
        format_text("\u{26A0}\u{20E3}", &policy),
        FormatResult::Changed("\u{26A0}\u{FE0E}\u{20E3}".to_owned())
    );
    assert_eq!(
        format_text("#\u{20E3}\u{FE0F}", &policy),
        FormatResult::Changed("#\u{FE0E}\u{20E3}".to_owned())
    );
}

#[test]
fn keycap_policy_can_select_bare_text_or_emoji_outputs() {
    let emoji_policy = Policy::default().with_bare_as_text(VariationSet::none());
    assert_eq!(
        format_text("#\u{20E3}", &emoji_policy),
        FormatResult::Changed("#\u{FE0F}\u{20E3}".to_owned())
    );

    let bare_text_policy = Policy::default()
        .with_prefer_bare(crate::variation_set::KEYCAP_CHARS)
        .with_bare_as_text(crate::variation_set::KEYCAP_CHARS);
    assert_eq!(
        format_text("#\u{FE0E}\u{20E3}", &bare_text_policy),
        FormatResult::Changed("#\u{20E3}".to_owned())
    );
    assert_eq!(
        format_text("#\u{20E3}", &bare_text_policy),
        FormatResult::Unchanged
    );
}

#[test]
fn keycap_policy_uses_first_modification_only() {
    let policy = default_policy();

    assert_eq!(
        format_text("#\u{20E3}\u{1F3FB}", &policy),
        FormatResult::Changed("#\u{FE0E}\u{20E3}\u{1F3FB}".to_owned())
    );
    assert_eq!(
        format_text("#\u{20E3}\u{1F3FB}\u{FE0F}", &policy),
        FormatResult::Changed("#\u{FE0E}\u{20E3}\u{1F3FB}".to_owned())
    );
}

#[test]
fn modifier_context_preserves_sanctioned_text_presentation() {
    let policy = default_policy();

    assert_eq!(
        format_text("\u{270C}\u{FE0E}\u{1F3FB}", &policy),
        FormatResult::Unchanged
    );
    assert_eq!(
        format_text("\u{270C}\u{FE0F}\u{1F3FB}", &policy),
        FormatResult::Changed("\u{270C}\u{1F3FB}".to_owned())
    );
}

#[test]
fn zwj_cleanup_uses_component_local_rules() {
    let policy = default_policy();

    assert_eq!(
        format_text("\u{2764}\u{200D}\u{1F525}", &policy),
        FormatResult::Changed("\u{2764}\u{FE0F}\u{200D}\u{1F525}".to_owned())
    );
    assert_eq!(
        format_text("\u{2764}\u{FE0F}\u{200D}\u{1F525}", &policy),
        FormatResult::Unchanged
    );
    assert_eq!(
        format_text("\u{2764}\u{FE0E}\u{200D}\u{1F525}", &policy),
        FormatResult::Unchanged
    );
    assert_eq!(
        format_text("\u{1F600}\u{FE0F}\u{200D}\u{1F525}", &policy),
        FormatResult::Changed("\u{1F600}\u{200D}\u{1F525}".to_owned())
    );
}

#[test]
fn mixed_content_formats_only_structural_items() {
    let policy = default_policy();

    assert_eq!(
        format_text("Press # for \u{00A9}", &policy),
        FormatResult::Changed("Press # for \u{00A9}\u{FE0F}".to_owned())
    );
}

#[derive(Debug, Clone, Copy)]
enum InputSelector {
    None,
    Text,
    Emoji,
}

#[derive(Debug, Clone, Copy)]
struct PolicyFlags {
    prefer_bare: bool,
    bare_as_text: bool,
}

fn build_input(ch: char, selector: InputSelector) -> String {
    let mut input = String::new();
    input.push(ch);
    match selector {
        InputSelector::None => {}
        InputSelector::Text => input.push(unicode::TEXT_PRESENTATION_SELECTOR),
        InputSelector::Emoji => input.push(unicode::EMOJI_PRESENTATION_SELECTOR),
    }
    input
}

fn expected_standalone(ch: char, selector: InputSelector, policy: PolicyFlags) -> String {
    let mut output = String::new();
    output.push(ch);

    match (policy.prefer_bare, policy.bare_as_text, selector) {
        (true, true, InputSelector::Text)
        | (true, false, InputSelector::Emoji)
        | (true, _, InputSelector::None) => {}
        (false, true, InputSelector::None) | (_, _, InputSelector::Text) => {
            output.push(unicode::TEXT_PRESENTATION_SELECTOR);
        }
        (false, false, InputSelector::None) | (_, _, InputSelector::Emoji) => {
            output.push(unicode::EMOJI_PRESENTATION_SELECTOR);
        }
    }

    output
}

#[test]
fn exhaustive_standalone_variation_sequence_policy_table() {
    let policies = [
        PolicyFlags {
            prefer_bare: false,
            bare_as_text: false,
        },
        PolicyFlags {
            prefer_bare: false,
            bare_as_text: true,
        },
        PolicyFlags {
            prefer_bare: true,
            bare_as_text: false,
        },
        PolicyFlags {
            prefer_bare: true,
            bare_as_text: true,
        },
    ];
    let selectors = [
        InputSelector::None,
        InputSelector::Text,
        InputSelector::Emoji,
    ];

    for ch in unicode::variation_sequence_chars() {
        for policy_flags in policies {
            let policy = Policy::default()
                .with_prefer_bare(bool_variation_set(policy_flags.prefer_bare))
                .with_bare_as_text(bool_variation_set(policy_flags.bare_as_text));

            for selector in selectors {
                let input = build_input(ch, selector);
                let actual = formatted_output(&input, &policy);
                let expected = expected_standalone(ch, selector, policy_flags);

                assert_eq!(
                    actual, expected,
                    "U+{:04X} selector={selector:?} policy={policy_flags:?}",
                    ch as u32
                );
            }
        }
    }
}

fn interesting_string_strategy() -> impl Strategy<Value = String> {
    let token = prop_oneof![
        3 => prop::sample::select(vec![
            '\u{0023}', '\u{002A}', '\u{0030}', '\u{00A9}', '\u{00AE}',
            '\u{203C}', '\u{2049}', '\u{2122}', '\u{2139}', '\u{2764}',
        ]),
        3 => prop::sample::select(vec![
            '\u{231A}', '\u{2728}', '\u{2614}', '\u{26A1}', '\u{2705}',
            '\u{270A}', '\u{2B50}', '\u{1F004}', '\u{1F600}',
            '\u{1F468}', '\u{1F466}', '\u{1F525}',
        ]),
        4 => prop::sample::select(vec![
            unicode::TEXT_PRESENTATION_SELECTOR,
            unicode::EMOJI_PRESENTATION_SELECTOR,
        ]),
        3 => prop::sample::select(vec![unicode::ZWJ, unicode::COMBINING_ENCLOSING_KEYCAP]),
        1 => prop::sample::select(vec![
            '\u{1F3FB}', '\u{1F3FC}', '\u{1F3FD}', '\u{1F3FE}', '\u{1F3FF}',
        ]),
        2 => prop::sample::select(vec![
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
            'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T',
            'U', 'V', 'W', 'X', 'Y', 'Z',
        ]),
        1 => prop::sample::select(vec!['\n', ' ']),
    ];

    prop::collection::vec(token, 0..40).prop_map(|chars| chars.into_iter().collect())
}

fn policy_strategy() -> impl Strategy<Value = Policy> {
    (prop::bool::ANY, prop::bool::ANY).prop_map(|(prefer_bare, bare_as_text)| {
        Policy::default()
            .with_prefer_bare(bool_variation_set(prefer_bare))
            .with_bare_as_text(bool_variation_set(bare_as_text))
    })
}

fn strip_selectors(s: &str) -> String {
    s.chars()
        .filter(|&ch| {
            ch != unicode::TEXT_PRESENTATION_SELECTOR && ch != unicode::EMOJI_PRESENTATION_SELECTOR
        })
        .collect()
}

fn check_finding_length_invariants(finding: &Finding<'_>) -> Result<(), TestCaseError> {
    let non_canonicality = finding.non_canonicality();
    let replacement = finding.default_canonical_replacement();
    let removed_chars = non_canonicality.unsanctioned_selectors
        + non_canonicality.defective_sequences
        + non_canonicality.redundant_selectors;
    let inserted_chars =
        non_canonicality.missing_required_selectors + non_canonicality.bases_to_resolve;
    // The byte-length invariant below counts selector insertions and removals
    // with one shared byte width. Keep that assumption explicit so it fails
    // near the accounting if the selector constants ever change.
    let selector_len = unicode::TEXT_PRESENTATION_SELECTOR.len_utf8();

    prop_assert_eq!(
        selector_len,
        unicode::EMOJI_PRESENTATION_SELECTOR.len_utf8()
    );

    prop_assert_eq!(
        finding.default_decisions().len(),
        non_canonicality.bases_to_resolve
    );
    prop_assert_eq!(
        replacement.chars().count() + removed_chars,
        finding.raw.chars().count() + inserted_chars,
        "replacement char delta must match non-canonicality accounting for {:?}",
        finding
    );
    prop_assert_eq!(
        replacement.len() + removed_chars * selector_len,
        finding.raw.len() + inserted_chars * selector_len,
        "byte delta must be selector-width times char delta for {:?}",
        finding
    );

    Ok(())
}

proptest! {
    #[test]
    fn prop_idempotent(input in interesting_string_strategy(), policy in policy_strategy()) {
        let first = formatted_output(&input, &policy);
        prop_assert_eq!(format_text(&first, &policy), FormatResult::Unchanged);
    }

    #[test]
    fn prop_no_analysis_findings_in_output(
        input in interesting_string_strategy(),
        policy in policy_strategy(),
    ) {
        let output = formatted_output(&input, &policy);
        for item in scan(&output) {
            if let Some(finding) = analyze_scan_item(&item, &policy) {
                prop_assert!(
                    false,
                    "finding remains after formatting: {finding:?} for item {item:?}"
                );
            }
        }
    }

    #[test]
    fn prop_only_modifies_selectors(input in interesting_string_strategy(), policy in policy_strategy()) {
        let output = formatted_output(&input, &policy);
        prop_assert_eq!(strip_selectors(&input), strip_selectors(&output));
    }

    #[test]
    fn prop_finding_lengths_match_non_canonicality_counts(
        input in interesting_string_strategy(),
        policy in policy_strategy(),
    ) {
        for item in scan(&input) {
            if let Some(finding) = analyze_scan_item(&item, &policy) {
                check_finding_length_invariants(&finding)?;
            }
        }
    }

}
