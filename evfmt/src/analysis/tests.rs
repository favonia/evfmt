use super::*;
use crate::presentation::Presentation;
use crate::scanner::scan;

fn default_policy() -> Policy {
    Policy::default()
}

fn finding_for_first_item(input: &str) -> Finding<'_> {
    let items: Vec<_> = scan(input).collect();
    #[allow(clippy::expect_used)]
    // Tests should fail at the missing finding, not at a later assertion.
    analyze_scan_item(&items[0], &default_policy()).expect("first item should produce a finding")
}

fn no_finding_for_first_item(input: &str) {
    let items: Vec<_> = scan(input).collect();
    assert_eq!(analyze_scan_item(&items[0], &default_policy()), None);
}

fn finding_for_first_item_with_policy<'a>(input: &'a str, policy: &Policy) -> Finding<'a> {
    let items: Vec<_> = scan(input).collect();
    #[allow(clippy::expect_used)]
    // Tests should fail at the missing finding, not at a later assertion.
    analyze_scan_item(&items[0], policy).expect("first item should produce a finding")
}

const fn non_canonicality(
    unsanctioned_selectors: usize,
    defective_sequences: usize,
    redundant_selectors: usize,
    missing_required_selectors: usize,
    bases_to_resolve: usize,
) -> NonCanonicality {
    NonCanonicality::new(
        unsanctioned_selectors,
        defective_sequences,
        redundant_selectors,
        missing_required_selectors,
        bases_to_resolve,
    )
}

fn default_choice_decisions(finding: &Finding<'_>) -> Vec<Presentation> {
    finding.default_decisions().collect()
}

#[test]
fn links_only_zwj_sequence_strips_link_selectors() {
    let finding = finding_for_first_item("\u{200D}\u{FE0F}\u{200D}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
    assert_eq!(finding.default_canonical_replacement(), "\u{200D}\u{200D}");
}

#[test]
fn links_only_zwj_sequence_without_selectors_is_canonical() {
    no_finding_for_first_item("\u{200D}\u{200D}");
}

#[test]
fn fixed_repair_has_empty_decision_vector() {
    let finding = finding_for_first_item("#\u{FE0E}");
    assert_eq!(finding.non_canonicality(), non_canonicality(0, 0, 1, 0, 0));
    assert!(default_choice_decisions(&finding).is_empty());
    assert_eq!(finding.default_canonical_replacement(), "#");
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[]),
        Some("#".to_owned())
    );
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Text]),
        None
    );
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Emoji]),
        None
    );
}

#[test]
fn unsanctioned_singleton_context_cleans_base_and_modification_selectors() {
    let finding = finding_for_first_item("\u{1F600}\u{FE0F}\u{20E3}\u{FE0E}");
    assert_eq!(finding.non_canonicality(), non_canonicality(2, 0, 0, 0, 0));
    assert_eq!(finding.default_canonical_replacement(), "\u{1F600}\u{20E3}");
    assert_eq!(finding.default_decisions().len(), 0);
}

#[test]
fn standalone_bare_singleton_uses_plain_presentation_resolution() {
    let finding = finding_for_first_item("\u{00A9}");
    assert_eq!(finding.non_canonicality(), non_canonicality(0, 0, 0, 0, 1));
    assert_eq!(default_choice_decisions(&finding), [Presentation::Emoji]);
    assert_eq!(finding.default_canonical_replacement(), "\u{00A9}\u{FE0F}");
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Text]),
        Some("\u{00A9}\u{FE0E}".to_owned())
    );
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Emoji]),
        Some("\u{00A9}\u{FE0F}".to_owned())
    );
    assert_eq!(finding.canonical_replacement_with_decisions(&[]), None);
}

#[test]
fn standalone_bare_singleton_can_default_to_text_resolution() {
    let policy = Policy::default()
        .with_prefer_bare(crate::variation_set::VariationSet::none())
        .with_bare_as_text(crate::variation_set::VariationSet::all());
    let finding = finding_for_first_item_with_policy("\u{00A9}", &policy);
    assert_eq!(default_choice_decisions(&finding), [Presentation::Text]);
    assert_eq!(finding.default_canonical_replacement(), "\u{00A9}\u{FE0E}");
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Emoji]),
        Some("\u{00A9}\u{FE0F}".to_owned())
    );
}

