use super::*;
use crate::charset::{CharSet, NamedSetId};
use crate::scanner::{VS_EMOJI, VS_TEXT};
use crate::unicode::{self, DefaultSide};

/// Create the default policy used by most tests.
/// prefer-bare='ascii': ASCII characters keep their bare form.
/// bare-as-text='ascii': ASCII bare characters resolve to text; all others to emoji.
fn default_policy() -> Policy {
    Policy {
        prefer_bare: CharSet::named(NamedSetId::Ascii),
        bare_as_text: CharSet::named(NamedSetId::Ascii),
    }
}

fn bool_charset(matches: bool) -> CharSet {
    if matches {
        CharSet::all()
    } else {
        CharSet::none()
    }
}

#[test]
fn test_plain_ascii_unchanged() {
    let policy = default_policy();
    // Plain ASCII text with no variation-sequence characters → unchanged.
    assert_eq!(
        format_text("Hello, world!", &policy),
        FormatResult::Unchanged
    );
}

#[test]
fn test_bare_ascii_eligible_stays_bare() {
    let policy = default_policy();
    // '#' has variation-sequence data and is ASCII → bare stays bare.
    assert_eq!(format_text("#", &policy), FormatResult::Unchanged);
}

#[test]
fn test_ascii_with_bare_side_selector_removed() {
    let policy = default_policy();
    // '#' is ASCII, bare_as_text=ascii → bare side is text.
    // FE0E matches bare side → redundant → removed.
    assert_eq!(
        format_text("#\u{FE0E}", &policy),
        FormatResult::Changed("#".to_owned())
    );
}

#[test]
fn test_ascii_with_non_bare_side_selector_kept() {
    let policy = default_policy();
    // '#' is ASCII, bare_as_text=ascii → bare side is text.
    // FE0F is the other side → meaningful → kept.
    assert_eq!(format_text("#\u{FE0F}", &policy), FormatResult::Unchanged);
}

#[test]
fn test_non_ascii_text_default_bare_gets_emoji_selector() {
    let policy = default_policy();
    // U+00A9 COPYRIGHT SIGN is non-ASCII and has text-default variation-sequence data.
    // With bare-as-text='ascii', ©️ is not ASCII → bare resolves to emoji → add FE0F.
    assert_eq!(
        format_text("\u{00A9}", &policy),
        FormatResult::Changed("\u{00A9}\u{FE0F}".to_owned())
    );
}

#[test]
fn test_non_ascii_with_explicit_text_selector_preserved() {
    let policy = default_policy();
    // U+00A9 with explicit FE0E → preserved.
    // Explicit selectors are always respected (Step 3).
    assert_eq!(
        format_text("\u{00A9}\u{FE0E}", &policy),
        FormatResult::Unchanged
    );
}

#[test]
fn test_non_ascii_with_explicit_emoji_selector_preserved() {
    let policy = default_policy();
    assert_eq!(
        format_text("\u{00A9}\u{FE0F}", &policy),
        FormatResult::Unchanged
    );
}

#[test]
fn test_illegal_selector_removed() {
    let policy = default_policy();
    // 'A' has no variation sequence, so FE0F after it
    // is illegal → removed (Step 1).
    assert_eq!(
        format_text("A\u{FE0F}", &policy),
        FormatResult::Changed("A".to_owned())
    );
}

#[test]
fn canonicalize_item_matches_format_once_behavior() {
    let policy = default_policy();
    let input = "#\u{FE0E}\u{20E3}\u{2764}\u{200D}\u{1F525}A\u{FE0F}";
    let items = scanner::scan(input);
    let rebuilt: String = items
        .iter()
        .map(|item| canonicalize_item(item, &policy))
        .collect();

    let expected = match format_text(input, &policy) {
        FormatResult::Changed(output) => output,
        FormatResult::Unchanged => input.to_owned(),
    };

    assert_eq!(rebuilt, expected);
}

#[test]
fn test_standalone_selector_removed() {
    let policy = default_policy();
    // Standalone FE0F at the start of the string → removed.
    assert_eq!(
        format_text("\u{FE0F}hello", &policy),
        FormatResult::Changed("hello".to_owned())
    );
}

#[test]
fn test_double_selector_after_eligible() {
    let policy = default_policy();
    // '#' followed by FE0F then FE0E → keep the first (non-default),
    // remove the second (extra consecutive selector).
    assert_eq!(
        format_text("#\u{FE0F}\u{FE0E}", &policy),
        FormatResult::Changed("#\u{FE0F}".to_owned())
    );
}

