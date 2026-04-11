use super::*;

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
fn reserved_file_names_require_explicit_separator() {
    let args = SharedArgs {
        files: vec![PathBuf::from("plain.txt"), PathBuf::from("check")],
    };

    assert!(validate_reserved_names(&args, true).is_ok());
    #[allow(clippy::expect_used)]
    let error = validate_reserved_names(&args, false).expect_err("check is reserved");
    assert!(error.contains("reserved as a subcommand"));
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
fn character_lists_parse_shortcuts_and_comma_lists() {
    let all = parse_character_list(UpdateKind::Add, "all").unwrap();
    assert!(all.contains('#'));
    assert!(all.contains('\u{00A9}'));

    assert_eq!(
        parse_character_list(UpdateKind::Set, "none").unwrap(),
        CharSet::none()
    );

    let set = parse_character_list(UpdateKind::Add, " ascii, u(00A9), *\u{FE0F} ").unwrap();
    assert!(set.contains('#'));
    assert!(set.contains('*'));
    assert!(set.contains('\u{00A9}'));
    assert!(!set.contains('\u{2728}'));
}

#[test]
fn character_lists_reject_invalid_shortcut_usage() {
    assert_parse_error(
        parse_character_list(UpdateKind::Add, "none"),
        "`none` is only allowed",
    );
    assert_parse_error(
        parse_character_list(UpdateKind::Set, "all,ascii"),
        "`all` and `none` must appear alone",
    );
    assert_parse_error(parse_character_list(UpdateKind::Set, ""), "empty list");
    assert_parse_error(
        parse_character_list(UpdateKind::Set, "ascii,"),
        "empty list item",
    );
}

#[test]
fn character_items_report_specific_errors() {
    assert_parse_error(
        parse_character_list(UpdateKind::Set, "arowws"),
        "did you mean `arrows`?",
    );
    assert_parse_error(
        parse_character_list(UpdateKind::Set, "u(110000)"),
        "invalid code point item",
    );
    assert_parse_error(
        parse_character_list(UpdateKind::Set, "u(00ag)"),
        "invalid code point item",
    );
    assert_parse_error(
        parse_character_list(UpdateKind::Set, "u(00A9"),
        "invalid code point item",
    );
    assert_parse_error(
        parse_character_list(UpdateKind::Set, "u(0041)"),
        "not eligible for emoji variation selectors",
    );
    assert_parse_error(
        parse_character_list(UpdateKind::Set, "A"),
        "not eligible for emoji variation selectors",
    );
    assert_parse_error(
        parse_character_list(UpdateKind::Set, "\u{00A9}#"),
        "invalid selector item",
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
    let policy = build_policy(&[
        operation(OperationId::SetPreferBare, "none"),
        operation(OperationId::AddPreferBare, "rights-marks"),
        operation(OperationId::RemovePreferBare, "u(00AE)"),
        operation(OperationId::SetBareAsText, "all"),
        operation(OperationId::RemoveBareAsText, "ascii"),
        operation(OperationId::AddBareAsText, "card-suits"),
        operation(OperationId::SetIgnore, "none"),
    ])
    .unwrap();

    assert!(policy.prefer_bare.contains('\u{00A9}'));
    assert!(!policy.prefer_bare.contains('\u{00AE}'));
    assert!(!policy.prefer_bare.contains('#'));
    assert!(!policy.bare_as_text.contains('#'));
    assert!(policy.bare_as_text.contains('\u{00A9}'));
    assert!(policy.bare_as_text.contains('\u{2660}'));
}

#[test]
#[allow(clippy::unwrap_used)]
fn ignore_operations_apply_left_to_right() {
    let settings = build_ignore_settings(&[
        operation(OperationId::SetIgnore, "none"),
        operation(OperationId::AddIgnore, "git,hidden"),
        operation(OperationId::RemoveIgnore, "hidden"),
        operation(OperationId::AddPreferBare, "rights-marks"),
    ])
    .unwrap();

    assert!(settings.git);
    assert!(!settings.evfmt);
    assert!(!settings.hidden);
}

#[test]
fn update_kind_helpers_classify_operation_ids() {
    assert_eq!(
        character_update_kind(OperationId::SetPreferBare, CharacterTarget::PreferBare),
        Some(UpdateKind::Set)
    );
    assert_eq!(
        character_update_kind(OperationId::AddBareAsText, CharacterTarget::BareAsText),
        Some(UpdateKind::Add)
    );
    assert_eq!(
        character_update_kind(OperationId::RemoveIgnore, CharacterTarget::PreferBare),
        None
    );
    assert_eq!(
        ignore_update_kind(OperationId::RemoveIgnore),
        Some(UpdateKind::Remove)
    );
    assert_eq!(ignore_update_kind(OperationId::SetBareAsText), None);
}

#[test]
fn edit_distance_supports_suggestions_threshold() {
    assert_eq!(edit_distance("hdden", "hidden"), 1);
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