#[test]
fn flag_without_selectors_is_canonical() {
    no_finding_for_first_item("\u{1F1E6}\u{1F1E8}");
}

#[test]
fn flag_selector_on_either_indicator_is_removed() {
    for input in ["\u{1F1E6}\u{FE0F}\u{1F1E8}", "\u{1F1E6}\u{1F1E8}\u{FE0E}"] {
        let finding = finding_for_first_item(input);
        assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
        assert_eq!(
            finding.default_canonical_replacement(),
            "\u{1F1E6}\u{1F1E8}"
        );
    }
}

#[test]
fn flag_finding_is_created_for_each_selector_source_independently() {
    let cases = [
        ("\u{1F1E6}\u{FE0F}\u{1F1E8}", "\u{1F1E6}\u{1F1E8}"),
        ("\u{1F1E6}\u{1F1E8}\u{FE0E}", "\u{1F1E6}\u{1F1E8}"),
        (
            "\u{1F1E6}\u{1F1E8}\u{1F3FB}\u{FE0F}",
            "\u{1F1E6}\u{1F1E8}\u{1F3FB}",
        ),
        (
            "\u{1F1E6}\u{1F1E8}\u{200D}\u{FE0F}",
            "\u{1F1E6}\u{1F1E8}\u{200D}",
        ),
    ];

    for (input, replacement) in cases {
        let finding = finding_for_first_item(input);
        assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
        assert_eq!(finding.default_canonical_replacement(), replacement);
    }
}

#[test]
fn flag_without_any_selector_source_has_no_finding() {
    no_finding_for_first_item("\u{1F1E6}\u{1F1E8}");
}

#[test]
fn single_emoji_zwj_wrapper_uses_singleton_resolution_but_preserves_link() {
    let finding = finding_for_first_item("\u{00A9}\u{200D}");
    assert_eq!(finding.non_canonicality(), non_canonicality(0, 0, 0, 0, 1));
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Text]),
        Some("\u{00A9}\u{FE0E}\u{200D}".to_owned())
    );
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Emoji]),
        Some("\u{00A9}\u{FE0F}\u{200D}".to_owned())
    );
}

#[test]
fn single_emoji_keycap_wrapper_keeps_singleton_text_keycap_semantics() {
    let items: Vec<_> = scan("#\u{FE0E}\u{20E3}\u{200D}").collect();
    assert_eq!(analyze_scan_item(&items[0], &default_policy()), None);
}

#[test]
fn single_emoji_keycap_wrapper_repairs_without_dropping_link() {
    let finding = finding_for_first_item("#\u{20E3}\u{200D}");
    assert_eq!(finding.non_canonicality(), non_canonicality(0, 0, 0, 0, 1));
    assert_eq!(
        finding.default_canonical_replacement(),
        "#\u{FE0E}\u{20E3}\u{200D}"
    );
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Emoji]),
        Some("#\u{FE0F}\u{20E3}\u{200D}".to_owned())
    );
}