#[test]
fn test_orphan_selector_after_zwj_is_resolved_in_one_pass() {
    let policy = default_policy();
    let input = "\u{1F525}\u{200D}\u{FE0F}\u{2764}";
    let expected = "\u{1F525}\u{200D}\u{2764}\u{FE0F}";

    assert_eq!(
        format_text(input, &policy),
        FormatResult::Changed(expected.to_owned())
    );
}

// --- Keycap formatting tests ---

#[test]
fn test_keycap_correct_unchanged() {
    let policy = default_policy();
    // #️⃣ (correct keycap) → unchanged.
    assert_eq!(
        format_text("#\u{FE0F}\u{20E3}", &policy),
        FormatResult::Unchanged
    );
}

#[test]
fn test_keycap_missing_fe0f() {
    let policy = default_policy();
    // #️⃣ (bare keycap) → add FE0F.
    assert_eq!(
        format_text("#\u{20E3}", &policy),
        FormatResult::Changed("#\u{FE0F}\u{20E3}".to_owned())
    );
}

#[test]
fn test_keycap_wrong_vs() {
    let policy = default_policy();
    // # FE0E ⃣ → replace FE0E with FE0F.
    assert_eq!(
        format_text("#\u{FE0E}\u{20E3}", &policy),
        FormatResult::Changed("#\u{FE0F}\u{20E3}".to_owned())
    );
}

#[test]
fn test_keycap_all_bases() {
    let policy = default_policy();
    for base in ['#', '*', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'] {
        let correct = format!("{base}\u{FE0F}\u{20E3}");
        assert_eq!(
            format_text(&correct, &policy),
            FormatResult::Unchanged,
            "base: {base}"
        );

        let bare = format!("{base}\u{20E3}");
        assert_eq!(
            format_text(&bare, &policy),
            FormatResult::Changed(correct.clone()),
            "base: {base} (bare)"
        );
    }
}

#[test]
fn test_keycap_idempotent() {
    let policy = default_policy();
    for base in ['#', '*', '0', '9'] {
        for vs in ["\u{FE0F}", "\u{FE0E}", ""] {
            let input = format!("{base}{vs}\u{20E3}");
            let first = match format_text(&input, &policy) {
                FormatResult::Unchanged => input.clone(),
                FormatResult::Changed(s) => s,
            };
            assert_eq!(
                format_text(&first, &policy),
                FormatResult::Unchanged,
                "not idempotent for keycap input: {input:?}"
            );
        }
    }
}

// --- ZWJ formatting tests ---

#[test]
fn test_zwj_text_default_gets_fe0f() {
    let policy = default_policy();
    // ❤️ (2764, text-default) ZWJ 🔥 (1F525) → ❤️ FE0F ZWJ 🔥
    assert_eq!(
        format_text("\u{2764}\u{200D}\u{1F525}", &policy),
        FormatResult::Changed("\u{2764}\u{FE0F}\u{200D}\u{1F525}".to_owned())
    );
}

#[test]
fn test_zwj_text_default_with_fe0f_unchanged() {
    let policy = default_policy();
    // ❤️ FE0F ZWJ 🔥 → unchanged
    assert_eq!(
        format_text("\u{2764}\u{FE0F}\u{200D}\u{1F525}", &policy),
        FormatResult::Unchanged
    );
}

#[test]
fn test_zwj_text_default_wrong_vs_fixed() {
    let policy = default_policy();
    // ❤️ FE0E ZWJ 🔥 → ❤️ FE0F ZWJ 🔥
    assert_eq!(
        format_text("\u{2764}\u{FE0E}\u{200D}\u{1F525}", &policy),
        FormatResult::Changed("\u{2764}\u{FE0F}\u{200D}\u{1F525}".to_owned())
    );
}

#[test]
fn test_zwj_non_eligible_selector_removed() {
    let policy = default_policy();
    // 😀 (1F600) has no variation sequence here (fully emoji, not dual-presentation).
    // Unsupported selectors on ZWJ components without variation-sequence data are removed.
    assert_eq!(
        format_text("\u{1F600}\u{FE0F}\u{200D}\u{1F525}", &policy),
        FormatResult::Changed("\u{1F600}\u{200D}\u{1F525}".to_owned())
    );
}

#[test]
fn test_zwj_with_skin_tone() {
    let policy = default_policy();
    // 👨 🏻 ZWJ 👦 → unchanged (no text-default components)
    assert_eq!(
        format_text("\u{1F468}\u{1F3FB}\u{200D}\u{1F466}", &policy),
        FormatResult::Unchanged
    );
}

