// This binary crate's user interface is stdout/stderr, and it has no public API
// surface to document; the library crate owns the reusable API.
#![allow(clippy::print_stdout, clippy::print_stderr, missing_docs)]

use std::process;

mod cli_args;
mod cli_run;

fn main() {
    let command = cli_args::parse_command();
    let status = cli_run::run(&command);
    process::exit(status.code());
}
