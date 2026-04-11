use super::*;
use crate::charset::{CharSet, NamedSetId};
use crate::find_violations;
use crate::scanner::scan;

fn default_policy() -> Policy {
    Policy {
        prefer_bare: CharSet::named(NamedSetId::Ascii),
        bare_as_text: CharSet::named(NamedSetId::Ascii),
    }
}

#[test]
fn test_classify_passthrough() {
    let items = scan("Hello");
    assert_eq!(classify(&items[0], &default_policy()), None);
}

#[test]
fn test_classify_standalone_selector_run() {
    let items = scan("\u{FE0F}");
    assert_eq!(
        classify(&items[0], &default_policy()),
        Some(ViolationKind::IllegalSelector)
    );
}

#[test]
fn test_classify_singleton_canonical() {
    let policy = default_policy();
    // # bare, bare-preferred → canonical
    let items = scan("#");
    assert_eq!(classify(&items[0], &policy), None);
}

#[test]
fn test_classify_singleton_redundant() {
    let policy = default_policy();
    // # + FE0E, bare-preferred, bare side=text → FE0E redundant
    let items = scan("#\u{FE0E}");
    assert_eq!(
        classify(&items[0], &policy),
        Some(ViolationKind::RedundantSelector)
    );
}

#[test]
fn test_classify_singleton_bare_needs_resolution() {
    let policy = default_policy();
    // ©️ bare, not bare-preferred → needs resolution
    let items = scan("\u{00A9}");
    assert_eq!(
        classify(&items[0], &policy),
        Some(ViolationKind::BareNeedsResolution)
    );
}

#[test]
fn test_classify_singleton_with_extra_selector_is_illegal() {
    let policy = default_policy();
    let items = scan("#\u{FE0F}\u{FE0E}");
    assert_eq!(
        classify(&items[0], &policy),
        Some(ViolationKind::IllegalSelector)
    );
}

#[test]
fn test_find_violations_reports_all_non_canonical_items() {
    let policy = default_policy();
    let findings = find_violations("A\u{FE0F} #\u{FE0E} \u{00A9}", &policy);
    assert_eq!(findings.len(), 3);
    assert_eq!(findings[0].violation, ViolationKind::IllegalSelector);
    assert_eq!(findings[0].raw, "\u{FE0F}");
    assert_eq!(findings[0].replacement, "");
    assert_eq!(findings[1].violation, ViolationKind::RedundantSelector);
    assert_eq!(findings[1].replacement, "#");
    assert_eq!(findings[2].violation, ViolationKind::BareNeedsResolution);
    assert_eq!(findings[2].replacement, "\u{00A9}\u{FE0F}");
}

#[test]
fn test_classify_keycap_correct() {
    let items = scan("#\u{FE0F}\u{20E3}");
    assert_eq!(classify(&items[0], &default_policy()), None);
}

#[test]
fn test_classify_keycap_missing_fe0f() {
    let items = scan("#\u{20E3}");
    assert_eq!(
        classify(&items[0], &default_policy()),
        Some(ViolationKind::SequenceSelectorViolation)
    );
}

#[test]
fn test_classify_keycap_wrong_vs() {
    let items = scan("#\u{FE0E}\u{20E3}");
    assert_eq!(
        classify(&items[0], &default_policy()),
        Some(ViolationKind::SequenceSelectorViolation)
    );
}

#[test]
fn test_classify_zwj_correct() {
    // ❤️ FE0F ZWJ 🔥 — ❤️ is text-default with FE0F → correct
    let items = scan("\u{2764}\u{FE0F}\u{200D}\u{1F525}");
    assert_eq!(classify(&items[0], &default_policy()), None);
}

#[test]
fn test_classify_zwj_missing_fe0f() {
    // ❤️ ZWJ 🔥 — ❤️ is text-default, bare → violation
    let items = scan("\u{2764}\u{200D}\u{1F525}");
    assert_eq!(
        classify(&items[0], &default_policy()),
        Some(ViolationKind::SequenceSelectorViolation)
    );
}

#[test]
fn test_classify_zwj_wrong_vs() {
    // ❤️ FE0E ZWJ 🔥 — ❤️ is text-default with FE0E → violation
    let items = scan("\u{2764}\u{FE0E}\u{200D}\u{1F525}");
    assert_eq!(
        classify(&items[0], &default_policy()),
        Some(ViolationKind::SequenceSelectorViolation)
    );
}

#[test]
fn test_classify_zwj_non_eligible_selector_is_violation() {
    // 😀 FE0F ZWJ 🔥 — 😀 has no sanctioned variation sequence → violation
    let items = scan("\u{1F600}\u{FE0F}\u{200D}\u{1F525}");
    assert_eq!(
        classify(&items[0], &default_policy()),
        Some(ViolationKind::SequenceSelectorViolation)
    );
}
