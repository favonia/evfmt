use super::*;
use std::io;

fn operation(id: OperationId, value: &str) -> OrderedOperation {
    OrderedOperation {
        id,
        value: value.to_owned(),
    }
}

#[allow(clippy::panic)]
fn assert_parse_error<T>(result: Result<T, CliParseError>, expected: &str) {
    let Err(error) = result else {
        panic!("parse should fail");
    };
    assert!(
        error.to_string().contains(expected),
        "expected error containing {expected:?}, got {error}"
    );
}

#[test]
fn exit_status_codes_match_cli_contract() {
    assert_eq!(ExitStatus::Success.code(), 0);
    assert_eq!(ExitStatus::CheckFoundChanges.code(), 1);
    assert_eq!(ExitStatus::UsageOrIoError.code(), 2);
}

#[test]
fn ignore_settings_apply_label_updates() {
    let mut settings = IgnoreSettings::from_labels(&[IgnoreLabel::Git]);

    assert!(settings.git);
    assert!(!settings.evfmt);
    assert!(!settings.hidden);

    settings.enable(&[IgnoreLabel::Evfmt, IgnoreLabel::Hidden]);
    assert!(settings.git);
    assert!(settings.evfmt);
    assert!(settings.hidden);

    settings.disable(&[IgnoreLabel::Git, IgnoreLabel::Hidden]);
    assert!(!settings.git);
    assert!(settings.evfmt);
    assert!(!settings.hidden);
}

#[test]
#[allow(clippy::unwrap_used)]
fn variation_set_lists_parse_shortcuts_and_comma_lists() {
    let all = parse_variation_set_list(UpdateKind::Add, "all").unwrap();
    assert!(all.contains('#'));
    assert!(all.contains_keycap('#'));
    assert!(all.contains('\u{00A9}'));

    assert_eq!(
        parse_variation_set_list(UpdateKind::Set, "none").unwrap(),
        VariationSet::none()
    );

    let set =
        parse_variation_set_list(UpdateKind::Add, " ascii, u(00A9), k(0023), *\u{FE0F} ").unwrap();
    assert!(set.contains('#'));
    assert!(set.contains('*'));
    assert!(set.contains('\u{00A9}'));
    assert!(set.contains_keycap('#'));
    assert!(!set.contains('\u{2728}'));

    let text_defaults = parse_variation_set_list(UpdateKind::Add, "text-defaults").unwrap();
    assert!(text_defaults.contains('\u{00A9}'));
    assert!(!text_defaults.contains('\u{2728}'));

    let emoji_defaults = parse_variation_set_list(UpdateKind::Add, "emoji-defaults").unwrap();
    assert!(emoji_defaults.contains('\u{2728}'));
    assert!(!emoji_defaults.contains('\u{00A9}'));

    let arrows = parse_variation_set_list(UpdateKind::Add, "arrows").unwrap();
    assert!(arrows.contains('\u{2194}'));
    assert!(!arrows.contains('\u{2660}'));

    let keycap_chars = parse_variation_set_list(UpdateKind::Add, "keycap-chars").unwrap();
    assert!(!keycap_chars.contains('#'));
    assert!(keycap_chars.contains_keycap('#'));

    let keycap_emojis = parse_variation_set_list(UpdateKind::Add, "keycap-emojis").unwrap();
    assert!(keycap_emojis.contains_keycap('#'));
    assert!(!keycap_emojis.contains_keycap('\u{00A9}'));
}

#[test]
fn variation_set_lists_reject_invalid_shortcut_usage() {
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Add, "none"),
        "`none` is only allowed",
    );
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "all,ascii"),
        "`all` and `none` must appear alone",
    );
    assert_parse_error(parse_variation_set_list(UpdateKind::Set, ""), "empty list");
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "ascii,"),
        "empty list item",
    );
}

