use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command, builder::ValueParser};

const PROG: &str = env!("CARGO_BIN_NAME");
const ROOT_HELP_FOOTER: &str = "\
Use `--` after a subcommand before file operands that would otherwise parse as options, for example: \
`evfmt format -- --set-ignore`.";
const FORMAT_HELP_FOOTER: &str = "\
Policy:
  bare-as-text: characters your reference platform renders as text when bare.
  prefer-bare: characters to keep bare when that preserves the chosen presentation.

Values:
  CHARSET: ascii, text-defaults, emoji-defaults, rights-marks, arrows, card-suits, u(HEX), \
or a single character.
  FILTER: git, evfmt, or hidden.
  Use all for every CHARSET or FILTER. Use none to clear a set with --set-*.

Use `--` before file operands that would otherwise parse as options, for example: \
`evfmt format -- --set-ignore`.";
const CHECK_HELP_FOOTER: &str = "\
Policy:
  bare-as-text: characters your reference platform renders as text when bare.
  prefer-bare: characters to keep bare when that preserves the chosen presentation.

Values:
  CHARSET: ascii, text-defaults, emoji-defaults, rights-marks, arrows, card-suits, u(HEX), \
or a single character.
  FILTER: git, evfmt, or hidden.
  Use all for every CHARSET or FILTER. Use none to clear a set with --set-*.

Use `--` before file operands that would otherwise parse as options, for example: \
`evfmt check -- --set-ignore`.";

#[derive(Debug, PartialEq)]
pub(crate) struct OrderedOperation {
    pub(crate) id: OperationId,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

pub(crate) struct SharedArgs {
    pub files: Vec<PathBuf>,
}

pub(crate) struct ParsedCommand {
    pub(crate) args: SharedArgs,
    pub(crate) check: bool,
    pub(crate) ordered_operations: Vec<OrderedOperation>,
}

#[derive(Clone, Copy)]
struct StatefulArg {
    help_heading: &'static str,
    value_name: &'static str,
    operations: &'static [StatefulOperation],
}

#[derive(Clone, Copy)]
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
        help: "Require explicit variation selectors for these bare characters",
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
    let matches = build_root_command().get_matches();

    let (check, matches) = match matches.subcommand() {
        Some(("format", matches)) => (false, matches),
        Some(("check", matches)) => (true, matches),
        Some((name, _)) => unreachable!("unexpected clap subcommand: {name}"),
        None => unreachable!("root command requires a subcommand"),
    };

    ParsedCommand {
        args: parse_shared_args(matches),
        check,
        ordered_operations: extract_ordered_operations(matches),
    }
}

fn build_root_command() -> Command {
    Command::new(PROG)
        .about("Emoji Variation Formatter")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(build_format_command())
        .subcommand(build_check_command())
        .after_help(ROOT_HELP_FOOTER)
}

fn build_format_command() -> Command {
    add_shared_args(
        Command::new("format")
            .bin_name(format!("{PROG} format"))
            .display_name(format!("{PROG} format"))
            .about("Format files in place")
            .version(env!("CARGO_PKG_VERSION"))
            .after_help(FORMAT_HELP_FOOTER),
        "Files to format (use `-` for stdin/stdout)",
    )
}

fn build_check_command() -> Command {
    add_shared_args(
        Command::new("check")
            .bin_name(format!("{PROG} check"))
            .display_name(format!("{PROG} check"))
            .about("Check whether formatting changes would be required")
            .version(env!("CARGO_PKG_VERSION"))
            .after_help(CHECK_HELP_FOOTER),
        "Files to check (use `-` for stdin)",
    )
}

fn add_shared_args(mut command: Command, file_help: &'static str) -> Command {
    for spec in STATEFUL_ARGS {
        command = add_stateful_arg(command, spec);
    }

    command.next_help_heading("Input").arg(
        Arg::new("files")
            .value_name("FILES")
            .help(file_help)
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
    #[allow(clippy::expect_used)]
    let values = matches
        .get_many::<String>(operation.arg_id)
        .expect("clap returned indices without values for an append-valued argument");

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
mod tests;
