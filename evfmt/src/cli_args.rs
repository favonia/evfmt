use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command, builder::ValueParser};

const PROG: &str = env!("CARGO_BIN_NAME");
const ROOT_HELP_FOOTER: &str = "\
Policy:
  bare-as-text: characters your reference platform renders as text when bare.
  prefer-bare: characters to keep bare when that preserves the chosen presentation.

Values:
  CHARSET: ascii, emoji-defaults, rights-marks, arrows, card-suits, u(HEX), \
or a single character.
  FILTER: git, evfmt, or hidden.
  Use all for every CHARSET or FILTER. Use none to clear a set with --set-*.

Commands:
  check           Check whether formatting changes are needed

Reserved command names are ambiguous as file operands. Use `--` before files \
when needed, for example: `evfmt -- check`.";
const CHECK_HELP_FOOTER: &str = "\
Policy:
  bare-as-text: characters your reference platform renders as text when bare.
  prefer-bare: characters to keep bare when that preserves the chosen presentation.

Values:
  CHARSET: ascii, emoji-defaults, rights-marks, arrows, card-suits, u(HEX), \
or a single character.
  FILTER: git, evfmt, or hidden.
  Use all for every CHARSET or FILTER. Use none to clear a set with --set-*.

Use `evfmt -- check` when `check` is a file name.";