#[test]
fn test_zwj_idempotent() {
    let policy = default_policy();
    let inputs = [
        "\u{2764}\u{200D}\u{1F525}",           // bare text-default
        "\u{2764}\u{FE0F}\u{200D}\u{1F525}",   // correct
        "\u{2764}\u{FE0E}\u{200D}\u{1F525}",   // wrong VS
        "\u{1F468}\u{1F3FB}\u{200D}\u{1F466}", // with emoji modifier
    ];
    for input in &inputs {
        let first = match format_text(input, &policy) {
            FormatResult::Unchanged => (*input).to_owned(),
            FormatResult::Changed(s) => s,
        };
        assert_eq!(
            format_text(&first, &policy),
            FormatResult::Unchanged,
            "not idempotent for ZWJ input: {input:?}"
        );
    }
}

#[test]
fn test_idempotence() {
    // Idempotence test: formatting twice should produce the same result as once.
    // fmt(fmt(x)) == fmt(x) — this is a fundamental property of the formatter.
    let policy = default_policy();
    let inputs = [
        "Hello #world",
        "#\u{FE0E}",
        "#\u{FE0F}",
        "\u{00A9}",
        "\u{00A9}\u{FE0E}",
        "\u{00A9}\u{FE0F}",
        "A\u{FE0F}B",
        "\u{FE0F}test",
        "\u{2728}", // SPARKLES - emoji default
        "\u{2728}\u{FE0E}",
        "\u{2728}\u{FE0F}",
    ];

    for input in &inputs {
        // First pass: format the input.
        let first = match format_text(input, &policy) {
            FormatResult::Unchanged => (*input).to_owned(),
            FormatResult::Changed(s) => s,
        };
        // Second pass: format the result of the first pass.
        // It should always be Unchanged (already canonical).
        let second = format_text(&first, &policy);
        assert_eq!(
            second,
            FormatResult::Unchanged,
            // This message is shown if the assertion fails.
            // `:?` uses Debug formatting which shows escape sequences.
            "not idempotent for input: {input:?}, first pass: {first:?}"
        );
    }
}

#[test]
fn test_emoji_default_prefer_bare() {
    // U+2728 SPARKLES is emoji-default.
    // With prefer-bare='all' and bare-as-text='all', bare side is text.
    // FE0E (text = bare side) is redundant → removed.
    // FE0F (emoji ≠ bare side) is meaningful → kept.
    let policy = Policy {
        prefer_bare: CharSet::all(),
        bare_as_text: CharSet::all(),
    };
    assert_eq!(format_text("\u{2728}", &policy), FormatResult::Unchanged);
    assert_eq!(
        format_text("\u{2728}\u{FE0E}", &policy),
        FormatResult::Changed("\u{2728}".to_owned())
    );
    assert_eq!(
        format_text("\u{2728}\u{FE0F}", &policy),
        FormatResult::Unchanged
    );
}

#[test]
fn test_non_ascii_bare_as_text() {
    // With bare-as-text='all', bare non-bare-preferred characters resolve
    // to the text side → add FE0E.
    let policy = Policy {
        prefer_bare: CharSet::none(),
        bare_as_text: CharSet::all(),
    };
    assert_eq!(
        format_text("\u{00A9}", &policy),
        FormatResult::Changed("\u{00A9}\u{FE0E}".to_owned())
    );
}

#[test]
fn test_mixed_content() {
    let policy = default_policy();
    // Mix of plain text, ASCII variation-sequence data (#), and non-ASCII
    // variation-sequence data (©️).
    // '#' is ASCII → kept bare. '©️' is non-ASCII → gets FE0F (emoji bias).
    let input = "Press # for \u{00A9}";
    let result = format_text(input, &policy);
    assert_eq!(
        result,
        FormatResult::Changed("Press # for \u{00A9}\u{FE0F}".to_owned())
    );
}

// -----------------------------------------------------------------------
// Phase 1: Decision table test
//
// This is a human-auditable semantic summary of the formatter's decision
// logic. Each row encodes the semantic axes explicitly so that a reviewer
// can verify the table covers all reachable cases without inspecting
// opaque input/output strings.
//
// Axes:
//   1. Character eligibility: Ineligible / TextDefault / EmojiDefault
//   2. Input selector: None / FE0E / FE0F
//   3. prefer_bare policy: true / false
//   4. bare_as_text policy: true / false
//
// Key design property: for characters with variation-sequence data (where both VS are
// sanctioned), the output depends ONLY on (prefer_bare, bare_as_text,
// input_selector). The Unicode default side does not enter the
// decision logic. We test with both text-default and emoji-default
// representative characters to verify this independence.
//
// Representative characters:
//   Ineligible:    'A' (U+0041) — no variation sequence
//   Text-default:  '©️' (U+00A9) — has_text_vs=true, has_emoji_vs=true, default=Text
//   Emoji-default: '✨️' (U+2728) — has_text_vs=true, has_emoji_vs=true, default=Emoji
// -----------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum CharType {
    Ineligible,
    TextDefault,
    EmojiDefault,
}

