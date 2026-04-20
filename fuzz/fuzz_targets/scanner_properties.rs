#![no_main]

use evfmt::{FormatResult, Policy, format_text, scan};
use libfuzzer_sys::fuzz_target;

fn formatted_output(input: &str, policy: &Policy) -> String {
    match format_text(input, policy) {
        FormatResult::Unchanged => input.to_owned(),
        FormatResult::Changed(output) => output,
    }
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
});