#[test]
fn variation_set_items_report_specific_errors() {
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "arowws"),
        "unknown variation set preset `arowws`; did you mean `arrows`?",
    );
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "card_suit"),
        "did you mean `card-suits`?",
    );
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "u(110000)"),
        "invalid code point item",
    );
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "u(00ag)"),
        "invalid code point item",
    );
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "u(00A9"),
        "invalid code point item",
    );
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "u(0041)"),
        "not eligible for emoji variation selectors",
    );
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "A"),
        "not eligible for emoji variation selectors",
    );
    assert_parse_error(
        parse_variation_set_list(UpdateKind::Set, "\u{00A9}#"),
        "invalid variation set item",
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn ignore_lists_parse_shortcuts_and_labels() {
    assert_eq!(
        parse_ignore_list(UpdateKind::Set, "none").unwrap(),
        Vec::<IgnoreLabel>::new()
    );
    assert_eq!(
        parse_ignore_list(UpdateKind::Remove, "all").unwrap(),
        [IgnoreLabel::Git, IgnoreLabel::Evfmt, IgnoreLabel::Hidden]
    );
    assert_eq!(
        parse_ignore_list(UpdateKind::Add, " git, hidden ").unwrap(),
        [IgnoreLabel::Git, IgnoreLabel::Hidden]
    );
}

#[test]
fn ignore_lists_reject_invalid_labels() {
    assert_parse_error(
        parse_ignore_list(UpdateKind::Add, "none"),
        "`none` is only allowed with `--set-ignore`",
    );
    assert_parse_error(
        parse_ignore_list(UpdateKind::Set, "hdden"),
        "did you mean `hidden`?",
    );
    assert_parse_error(parse_ignore_list(UpdateKind::Set, "  "), "empty list");
    assert_parse_error(
        parse_ignore_list(UpdateKind::Set, "git,"),
        "empty list item",
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn policy_operations_apply_to_their_target_in_order() {
    let settings = build_runtime_settings(&[
        operation(OperationId::SetPreferBare, "none"),
        operation(OperationId::AddPreferBare, "rights-marks"),
        operation(OperationId::RemovePreferBare, "u(00AE)"),
        operation(OperationId::SetBareAsText, "all"),
        operation(OperationId::RemoveBareAsText, "ascii"),
        operation(OperationId::AddBareAsText, "card-suits"),
        operation(OperationId::SetIgnore, "none"),
    ])
    .unwrap();

    assert_eq!(
        formatter::format_text("\u{00A9}", &settings.policy),
        FormatResult::Unchanged
    );
    assert_eq!(
        formatter::format_text("\u{00AE}", &settings.policy),
        FormatResult::Changed("\u{00AE}\u{FE0E}".to_owned())
    );
    assert_eq!(
        formatter::format_text("#", &settings.policy),
        FormatResult::Changed("#\u{FE0F}".to_owned())
    );
    assert_eq!(
        formatter::format_text("\u{2660}", &settings.policy),
        FormatResult::Changed("\u{2660}\u{FE0E}".to_owned())
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn ignore_operations_apply_left_to_right() {
    let settings = build_runtime_settings(&[
        operation(OperationId::SetIgnore, "none"),
        operation(OperationId::AddIgnore, "git,hidden"),
        operation(OperationId::RemoveIgnore, "hidden"),
        operation(OperationId::AddPreferBare, "rights-marks"),
    ])
    .unwrap();

    assert!(settings.ignore.git);
    assert!(!settings.ignore.evfmt);
    assert!(!settings.ignore.hidden);
}

#[test]
fn update_kind_helpers_classify_operation_ids() {
    assert_eq!(
        OperationId::SetPreferBare.runtime_operation(),
        RuntimeOperation::PreferBare(UpdateKind::Set)
    );
    assert_eq!(
        OperationId::AddBareAsText.runtime_operation(),
        RuntimeOperation::BareAsText(UpdateKind::Add)
    );
    assert_eq!(
        OperationId::RemoveIgnore.runtime_operation(),
        RuntimeOperation::Ignore(UpdateKind::Remove)
    );
}

#[test]
fn edit_distance_supports_suggestions_threshold() {
    assert_eq!(edit_distance("hdden", "hidden"), 1);
    assert_eq!(edit_distance("", "abc"), 3);
    assert_eq!(edit_distance("abc", ""), 3);
    assert_eq!(edit_distance("abc", "a"), 2);
    assert_eq!(edit_distance("abc", "ac"), 1);
    assert_eq!(edit_distance("kitten", "sitting"), 3);
    assert_eq!(suggest_name("abd", &["abc", "abe"]), Some("abc")); // spellchecker:disable-line
    assert_eq!(suggest_name("arr", &named_set_names()), Some("arrows"));
    assert_eq!(suggest_name("unrelated", &named_set_names()), None);
    assert_eq!(
        suggest_name("card-suit", &named_set_names()),
        Some("card-suits")
    );
}

#[test]
fn flag_names_match_public_options() {
    assert_eq!(OperationId::SetPreferBare.flag_name(), "--set-prefer-bare");
    assert_eq!(OperationId::AddPreferBare.flag_name(), "--add-prefer-bare");
    assert_eq!(
        OperationId::RemovePreferBare.flag_name(),
        "--remove-prefer-bare"
    );
    assert_eq!(OperationId::SetBareAsText.flag_name(), "--set-bare-as-text");
    assert_eq!(OperationId::AddBareAsText.flag_name(), "--add-bare-as-text");
    assert_eq!(
        OperationId::RemoveBareAsText.flag_name(),
        "--remove-bare-as-text"
    );
    assert_eq!(OperationId::SetIgnore.flag_name(), "--set-ignore");
    assert_eq!(OperationId::AddIgnore.flag_name(), "--add-ignore");
    assert_eq!(OperationId::RemoveIgnore.flag_name(), "--remove-ignore");
}

struct FailingWriter;

impl io::Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "test writer failed",
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
#[allow(clippy::panic)]
fn format_lines_reports_writer_errors() {
    let mut reader = io::Cursor::new("\u{00A9}");
    let mut writer = FailingWriter;

    let result = format_lines(&mut reader, &mut writer, &Policy::default());

    let Err(ProcessLinesError::Write(error)) = result else {
        panic!("format_lines should report the writer error");
    };
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}

#[test]
#[allow(clippy::unwrap_used)]
fn atomic_rewrite_reports_read_errors_from_rewrite_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("bad.bin");
    let mut content = "\u{00A9}\n".as_bytes().to_vec();
    content.push(0xFF);
    fs::write(&path, &content).unwrap();

    let error = atomic_rewrite(&path, &Policy::default()).unwrap_err();

    assert!(error.contains("stream did not contain valid UTF-8"));
    assert_eq!(fs::read(&path).unwrap(), content);
}

#[test]
#[allow(clippy::unwrap_used)]
fn atomic_rewrite_reports_no_change_for_canonical_input() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("canonical.txt");
    fs::write(&path, "Hello, world!").unwrap();

    let result = atomic_rewrite(&path, &Policy::default()).unwrap();

    assert_eq!(result, (false, Vec::new()));
    assert_eq!(fs::read_to_string(&path).unwrap(), "Hello, world!");
}