#[derive(Debug, Clone, Copy)]
enum InputSelector {
    None,
    TextVS,
    EmojiVS,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedForm {
    Bare,
    WithTextVS,
    WithEmojiVS,
}

struct DecisionRow {
    label: &'static str,
    char_type: CharType,
    input_selector: InputSelector,
    prefer_bare: bool,
    bare_as_text: bool,
    expected: ExpectedForm,
}

#[derive(Debug, Clone, Copy)]
struct PolicyFlags {
    prefer_bare: bool,
    bare_as_text: bool,
}

#[derive(Debug, Clone, Copy)]
struct EligibilityFlags {
    has_text_vs: bool,
    has_emoji_vs: bool,
}

fn representative_char(ct: CharType) -> char {
    match ct {
        CharType::Ineligible => 'A',
        CharType::TextDefault => '\u{00A9}',  // ©️
        CharType::EmojiDefault => '\u{2728}', // ✨️
    }
}

fn build_input(ch: char, sel: InputSelector) -> String {
    let mut s = String::new();
    s.push(ch);
    match sel {
        InputSelector::None => {}
        InputSelector::TextVS => s.push(VS_TEXT),
        InputSelector::EmojiVS => s.push(VS_EMOJI),
    }
    s
}

fn build_expected(ch: char, form: ExpectedForm) -> String {
    let mut s = String::new();
    s.push(ch);
    match form {
        ExpectedForm::Bare => {}
        ExpectedForm::WithTextVS => s.push(VS_TEXT),
        ExpectedForm::WithEmojiVS => s.push(VS_EMOJI),
    }
    s
}

fn decision_table_ineligible() -> &'static [DecisionRow] {
    use CharType::*;
    use InputSelector::*;
    &[
        DecisionRow {
            label: "ineligible, bare",
            char_type: Ineligible,
            input_selector: None,
            prefer_bare: false,
            bare_as_text: false,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "ineligible, +FE0E (illegal)",
            char_type: Ineligible,
            input_selector: TextVS,
            prefer_bare: false,
            bare_as_text: false,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "ineligible, +FE0F (illegal)",
            char_type: Ineligible,
            input_selector: EmojiVS,
            prefer_bare: false,
            bare_as_text: false,
            expected: ExpectedForm::Bare,
        },
    ]
}

fn decision_table_text_default() -> &'static [DecisionRow] {
    use CharType::TextDefault;
    use InputSelector::{EmojiVS, None, TextVS};
    &[
        DecisionRow {
            label: "text-default, bare, kb=T, bat=T",
            char_type: TextDefault,
            input_selector: None,
            prefer_bare: true,
            bare_as_text: true,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "text-default, +FE0E (redundant), kb=T, bat=T",
            char_type: TextDefault,
            input_selector: TextVS,
            prefer_bare: true,
            bare_as_text: true,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "text-default, +FE0F (meaningful), kb=T, bat=T",
            char_type: TextDefault,
            input_selector: EmojiVS,
            prefer_bare: true,
            bare_as_text: true,
            expected: ExpectedForm::WithEmojiVS,
        },
        DecisionRow {
            label: "text-default, bare, kb=T, bat=F",
            char_type: TextDefault,
            input_selector: None,
            prefer_bare: true,
            bare_as_text: false,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "text-default, +FE0E (meaningful), kb=T, bat=F",
            char_type: TextDefault,
            input_selector: TextVS,
            prefer_bare: true,
            bare_as_text: false,
            expected: ExpectedForm::WithTextVS,
        },
        DecisionRow {
            label: "text-default, +FE0F (redundant), kb=T, bat=F",
            char_type: TextDefault,
            input_selector: EmojiVS,
            prefer_bare: true,
            bare_as_text: false,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "text-default, +FE0E (explicit), kb=F, bat=T",
            char_type: TextDefault,
            input_selector: TextVS,
            prefer_bare: false,
            bare_as_text: true,
            expected: ExpectedForm::WithTextVS,
        },
        DecisionRow {
            label: "text-default, +FE0F (explicit), kb=F, bat=T",
            char_type: TextDefault,
            input_selector: EmojiVS,
            prefer_bare: false,
            bare_as_text: true,
            expected: ExpectedForm::WithEmojiVS,
        },
        DecisionRow {
            label: "text-default, bare, kb=F, bat=T",
            char_type: TextDefault,
            input_selector: None,
            prefer_bare: false,
            bare_as_text: true,
            expected: ExpectedForm::WithTextVS,
        },
        DecisionRow {
            label: "text-default, +FE0E (explicit), kb=F, bat=F",
            char_type: TextDefault,
            input_selector: TextVS,
            prefer_bare: false,
            bare_as_text: false,
            expected: ExpectedForm::WithTextVS,
        },
        DecisionRow {
            label: "text-default, +FE0F (explicit), kb=F, bat=F",
            char_type: TextDefault,
            input_selector: EmojiVS,
            prefer_bare: false,
            bare_as_text: false,
            expected: ExpectedForm::WithEmojiVS,
        },
        DecisionRow {
            label: "text-default, bare, kb=F, bat=F",
            char_type: TextDefault,
            input_selector: None,
            prefer_bare: false,
            bare_as_text: false,
            expected: ExpectedForm::WithEmojiVS,
        },
    ]
}