#[test]
fn single_emoji_keycap_wrapper_reports_trailing_link_selector_cleanup() {
    let finding = finding_for_first_item("#\u{20E3}\u{200D}\u{FE0F}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 1));
    assert_eq!(
        finding.default_canonical_replacement(),
        "#\u{FE0E}\u{20E3}\u{200D}"
    );
}

#[test]
fn tag_modifier_trailing_selector_is_cleaned() {
    let finding = finding_for_first_item("\u{1F3F4}\u{E0067}\u{FE0F}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{1F3F4}\u{E0067}"
    );
}

#[test]
fn tag_modifier_on_emoji_default_base_does_not_add_base_selector() {
    let finding = finding_for_first_item("\u{2728}\u{E0067}\u{FE0F}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
    assert_eq!(finding.default_canonical_replacement(), "\u{2728}\u{E0067}");
}

#[test]
fn emoji_modifier_legacy_emoji_selector_is_defective() {
    let finding = finding_for_first_item("\u{270C}\u{FE0F}\u{1F3FB}");
    assert_eq!(finding.non_canonicality(), non_canonicality(0, 1, 0, 0, 0));
    assert_eq!(finding.default_canonical_replacement(), "\u{270C}\u{1F3FB}");
}

#[test]
fn emoji_modifier_extra_selector_after_legacy_emoji_selector_is_unsanctioned() {
    let finding = finding_for_first_item("\u{270C}\u{FE0F}\u{FE0E}\u{1F3FB}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 1, 0, 0, 0));
    assert_eq!(finding.default_canonical_replacement(), "\u{270C}\u{1F3FB}");
}

#[test]
fn emoji_modifier_sanctioned_text_selector_is_preserved() {
    no_finding_for_first_item("\u{270C}\u{FE0E}\u{1F3FB}");
}

#[test]
fn emoji_modifier_extra_selector_after_text_selector_is_unsanctioned() {
    let finding = finding_for_first_item("\u{270C}\u{FE0E}\u{FE0F}\u{1F3FB}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{270C}\u{FE0E}\u{1F3FB}"
    );
}

#[test]
fn tag_modifier_missing_emoji_selector_is_counted() {
    let finding = finding_for_first_item("\u{00A9}\u{E0067}");
    assert_eq!(finding.non_canonicality(), non_canonicality(0, 0, 0, 1, 0));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{00A9}\u{FE0F}\u{E0067}"
    );
}

#[test]
fn tag_modifier_wrong_base_presentation_counts_cleanup_and_missing_selector() {
    let finding = finding_for_first_item("\u{00A9}\u{FE0E}\u{E0067}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 1, 0));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{00A9}\u{FE0F}\u{E0067}"
    );
}

#[test]
fn tag_modifier_extra_selectors_after_wrong_presentation_are_unsanctioned() {
    let finding = finding_for_first_item("\u{00A9}\u{FE0E}\u{FE0E}\u{E0067}");
    assert_eq!(finding.non_canonicality(), non_canonicality(2, 0, 0, 1, 0));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{00A9}\u{FE0F}\u{E0067}"
    );
}

#[test]
fn multi_emoji_zwj_sequence_resolves_bare_components_with_component_policy() {
    let finding = finding_for_first_item("\u{2764}\u{200D}\u{1F525}");
    assert_eq!(finding.non_canonicality(), non_canonicality(0, 0, 0, 0, 1));
    assert_eq!(default_choice_decisions(&finding), [Presentation::Emoji]);
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Text]),
        Some("\u{2764}\u{FE0E}\u{200D}\u{1F525}".to_owned())
    );
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{2764}\u{FE0F}\u{200D}\u{1F525}"
    );
}

#[test]
fn multi_emoji_zwj_sequence_exposes_multiple_component_decision_slots() {
    let finding = finding_for_first_item("\u{00A9}\u{200D}\u{00AE}");
    assert_eq!(finding.non_canonicality(), non_canonicality(0, 0, 0, 0, 2));
    assert_eq!(
        default_choice_decisions(&finding),
        [Presentation::Emoji, Presentation::Emoji]
    );
    assert_eq!(
        finding.canonical_replacement_with_decisions(&[Presentation::Text, Presentation::Emoji]),
        Some("\u{00A9}\u{FE0E}\u{200D}\u{00AE}\u{FE0F}".to_owned())
    );
}

#[test]
fn multi_emoji_zwj_sequence_keeps_mixed_component_non_canonicality_counts() {
    let finding = finding_for_first_item("\u{1F1E6}\u{FE0F}\u{1F1E8}\u{200D}\u{00A9}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 1));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{1F1E6}\u{1F1E8}\u{200D}\u{00A9}\u{FE0F}"
    );
}

#[test]
fn multi_emoji_zwj_sequence_repairs_noncanonical_joined_component_by_policy() {
    let finding = finding_for_first_item("\u{1F525}\u{200D}\u{2764}");
    assert_eq!(finding.non_canonicality(), non_canonicality(0, 0, 0, 0, 1));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{1F525}\u{200D}\u{2764}\u{FE0F}"
    );
}

#[test]
fn multi_emoji_zwj_sequence_keeps_explicit_text_component_request() {
    no_finding_for_first_item("\u{2764}\u{FE0E}\u{200D}\u{1F525}");
}

#[test]
fn multi_emoji_zwj_sequence_removes_unsupported_component_selector() {
    let finding = finding_for_first_item("\u{1F600}\u{FE0F}\u{200D}\u{1F525}");
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{1F600}\u{200D}\u{1F525}"
    );
}

