#![no_main]

use evfmt::{Finding, FormatResult, Policy, analyze_scan_item, format_text, scan};
use libfuzzer_sys::fuzz_target;

fn formatted_output(input: &str, policy: &Policy) -> String {
    match format_text(input, policy) {
        FormatResult::Unchanged => input.to_owned(),
        FormatResult::Changed(output) => output,
    }
}

fn assert_finding_length_invariants(finding: &Finding<'_>) {
    let non_canonicality = finding.non_canonicality();
    let replacement = finding.default_canonical_replacement();
    let raw_chars = finding.raw.chars().count() as isize;
    let replacement_chars = replacement.chars().count() as isize;
    let char_delta = replacement_chars - raw_chars;
    let expected_char_delta = non_canonicality.missing_required_selectors as isize
        + non_canonicality.bases_to_resolve as isize
        - non_canonicality.unsanctioned_selectors as isize
        - non_canonicality.defective_sequences as isize
        - non_canonicality.redundant_selectors as isize;

    assert_eq!(
        finding.default_decisions().len(),
        non_canonicality.bases_to_resolve,
        "decision slots must match bases_to_resolve"
    );
    assert_eq!(
        char_delta, expected_char_delta,
        "replacement char delta must match non-canonicality accounting for {finding:?}"
    );
    assert_eq!(
        replacement.len() as isize - finding.raw.len() as isize,
        char_delta * '\u{FE0E}'.len_utf8() as isize,
        "byte delta must be selector-width times char delta"
    );
}

fuzz_target!(|data: &str| {
    let reconstructed: String = scan(data).map(|item| item.raw).collect();
    assert_eq!(reconstructed, data, "scanner must be lossless");

    let policy = Policy::default();
    let first = formatted_output(data, &policy);
    assert_eq!(
        format_text(&first, &policy),
        FormatResult::Unchanged,
        "formatting must be idempotent"
    );

    for item in scan(data) {
        if let Some(finding) = analyze_scan_item(&item, &policy) {
            assert_finding_length_invariants(&finding);
        }
    }
});