fn decision_table_emoji_default() -> &'static [DecisionRow] {
    use CharType::EmojiDefault;
    use InputSelector::{EmojiVS, None, TextVS};
    &[
        DecisionRow {
            label: "emoji-default, bare, kb=T, bat=T",
            char_type: EmojiDefault,
            input_selector: None,
            prefer_bare: true,
            bare_as_text: true,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "emoji-default, +FE0E (redundant), kb=T, bat=T",
            char_type: EmojiDefault,
            input_selector: TextVS,
            prefer_bare: true,
            bare_as_text: true,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "emoji-default, +FE0F (meaningful), kb=T, bat=T",
            char_type: EmojiDefault,
            input_selector: EmojiVS,
            prefer_bare: true,
            bare_as_text: true,
            expected: ExpectedForm::WithEmojiVS,
        },
        DecisionRow {
            label: "emoji-default, bare, kb=T, bat=F",
            char_type: EmojiDefault,
            input_selector: None,
            prefer_bare: true,
            bare_as_text: false,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "emoji-default, +FE0E (meaningful), kb=T, bat=F",
            char_type: EmojiDefault,
            input_selector: TextVS,
            prefer_bare: true,
            bare_as_text: false,
            expected: ExpectedForm::WithTextVS,
        },
        DecisionRow {
            label: "emoji-default, +FE0F (redundant), kb=T, bat=F",
            char_type: EmojiDefault,
            input_selector: EmojiVS,
            prefer_bare: true,
            bare_as_text: false,
            expected: ExpectedForm::Bare,
        },
        DecisionRow {
            label: "emoji-default, +FE0E (explicit), kb=F, bat=T",
            char_type: EmojiDefault,
            input_selector: TextVS,
            prefer_bare: false,
            bare_as_text: true,
            expected: ExpectedForm::WithTextVS,
        },
        DecisionRow {
            label: "emoji-default, +FE0F (explicit), kb=F, bat=T",
            char_type: EmojiDefault,
            input_selector: EmojiVS,
            prefer_bare: false,
            bare_as_text: true,
            expected: ExpectedForm::WithEmojiVS,
        },
        DecisionRow {
            label: "emoji-default, bare, kb=F, bat=T",
            char_type: EmojiDefault,
            input_selector: None,
            prefer_bare: false,
            bare_as_text: true,
            expected: ExpectedForm::WithTextVS,
        },
        DecisionRow {
            label: "emoji-default, +FE0E (explicit), kb=F, bat=F",
            char_type: EmojiDefault,
            input_selector: TextVS,
            prefer_bare: false,
            bare_as_text: false,
            expected: ExpectedForm::WithTextVS,
        },
        DecisionRow {
            label: "emoji-default, +FE0F (explicit), kb=F, bat=F",
            char_type: EmojiDefault,
            input_selector: EmojiVS,
            prefer_bare: false,
            bare_as_text: false,
            expected: ExpectedForm::WithEmojiVS,
        },
        DecisionRow {
            label: "emoji-default, bare, kb=F, bat=F",
            char_type: EmojiDefault,
            input_selector: None,
            prefer_bare: false,
            bare_as_text: false,
            expected: ExpectedForm::WithEmojiVS,
        },
    ]
}

