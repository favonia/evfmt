use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser};

const PROG: &str = env!("CARGO_BIN_NAME");

pub(crate) const RESERVED_COMMANDS: [&str; 1] = ["check"];

#[derive(Parser)]
#[command(
    name = "evfmt",
    about = "Emoji Variation Formatter",
    version,
    after_help = r#"Commands:
  check           Check whether formatting changes are needed

Reserved command names are ambiguous as file operands. Use `--` before files when needed, for example: `evfmt -- check`."#
)]
struct FormatCli {
    /// Check whether formatting changes would be required
    #[arg(long)]
    check: bool,

    #[command(flatten)]
    args: SharedArgs,
}

#[derive(Parser)]
#[command(
    name = "evfmt check",
    about = "Check whether formatting changes would be required",
    version,
    after_help = "Use `evfmt -- check` when `check` is a file name."
)]
struct CheckCli {
    #[command(flatten)]
    args: SharedArgs,
}

#[derive(Args)]
pub(crate) struct SharedArgs {
    /// Print expression language reference and exit
    #[arg(long = "help-expression")]
    pub help_expression: bool,

    /// Do not respect ignore files (.gitignore, .evfmtignore, etc.)
    #[arg(long, overrides_with = "ignore")]
    pub no_ignore: bool,

    /// Re-enable ignore file processing (overrides --no-ignore)
    #[arg(long, overrides_with = "no_ignore", hide = true)]
    pub ignore: bool,

    /// Expression for characters whose bare form is preferred (canonical)
    #[arg(long = "prefer-bare-for", value_name = "EXPR", default_value = "ascii")]
    pub prefer_bare_for: String,

    /// Expression for characters whose bare form is treated as text presentation
    #[arg(
        long = "treat-bare-as-text-for",
        value_name = "EXPR",
        default_value = "ascii"
    )]
    pub treat_bare_as_text_for: String,

    /// Files to format (use `-` for stdin/stdout)
    pub files: Vec<PathBuf>,
}

pub(crate) struct ParsedCommand {
    pub(crate) args: SharedArgs,
    pub(crate) check: bool,
    pub(crate) allow_reserved_files: bool,
}

#[must_use]
pub(crate) fn parse_command() -> ParsedCommand {
    let args: Vec<OsString> = std::env::args_os().collect();

    if let Some("check") = args.get(1).and_then(|arg| arg.to_str()) {
        let allow_reserved_files = args.iter().skip(2).any(|arg| arg == "--");
        let cli = CheckCli::parse_from(
            std::iter::once(OsString::from(format!("{PROG} check")))
                .chain(args.into_iter().skip(2)),
        );
        ParsedCommand {
            args: cli.args,
            check: true,
            allow_reserved_files,
        }
    } else {
        let allow_reserved_files = args.iter().skip(1).any(|arg| arg == "--");
        let cli = FormatCli::parse_from(args);
        ParsedCommand {
            args: cli.args,
            check: cli.check,
            allow_reserved_files,
        }
    }
}
