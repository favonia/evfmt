use super::*;
use crate::charset::CharSet;
use crate::formatter::{FormatResult, format_text};
use crate::scanner::{VariationSelector, scan};

fn default_policy() -> Policy {
    Policy::default()
}

fn finding_for(input: &str) -> ReviewFinding<'_> {
    let policy = default_policy();
    let items = scan(input);
    #[allow(clippy::expect_used)]
    review_item(&items[0], &policy).expect("input should produce a finding")
}

#[test]
fn test_review_passthrough() {
    let items = scan("Hello");
    assert_eq!(review_item(&items[0], &default_policy()), None);
}

#[test]
fn test_review_standalone_selector_run() {
    let finding = finding_for("\u{FE0F}");
    assert_eq!(finding.violation(), ViolationKind::IllegalVariationSelector);
    assert_eq!(finding.choices(), [ReplacementDecision::Fix]);
    assert_eq!(finding.default_decision(), ReplacementDecision::Fix);
    assert_eq!(finding.default_replacement(), "");
}

#[test]
fn test_review_singleton_canonical() {
    let policy = default_policy();
    // # bare, bare-preferred -> canonical
    let items = scan("#");
    assert_eq!(review_item(&items[0], &policy), None);
}

#[test]
fn test_review_singleton_redundant() {
    let finding = finding_for("#\u{FE0E}");
    assert_eq!(
        finding.violation(),
        ViolationKind::RedundantVariationSelector
    );
    assert_eq!(finding.choices(), [ReplacementDecision::Fix]);
    assert_eq!(finding.default_decision(), ReplacementDecision::Fix);
    assert_eq!(finding.default_replacement(), "#");
}

#[test]
fn test_review_singleton_bare_needs_resolution() {
    let finding = finding_for("\u{00A9}");
    assert_eq!(finding.violation(), ViolationKind::BareNeedsResolution);
    assert_eq!(
        finding.choices(),
        [ReplacementDecision::Text, ReplacementDecision::Emoji]
    );
    assert_eq!(finding.default_decision(), ReplacementDecision::Emoji);
    assert_eq!(finding.default_replacement(), "\u{00A9}\u{FE0F}");
}

#[test]
fn review_singleton_extra_selector_keeps_meaningful_first_selector() {
    let finding = finding_for("#\u{FE0F}\u{FE0E}");
    assert_eq!(finding.violation(), ViolationKind::IllegalVariationSelector);
    assert_eq!(finding.default_decision(), ReplacementDecision::Fix);
    assert_eq!(finding.default_replacement(), "#\u{FE0F}");
    assert_eq!(
        finding.replacement(ReplacementDecision::Fix),
        Some("#\u{FE0F}")
    );
}

#[test]
fn review_singleton_extra_selector_removes_redundant_first_selector() {
    let finding = finding_for("#\u{FE0E}\u{FE0F}");
    assert_eq!(finding.violation(), ViolationKind::IllegalVariationSelector);
    assert_eq!(finding.replacement(ReplacementDecision::Fix), Some("#"));
}

#[test]
fn render_sequence_fix_returns_none_for_non_sequence_items() {
    let passthrough = ScanItem {
        raw: "A",
        span: 0..1,
        kind: ScanKind::Passthrough,
    };
    assert_eq!(render_sequence_fix(&passthrough), None);

    let standalone = ScanItem {
        raw: "\u{FE0F}",
        span: 0..3,
        kind: ScanKind::StandaloneVariationSelectors(vec![VariationSelector::Emoji]),
    };
    assert_eq!(render_sequence_fix(&standalone), None);

    let singleton = ScanItem {
        raw: "#",
        span: 0..1,
        kind: ScanKind::Singleton {
            base: '#',
            variation_selectors: vec![],
        },
    };
    assert_eq!(render_sequence_fix(&singleton), None);
}

#[test]
fn test_review_text_reports_all_non_canonical_items() {
    let policy = default_policy();
    let findings = review_text("A\u{FE0F} #\u{FE0E} \u{00A9}", &policy);
    assert_eq!(findings.len(), 3);
    assert_eq!(
        findings[0].violation(),
        ViolationKind::IllegalVariationSelector
    );
    assert_eq!(findings[0].raw, "\u{FE0F}");
    assert_eq!(
        findings[1].violation(),
        ViolationKind::RedundantVariationSelector
    );
    assert_eq!(findings[2].violation(), ViolationKind::BareNeedsResolution);
}

#[test]
fn applying_review_default_matches_formatter() {
    let policy = default_policy();
    let input = "#\u{FE0E}\u{20E3}\u{2764}\u{200D}\u{1F525}A\u{FE0F}";
    let items = scan(input);
    let rebuilt: String = items
        .iter()
        .map(|item| {
            if let Some(finding) = review_item(item, &policy) {
                finding.default_replacement().to_owned()
            } else {
                item.raw.to_owned()
            }
        })
        .collect();

    let expected = match format_text(input, &policy) {
        FormatResult::Changed(output) => output,
        FormatResult::Unchanged => input.to_owned(),
    };

    assert_eq!(rebuilt, expected);
}

#[test]
fn review_text_and_emoji_decisions_apply_without_policy() {
    let policy = default_policy();
    let items = scan("\u{00A9}");
    #[allow(clippy::expect_used)]
    let finding = review_item(&items[0], &policy).expect("bare copyright should need resolution");

    assert_eq!(
        finding.replacement(ReplacementDecision::Text),
        Some("\u{00A9}\u{FE0E}")
    );
    assert_eq!(
        finding.replacement(ReplacementDecision::Emoji),
        Some("\u{00A9}\u{FE0F}")
    );
}