#[test]
fn test_decision_table() {
    for section in [
        decision_table_ineligible(),
        decision_table_text_default(),
        decision_table_emoji_default(),
    ] {
        for row in section {
            let ch = representative_char(row.char_type);

            // Build the policy using expressions that match exactly this character.
            let policy = Policy {
                prefer_bare: bool_charset(row.prefer_bare),
                bare_as_text: bool_charset(row.bare_as_text),
            };

            let input = build_input(ch, row.input_selector);
            let expected = build_expected(ch, row.expected);

            let result = format_text(&input, &policy);
            let actual = match &result {
                FormatResult::Unchanged => input.clone(),
                FormatResult::Changed(s) => s.clone(),
            };

            assert_eq!(
                actual, expected,
                "decision table row {:?}: input={:?}, expected={:?}, got={:?}",
                row.label, input, expected, actual
            );
        }
    }
}

// -------------------------------------------------------------------
// Phase 3a: Exhaustive per-entry formatter test
//
// Verify formatter behavior for every entry in VARIATION_ENTRIES
// under all 4 policy combinations × 3 input forms. The expected
// output is computed independently from the formatter using a simple,
// flat match on (prefer_bare, bare_as_text, input_selector, has_text_vs,
// has_emoji_vs). If both this logic and the implementation agree on
// all cases, either both are correct or both share the same bug.
// -------------------------------------------------------------------

#[test]
fn test_exhaustive_per_entry() {
    use crate::unicode::VARIATION_ENTRIES;

    let policies: [(bool, bool); 4] = [(false, false), (false, true), (true, false), (true, true)];

    for entry in VARIATION_ENTRIES {
        let ch = entry.code_point;

        for &(prefer_bare, bare_as_text) in &policies {
            let policy = Policy {
                prefer_bare: bool_charset(prefer_bare),
                bare_as_text: bool_charset(bare_as_text),
            };

            // 3 input forms: bare, +FE0E, +FE0F
            let inputs: [(&str, String); 3] = [
                ("bare", format!("{ch}")),
                ("+FE0E", format!("{ch}\u{FE0E}")),
                ("+FE0F", format!("{ch}\u{FE0F}")),
            ];

            for (form_label, input) in &inputs {
                let result = format_text(input, &policy);
                let actual = match &result {
                    FormatResult::Unchanged => input.clone(),
                    FormatResult::Changed(s) => s.clone(),
                };

                // Independently compute expected output.
                let expected = compute_expected(
                    ch,
                    form_label,
                    PolicyFlags {
                        prefer_bare,
                        bare_as_text,
                    },
                    EligibilityFlags {
                        has_text_vs: entry.has_text_vs,
                        has_emoji_vs: entry.has_emoji_vs,
                    },
                );

                assert_eq!(
                    actual, expected,
                    "U+{:04X} {form_label} kb={prefer_bare} bat={bare_as_text}: \
                     expected={expected:?}, got={actual:?}",
                    ch as u32
                );
            }
        }
    }
}

/// Independent expected-output computation for the exhaustive test.
/// Written as a simple, flat function with no shared code with the
/// formatter implementation.
fn compute_expected(
    ch: char,
    form: &str,
    policy: PolicyFlags,
    eligibility: EligibilityFlags,
) -> String {
    let bare_side_is_emoji = !policy.bare_as_text;

    // Determine the sanctioned selector from the input form.
    let sanctioned_side: Option<bool> = match form {
        "bare" => None,
        "+FE0E" => {
            if eligibility.has_text_vs {
                Some(false)
            } else {
                None
            }
        } // false = text
        "+FE0F" => {
            if eligibility.has_emoji_vs {
                Some(true)
            } else {
                None
            }
        } // true = emoji
        _ => unreachable!(),
    };

    if policy.prefer_bare {
        // Step 2: bare is canonical.
        match sanctioned_side {
            None => {
                // No sanctioned selector → bare.
                format!("{ch}")
            }
            Some(is_emoji) if is_emoji == bare_side_is_emoji => {
                // Selector matches bare side → redundant → bare.
                format!("{ch}")
            }
            Some(false) => {
                // Text selector, bare side is emoji → meaningful.
                format!("{ch}\u{FE0E}")
            }
            Some(true) => {
                // Emoji selector, bare side is text → meaningful.
                format!("{ch}\u{FE0F}")
            }
        }
    } else {
        // Step 3: bare is not canonical.
        match sanctioned_side {
            Some(false) => format!("{ch}\u{FE0E}"), // explicit text
            Some(true) => format!("{ch}\u{FE0F}"),  // explicit emoji
            None => {
                // Bare -> resolve via bare_as_text.
                if policy.bare_as_text {
                    format!("{ch}\u{FE0E}")
                } else {
                    format!("{ch}\u{FE0F}")
                }
            }
        }
    }
}