pub(crate) const RESERVED_COMMANDS: [&str; 1] = ["check"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrderedOperation {
    pub(crate) id: OperationId,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperationId {
    SetPreferBare,
    AddPreferBare,
    RemovePreferBare,
    SetBareAsText,
    AddBareAsText,
    RemoveBareAsText,
    SetIgnore,
    AddIgnore,
    RemoveIgnore,
}

#[derive(Debug, Default)]
pub(crate) struct SharedArgs {
    pub files: Vec<PathBuf>,
}

pub(crate) struct ParsedCommand {
    pub(crate) args: SharedArgs,
    pub(crate) check: bool,
    pub(crate) allow_reserved_files: bool,
    pub(crate) ordered_operations: Vec<OrderedOperation>,
}

#[derive(Debug, Clone, Copy)]
struct StatefulArg {
    help_heading: &'static str,
    value_name: &'static str,
    operations: &'static [StatefulOperation],
}

#[derive(Debug, Clone, Copy)]
struct StatefulOperation {
    operation: OperationId,
    arg_id: &'static str,
    long: &'static str,
    help: &'static str,
}

const SET_ADD_REMOVE_OPERATIONS_PREFER_BARE: [StatefulOperation; 3] = [
    StatefulOperation {
        operation: OperationId::SetPreferBare,
        arg_id: "set_prefer_bare",
        long: "set-prefer-bare",
        help: "Replace the set of bare characters that are already canonical",
    },
    StatefulOperation {
        operation: OperationId::AddPreferBare,
        arg_id: "add_prefer_bare",
        long: "add-prefer-bare",
        help: "Add characters that may remain bare in canonical output",
    },
    StatefulOperation {
        operation: OperationId::RemovePreferBare,
        arg_id: "remove_prefer_bare",
        long: "remove-prefer-bare",
        help: "Require explicit selectors for these bare characters",
    },
];

const SET_ADD_REMOVE_OPERATIONS_BARE_AS_TEXT: [StatefulOperation; 3] = [
    StatefulOperation {
        operation: OperationId::SetBareAsText,
        arg_id: "set_bare_as_text",
        long: "set-bare-as-text",
        help: "Replace the set of bare characters interpreted as text",
    },
    StatefulOperation {
        operation: OperationId::AddBareAsText,
        arg_id: "add_bare_as_text",
        long: "add-bare-as-text",
        help: "Interpret these bare characters as text presentation",
    },
    StatefulOperation {
        operation: OperationId::RemoveBareAsText,
        arg_id: "remove_bare_as_text",
        long: "remove-bare-as-text",
        help: "Stop resolving these bare characters as text",
    },
];

const SET_ADD_REMOVE_OPERATIONS_IGNORE: [StatefulOperation; 3] = [
    StatefulOperation {
        operation: OperationId::SetIgnore,
        arg_id: "set_ignore",
        long: "set-ignore",
        help: "Replace enabled ignore filters",
    },
    StatefulOperation {
        operation: OperationId::AddIgnore,
        arg_id: "add_ignore",
        long: "add-ignore",
        help: "Add ignore filters",
    },
    StatefulOperation {
        operation: OperationId::RemoveIgnore,
        arg_id: "remove_ignore",
        long: "remove-ignore",
        help: "Remove ignore filters",
    },
];

const PREFER_BARE_ARG: StatefulArg = StatefulArg {
    help_heading: "Policy [prefer-bare]",
    value_name: "CHARSET[,CHARSET]...",
    operations: &SET_ADD_REMOVE_OPERATIONS_PREFER_BARE,
};

const BARE_AS_TEXT_ARG: StatefulArg = StatefulArg {
    help_heading: "Policy [bare-as-text]",
    value_name: "CHARSET[,CHARSET]...",
    operations: &SET_ADD_REMOVE_OPERATIONS_BARE_AS_TEXT,
};

const IGNORE_ARG: StatefulArg = StatefulArg {
    help_heading: "Ignore Filters",
    value_name: "FILTER[,FILTER]...",
    operations: &SET_ADD_REMOVE_OPERATIONS_IGNORE,
};

const STATEFUL_ARGS: [StatefulArg; 3] = [BARE_AS_TEXT_ARG, PREFER_BARE_ARG, IGNORE_ARG];

#[must_use]
pub(crate) fn parse_command() -> ParsedCommand {
    let raw_args: Vec<OsString> = std::env::args_os().collect();

    if let Some("check") = raw_args.get(1).and_then(|arg| arg.to_str()) {
        let allow_reserved_files = raw_args.iter().skip(2).any(|arg| arg == "--");
        let adjusted_args = std::iter::once(OsString::from(format!("{PROG} check")))
            .chain(raw_args.into_iter().skip(2))
            .collect::<Vec<_>>();
        let matches = build_check_command().get_matches_from(adjusted_args);
        ParsedCommand {
            args: parse_shared_args(&matches),
            check: true,
            allow_reserved_files,
            ordered_operations: extract_ordered_operations(&matches),
        }
    } else {
        let allow_reserved_files = raw_args.iter().skip(1).any(|arg| arg == "--");
        let matches = build_root_command().get_matches_from(raw_args);
        ParsedCommand {
            args: parse_shared_args(&matches),
            check: matches.get_flag("check"),
            allow_reserved_files,
            ordered_operations: extract_ordered_operations(&matches),
        }
    }
}

fn build_root_command() -> Command {
    let command = Command::new(PROG)
        .about("Emoji Variation Formatter")
        .version(env!("CARGO_PKG_VERSION"))
        .after_help(ROOT_HELP_FOOTER)
        .arg(
            Arg::new("check")
                .long("check")
                .help("Check whether formatting changes would be required")
                .action(ArgAction::SetTrue),
        );

    add_shared_args(command)
}

fn build_check_command() -> Command {
    add_shared_args(
        Command::new("evfmt check")
            .about("Check whether formatting changes would be required")
            .version(env!("CARGO_PKG_VERSION"))
            .after_help(CHECK_HELP_FOOTER),
    )
}

fn add_shared_args(mut command: Command) -> Command {
    for spec in STATEFUL_ARGS {
        command = add_stateful_arg(command, spec);
    }

    command.next_help_heading("Input").arg(
        Arg::new("files")
            .value_name("FILES")
            .help("Files to format (use `-` for stdin/stdout)")
            .value_parser(ValueParser::path_buf())
            .action(ArgAction::Append),
    )
}

fn add_stateful_arg(mut command: Command, spec: StatefulArg) -> Command {
    command = command.next_help_heading(spec.help_heading);
    for operation in spec.operations {
        command = command.arg(
            Arg::new(operation.arg_id)
                .long(operation.long)
                .value_name(spec.value_name)
                .help(operation.help)
                .action(ArgAction::Append),
        );
    }
    command
}

fn parse_shared_args(matches: &ArgMatches) -> SharedArgs {
    SharedArgs {
        files: matches
            .get_many::<PathBuf>("files")
            .map(|values| values.cloned().collect())
            .unwrap_or_default(),
    }
}

fn extract_ordered_operations(matches: &ArgMatches) -> Vec<OrderedOperation> {
    let mut indexed = Vec::new();

    for spec in STATEFUL_ARGS {
        for operation in spec.operations {
            collect_operations(matches, *operation, &mut indexed);
        }
    }

    indexed.sort_by_key(|(index, _)| *index);
    indexed
        .into_iter()
        .map(|(_, operation)| operation)
        .collect()
}

fn collect_operations(
    matches: &ArgMatches,
    operation: StatefulOperation,
    out: &mut Vec<(usize, OrderedOperation)>,
) {
    let Some(indices) = matches.indices_of(operation.arg_id) else {
        return;
    };
    let Some(values) = matches.get_many::<String>(operation.arg_id) else {
        return;
    };

    for (index, value) in indices.zip(values) {
        out.push((
            index,
            OrderedOperation {
                id: operation.operation,
                value: value.clone(),
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_command_parses_check_flag_and_files() {
        let matches =
            build_root_command().get_matches_from(["evfmt", "--check", "one.txt", "two.txt"]);

        assert!(matches.get_flag("check"));
        assert_eq!(
            parse_shared_args(&matches).files,
            [PathBuf::from("one.txt"), PathBuf::from("two.txt")]
        );
    }

    #[test]
    fn root_command_defaults_to_format_mode_without_files() {
        let matches = build_root_command().get_matches_from(["evfmt"]);

        assert!(!matches.get_flag("check"));
        assert!(parse_shared_args(&matches).files.is_empty());
        assert!(extract_ordered_operations(&matches).is_empty());
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
    fn separator_allows_reserved_command_name_as_file() {
        let matches = build_root_command().get_matches_from(["evfmt", "--", "check"]);

        assert_eq!(parse_shared_args(&matches).files, [PathBuf::from("check")]);
    }

    #[test]
    fn operations_are_extracted_in_cli_order_across_option_groups() {
        let matches = build_root_command().get_matches_from([
            "evfmt",
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
        let matches = build_root_command().get_matches_from([
            "evfmt",
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
}
