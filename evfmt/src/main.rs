// Binary crate: printing to stdout/stderr is the primary user interface,
// and there is no public API to document.
#![allow(clippy::print_stdout, clippy::print_stderr, missing_docs)]

use std::process;

mod cli_args;
mod cli_run;

fn main() {
    let command = cli_args::parse_command();
    let status = cli_run::run(&command.args, command.check, command.allow_reserved_files);
    process::exit(status.code());
}