// -------------------------------------------------------------------
// Phase 3b: Property-based formatter tests
//
// Verify string-level invariants over randomly generated inputs.
// -------------------------------------------------------------------

use proptest::{prelude::*, test_runner::TestCaseError};

/// Strategy that generates strings biased toward interesting token
/// classes: variation-sequence characters, variation selectors, and boundaries.
fn interesting_string_strategy() -> impl Strategy<Value = String> {
    // Token classes with weights.
    let token = prop_oneof![
        // Text-default variation-sequence characters (includes keycap bases #*0-9)
        3 => prop::sample::select(vec![
            '\u{0023}', '\u{002A}', '\u{0030}', '\u{00A9}', '\u{00AE}',
            '\u{203C}', '\u{2049}', '\u{2122}', '\u{2139}', '\u{2764}',
        ]),
        // Emoji-default variation-sequence characters
        3 => prop::sample::select(vec![
            '\u{231A}', '\u{2728}', '\u{2614}', '\u{26A1}', '\u{2705}',
            '\u{270A}', '\u{2B50}', '\u{1F004}', '\u{1F600}',
            '\u{1F468}', '\u{1F466}', '\u{1F525}',
        ]),
        // Variation selectors
        4 => prop::sample::select(vec!['\u{FE0E}', '\u{FE0F}']),
        // ZWJ and keycap combining enclosing
        3 => prop::sample::select(vec!['\u{200D}', '\u{20E3}']),
        // Skin tone modifiers
        1 => prop::sample::select(vec![
            '\u{1F3FB}', '\u{1F3FC}', '\u{1F3FD}', '\u{1F3FE}', '\u{1F3FF}',
        ]),
        // Ineligible ASCII
        2 => prop::sample::select(vec![
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
            'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T',
            'U', 'V', 'W', 'X', 'Y', 'Z',
        ]),
        // Newlines / boundaries
        1 => prop::sample::select(vec!['\n', ' ']),
    ];

    prop::collection::vec(token, 0..40).prop_map(|chars| chars.into_iter().collect::<String>())
}

/// Strategy for policy combinations.
fn policy_strategy() -> impl Strategy<Value = Policy> {
    (prop::bool::ANY, prop::bool::ANY).prop_map(|(kb, bat)| Policy {
        prefer_bare: bool_charset(kb),
        bare_as_text: bool_charset(bat),
    })
}

/// Strip all FE0E and FE0F from a string.
fn strip_selectors(s: &str) -> String {
    s.chars()
        .filter(|&ch| ch != '\u{FE0E}' && ch != '\u{FE0F}')
        .collect()
}

fn assert_zwj_component_selectors_are_canonical(
    sequence: &scanner::ZwjSequence,
) -> Result<(), TestCaseError> {
    match sequence {
        scanner::ZwjSequence::Terminal(component) => {
            let expected = if let Some(info) = unicode::variation_sequence_info(component.base) {
                if info.default_side == DefaultSide::Text && component.emoji_modifier.is_none() {
                    Some(VS_EMOJI)
                } else {
                    None
                }
            } else {
                None
            };
            prop_assert_eq!(
                scanner::zwj_component_effective_selector(component),
                expected,
                "non-canonical ZWJ selector state on U+{:04X}",
                component.base as u32
            );
            Ok(())
        }
        scanner::ZwjSequence::Joined { head, link, tail } => {
            let expected = if let Some(info) = unicode::variation_sequence_info(head.base) {
                if info.default_side == DefaultSide::Text && head.emoji_modifier.is_none() {
                    Some(VS_EMOJI)
                } else {
                    None
                }
            } else {
                None
            };
            prop_assert!(
                link.selectors.is_empty(),
                "non-canonical selectors after ZWJ: {:?}",
                link.selectors
            );
            prop_assert_eq!(
                scanner::zwj_component_effective_selector(head),
                expected,
                "non-canonical ZWJ selector state on U+{:04X}",
                head.base as u32
            );
            assert_zwj_component_selectors_are_canonical(tail)
        }
    }
}