#[test]
fn review_finding_carries_precomputed_replacements() {
    let policy = default_policy();

    let items = scan("#\u{20E3}");
    #[allow(clippy::expect_used)]
    let keycap = review_item(&items[0], &policy).expect("bare keycap should need repair");
    assert_eq!(
        keycap.replacement(ReplacementDecision::Fix),
        Some("#\u{FE0F}\u{20E3}")
    );
    assert_eq!(keycap.replacement(ReplacementDecision::Text), None);

    let items = scan("\u{2764}\u{200D}\u{1F525}");
    #[allow(clippy::expect_used)]
    let zwj = review_item(&items[0], &policy).expect("bare heart ZWJ should need repair");
    assert_eq!(
        zwj.replacement(ReplacementDecision::Fix),
        Some("\u{2764}\u{FE0F}\u{200D}\u{1F525}")
    );

    let items = scan("\u{00A9}");
    #[allow(clippy::expect_used)]
    let singleton = review_item(&items[0], &policy).expect("bare copyright should need resolution");
    assert_eq!(
        singleton.replacement(ReplacementDecision::Text),
        Some("\u{00A9}\u{FE0E}")
    );
    assert_eq!(
        singleton.replacement(ReplacementDecision::Emoji),
        Some("\u{00A9}\u{FE0F}")
    );
    assert_eq!(singleton.raw, "\u{00A9}");
    assert_eq!(singleton.replacement(ReplacementDecision::Fix), None);
}

#[test]
fn test_review_keycap_correct() {
    let items = scan("#\u{FE0F}\u{20E3}");
    assert_eq!(review_item(&items[0], &default_policy()), None);
}

#[test]
fn test_review_keycap_missing_fe0f() {
    let finding = finding_for("#\u{20E3}");
    assert_eq!(
        finding.violation(),
        ViolationKind::NotFullyQualifiedEmojiSequence
    );
}

#[test]
fn test_review_keycap_wrong_vs() {
    let finding = finding_for("#\u{FE0E}\u{20E3}");
    assert_eq!(
        finding.violation(),
        ViolationKind::NotFullyQualifiedEmojiSequence
    );
}

#[test]
fn test_review_zwj_correct() {
    // heart FE0F ZWJ fire: heart is text-default with FE0F -> correct
    let items = scan("\u{2764}\u{FE0F}\u{200D}\u{1F525}");
    assert_eq!(review_item(&items[0], &default_policy()), None);
}

#[test]
fn test_review_zwj_missing_fe0f() {
    let finding = finding_for("\u{2764}\u{200D}\u{1F525}");
    assert_eq!(
        finding.violation(),
        ViolationKind::NotFullyQualifiedEmojiSequence
    );
}

#[test]
fn test_review_zwj_wrong_vs() {
    let finding = finding_for("\u{2764}\u{FE0E}\u{200D}\u{1F525}");
    assert_eq!(
        finding.violation(),
        ViolationKind::NotFullyQualifiedEmojiSequence
    );
}

#[test]
fn test_review_zwj_extra_terminal_selector_is_violation() {
    let finding = finding_for("\u{2764}\u{FE0F}\u{FE0E}\u{200D}\u{1F525}");
    assert_eq!(
        finding.violation(),
        ViolationKind::NotFullyQualifiedEmojiSequence
    );
    assert_eq!(
        finding.replacement(ReplacementDecision::Fix),
        Some("\u{2764}\u{FE0F}\u{200D}\u{1F525}")
    );
}

#[test]
fn test_review_zwj_non_eligible_selector_is_violation() {
    let finding = finding_for("\u{1F600}\u{FE0F}\u{200D}\u{1F525}");
    assert_eq!(
        finding.violation(),
        ViolationKind::NotFullyQualifiedEmojiSequence
    );
}

#[test]
fn review_bare_as_text_policy_defaults_to_text() {
    // \u{00A9} (copyright) has variation sequences and is emoji-default.
    // With bare_as_text containing it (but not prefer_bare), the policy
    // yields BareToText, so the finding should default to Text.
    let policy = Policy::default()
        .with_prefer_bare(CharSet::none())
        .with_bare_as_text(CharSet::all());
    let items = scan("\u{00A9}");
    #[allow(clippy::expect_used)]
    let finding = review_item(&items[0], &policy).expect("bare copyright should need resolution");
    assert_eq!(finding.violation(), ViolationKind::BareNeedsResolution);
    assert_eq!(finding.default_decision(), ReplacementDecision::Text);
    assert_eq!(finding.default_replacement(), "\u{00A9}\u{FE0E}");
}

#[test]
fn review_singleton_without_variation_sequence_is_illegal() {
    // 'A' has no sanctioned variation sequences. If a ScanItem arrives as
    // Singleton with a variation selector, review should flag it as illegal.
    let item = ScanItem {
        raw: "A\u{FE0F}",
        span: 0..4,
        kind: ScanKind::Singleton {
            base: 'A',
            variation_selectors: vec![VariationSelector::Emoji],
        },
    };
    let policy = default_policy();
    #[allow(clippy::expect_used)]
    let finding = review_item(&item, &policy).expect("should produce a finding");
    assert_eq!(finding.violation(), ViolationKind::IllegalVariationSelector);
    assert_eq!(finding.default_decision(), ReplacementDecision::Fix);
    assert_eq!(finding.default_replacement(), "A");
}