#[test]
fn multi_emoji_zwj_sequence_cleans_joined_link_selector_without_component_repair() {
    let finding = finding_for_first_item("\u{1F525}\u{200D}\u{FE0F}\u{1F600}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{1F525}\u{200D}\u{1F600}"
    );
}

#[test]
fn multi_emoji_zwj_sequence_cleans_trailing_link_selector_without_component_repair() {
    let finding = finding_for_first_item("\u{1F525}\u{200D}\u{1F600}\u{200D}\u{FE0F}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{1F525}\u{200D}\u{1F600}\u{200D}"
    );
}

#[test]
fn multi_emoji_zwj_sequence_repairs_flag_component_selectors() {
    for (input, replacement) in [
        (
            "\u{1F1E6}\u{FE0F}\u{1F1E8}\u{200D}\u{1F525}",
            "\u{1F1E6}\u{1F1E8}\u{200D}\u{1F525}",
        ),
        (
            "\u{1F1E6}\u{1F1E8}\u{FE0E}\u{200D}\u{1F525}",
            "\u{1F1E6}\u{1F1E8}\u{200D}\u{1F525}",
        ),
        (
            "\u{1F1E6}\u{1F1E8}\u{1F3FB}\u{FE0F}\u{200D}\u{1F525}",
            "\u{1F1E6}\u{1F1E8}\u{1F3FB}\u{200D}\u{1F525}",
        ),
    ] {
        let finding = finding_for_first_item(input);
        assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
        assert_eq!(finding.default_canonical_replacement(), replacement);
    }
}

#[test]
fn multi_emoji_zwj_keycap_component_uses_component_policy() {
    no_finding_for_first_item("#\u{FE0E}\u{20E3}\u{200D}\u{1F525}");
}

#[test]
fn zwj_flag_component_without_selectors_is_canonical() {
    no_finding_for_first_item("\u{1F1E6}\u{1F1E8}\u{200D}\u{1F525}");
}

#[test]
fn combo_leading_zwj_run_does_not_attach_to_following_emoji() {
    let items: Vec<_> = scan("\u{200D}\u{FE0F}\u{00A9}").collect();
    assert_eq!(items.len(), 2);

    #[allow(clippy::expect_used)] // This test fixture is non-canonical by construction.
    let link_finding =
        analyze_scan_item(&items[0], &default_policy()).expect("link selector should be repaired");
    assert_eq!(link_finding.default_canonical_replacement(), "\u{200D}");

    #[allow(clippy::expect_used)] // This test fixture is non-canonical by construction.
    let emoji_finding =
        analyze_scan_item(&items[1], &default_policy()).expect("bare copyright still uses policy");
    assert_eq!(
        emoji_finding.non_canonicality(),
        non_canonicality(0, 0, 0, 0, 1)
    );
    assert_eq!(
        emoji_finding.default_canonical_replacement(),
        "\u{00A9}\u{FE0F}"
    );
}

#[test]
fn combo_dangling_zwj_after_one_emoji_uses_singleton_policy() {
    let finding = finding_for_first_item("\u{00A9}\u{200D}\u{FE0F}\u{200D}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 1));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{00A9}\u{FE0F}\u{200D}\u{200D}"
    );
}

#[test]
fn combo_dangling_zwj_after_canonical_singleton_only_cleans_link_selectors() {
    let finding = finding_for_first_item("\u{00A9}\u{FE0F}\u{200D}\u{FE0F}");
    assert_eq!(finding.non_canonicality(), non_canonicality(1, 0, 0, 0, 0));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{00A9}\u{FE0F}\u{200D}"
    );
}

#[test]
fn combo_true_zwj_sequence_uses_component_local_cleanup() {
    let finding =
        finding_for_first_item("\u{2764}\u{FE0E}\u{200D}\u{FE0F}\u{1F525}\u{FE0F}\u{200D}\u{FE0E}");
    assert_eq!(finding.non_canonicality(), non_canonicality(3, 0, 0, 0, 0));
    assert_eq!(
        finding.default_canonical_replacement(),
        "\u{2764}\u{FE0E}\u{200D}\u{1F525}\u{200D}"
    );
}