proptest! {
    /// 3a. Idempotency: format(format(x)) == format(x).
    #[test]
    fn prop_idempotent(
        input in interesting_string_strategy(),
        policy in policy_strategy(),
    ) {
        let first = match format_text(&input, &policy) {
            FormatResult::Unchanged => input.clone(),
            FormatResult::Changed(s) => s,
        };
        let second = format_text(&first, &policy);
        prop_assert_eq!(
            second, FormatResult::Unchanged,
            "not idempotent: input={:?}, first={:?}", input, first
        );
    }

    /// 3b. No violations in output: re-scanning the formatted output
    /// should produce zero violations under the same policy.
    #[test]
    fn prop_no_violations_in_output(
        input in interesting_string_strategy(),
        policy in policy_strategy(),
    ) {
        use crate::classify::classify;
        use crate::scanner::scan;

        let output = match format_text(&input, &policy) {
            FormatResult::Unchanged => input.clone(),
            FormatResult::Changed(s) => s,
        };
        let items = scan(&output);
        for item in &items {
            let violation = classify(item, &policy);
            prop_assert_eq!(
                violation, None,
                "violation in output: {:?} for item {:?}", violation, item
            );
        }
    }

    /// 3c. No standalone selector runs in output.
    #[test]
    fn prop_no_standalone_selectors(
        input in interesting_string_strategy(),
        policy in policy_strategy(),
    ) {
        use crate::scanner::{scan, ScanKind};

        let output = match format_text(&input, &policy) {
            FormatResult::Unchanged => input.clone(),
            FormatResult::Changed(s) => s,
        };
        let items = scan(&output);
        for item in &items {
            prop_assert!(
                !matches!(item.kind, ScanKind::StandaloneSelectors(_)),
                "standalone selector run in output: {:?}", item.raw
            );
        }
    }

    /// 3d. Singleton properties: for singleton items in output,
    /// Bare-preferred chars have no redundant selectors and other
    /// variation-sequence chars
    /// chars always have a selector.
    #[test]
    fn prop_singleton_properties(
        input in interesting_string_strategy(),
        policy in policy_strategy(),
    ) {
        use crate::scanner::{scan, ScanKind};

        let output = match format_text(&input, &policy) {
            FormatResult::Unchanged => input.clone(),
            FormatResult::Changed(s) => s,
        };
        let items = scan(&output);
        for item in &items {
            if let ScanKind::Singleton { base, selectors } = &item.kind {
                let selector = scanner::effective_selector(selectors);
                if unicode::has_variation_sequence(*base) {
                    if policy.prefer_bare.contains(*base) {
                        // No redundant selectors.
                        if let Some(sel) = selector {
                            let bare_is_emoji = !policy.bare_as_text.contains(*base);
                            let sel_is_emoji = sel == VS_EMOJI;
                            prop_assert!(
                                sel_is_emoji != bare_is_emoji,
                                "redundant selector on bare-preferred U+{:04X}", *base as u32
                            );
                        }
                    } else {
                        // Must have a selector.
                        prop_assert!(
                            selector.is_some(),
                            "unresolved bare non-bare-preferred U+{:04X}", *base as u32
                        );
                    }
                }
            }
        }
    }

    /// 3e. Keycap sequences always have FE0F.
    #[test]
    fn prop_keycap_has_fe0f(
        input in interesting_string_strategy(),
        policy in policy_strategy(),
    ) {
        use crate::scanner::{scan, ScanKind};

        let output = match format_text(&input, &policy) {
            FormatResult::Unchanged => input.clone(),
            FormatResult::Changed(s) => s,
        };
        let items = scan(&output);
        for item in &items {
            if let ScanKind::Keycap { base, selectors } = &item.kind {
                prop_assert_eq!(
                    selectors.as_slice(), &[VS_EMOJI],
                    "keycap base {:?} missing FE0F", base
                );
            }
        }
    }

    /// 3f. ZWJ components carry selectors only where the sequence discipline
    /// requires them.
    #[test]
    fn prop_zwj_component_selectors_are_canonical(
        input in interesting_string_strategy(),
        policy in policy_strategy(),
    ) {
        use crate::scanner::{scan, ScanKind};

        let output = match format_text(&input, &policy) {
            FormatResult::Unchanged => input.clone(),
            FormatResult::Changed(s) => s,
        };
        let items = scan(&output);
        for item in &items {
            if let ScanKind::Zwj(sequence) = &item.kind {
                assert_zwj_component_selectors_are_canonical(sequence)?;
            }
        }
    }

    /// 3g. Formatting only inserts/removes FE0E and FE0F.
    /// Strip all selectors from both input and output → must be equal.
    #[test]
    fn prop_only_modifies_selectors(
        input in interesting_string_strategy(),
        policy in policy_strategy(),
    ) {
        let output = match format_text(&input, &policy) {
            FormatResult::Unchanged => input.clone(),
            FormatResult::Changed(s) => s,
        };
        let stripped_input = strip_selectors(&input);
        let stripped_output = strip_selectors(&output);
        prop_assert_eq!(
            stripped_input, stripped_output,
            "non-selector content differs"
        );
    }
}
