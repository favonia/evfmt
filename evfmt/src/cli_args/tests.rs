use super::*;

#[test]
fn format_command_parses_files() {
    let matches = build_format_command().get_matches_from(["evfmt format", "one.txt", "two.txt"]);

    assert_eq!(
        parse_shared_args(&matches).files,
        [PathBuf::from("one.txt"), PathBuf::from("two.txt")]
    );
}

#[test]
fn root_command_lists_subcommands() {
    let matches = build_root_command().get_matches_from(["evfmt", "format"]);

    assert_eq!(matches.subcommand_name(), Some("format"));
}

#[test]
fn check_command_parses_shared_files_without_check_flag() {
    let matches = build_check_command().get_matches_from(["evfmt check", "input.txt"]);

    assert_eq!(
        parse_shared_args(&matches).files,
        [PathBuf::from("input.txt")]
    );
}

#[test]
fn format_command_accepts_check_as_a_file_name() {
    let matches = build_format_command().get_matches_from(["evfmt format", "check"]);

    assert_eq!(parse_shared_args(&matches).files, [PathBuf::from("check")]);
}

#[test]
fn operations_are_extracted_in_cli_order_across_option_groups() {
    let matches = build_format_command().get_matches_from([
        "evfmt format",
        "--add-prefer-bare",
        "rights-marks",
        "--set-ignore",
        "none",
        "--remove-bare-as-text",
        "ascii",
        "--set-prefer-bare",
        "all",
        "input.txt",
    ]);

    assert_eq!(
        extract_ordered_operations(&matches),
        [
            OrderedOperation {
                id: OperationId::AddPreferBare,
                value: "rights-marks".to_owned(),
            },
            OrderedOperation {
                id: OperationId::SetIgnore,
                value: "none".to_owned(),
            },
            OrderedOperation {
                id: OperationId::RemoveBareAsText,
                value: "ascii".to_owned(),
            },
            OrderedOperation {
                id: OperationId::SetPreferBare,
                value: "all".to_owned(),
            },
        ]
    );
}

#[test]
fn repeated_operations_keep_repetition_order() {
    let matches = build_format_command().get_matches_from([
        "evfmt format",
        "--add-ignore",
        "git",
        "--add-ignore",
        "hidden",
    ]);

    assert_eq!(
        extract_ordered_operations(&matches),
        [
            OrderedOperation {
                id: OperationId::AddIgnore,
                value: "git".to_owned(),
            },
            OrderedOperation {
                id: OperationId::AddIgnore,
                value: "hidden".to_owned(),
            },
        ]
    );
}
