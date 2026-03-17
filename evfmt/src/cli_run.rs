use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use evfmt::expr;
use evfmt::formatter::{self, FormatResult, Policy};

use crate::cli_args::{RESERVED_COMMANDS, SharedArgs};

const PROG: &str = env!("CARGO_BIN_NAME");

pub(crate) enum ExitStatus {
    Success,
    CheckFoundChanges,
    UsageOrIoError,
}

impl ExitStatus {
    #[must_use]
    pub(crate) const fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::CheckFoundChanges => 1,
            Self::UsageOrIoError => 2,
        }
    }
}

#[must_use]
pub(crate) fn run(args: &SharedArgs, check: bool, allow_reserved_files: bool) -> ExitStatus {
    if args.help_expression {
        print!("{}", expr::EXPRESSION_HELP);
        return ExitStatus::Success;
    }

    if let Err(message) = validate_reserved_names(args, allow_reserved_files) {
        eprintln!("{PROG}: {message}");
        return ExitStatus::UsageOrIoError;
    }

    let stdin_count = args.files.iter().filter(|f| f.as_os_str() == "-").count();
    if stdin_count > 1 {
        eprintln!("{PROG}: at most one `-` operand is allowed");
        return ExitStatus::UsageOrIoError;
    }
    let has_stdin = stdin_count == 1;

    if args.files.is_empty() && !has_stdin {
        if check && !allow_reserved_files {
            eprintln!(
                "{PROG}: no files specified (if you meant a file named `check`, use `evfmt -- check`)"
            );
        } else {
            eprintln!("{PROG}: no files specified");
        }
        return ExitStatus::UsageOrIoError;
    }

    let mut had_error = false;
    let files = expand_paths(&args.files, args.no_ignore, &mut had_error);

    let Ok(policy) = build_policy(args) else {
        return ExitStatus::UsageOrIoError;
    };

    let mut any_changed = false;

    if has_stdin && let Some(changed) = process_stdin(&policy, check, &mut had_error) {
        any_changed |= changed;
    }

    for path in &files {
        any_changed |= process_file(path, &policy, check, &mut had_error);
    }

    if had_error {
        ExitStatus::UsageOrIoError
    } else if check && any_changed {
        ExitStatus::CheckFoundChanges
    } else {
        ExitStatus::Success
    }
}

fn validate_reserved_names(args: &SharedArgs, allow_reserved_files: bool) -> Result<(), String> {
    if !allow_reserved_files
        && let Some(reserved) = args
            .files
            .iter()
            .filter_map(|path| path.to_str())
            .find(|path| RESERVED_COMMANDS.contains(path))
    {
        return Err(format!(
            "`{reserved}` is reserved as a subcommand; use `--` before file operands, for example `evfmt -- {reserved}`"
        ));
    }
    Ok(())
}

fn build_policy(args: &SharedArgs) -> Result<Policy, ()> {
    Ok(Policy::default()
        .with_prefer_bare_for(parse_policy_expr(
            "--prefer-bare-for",
            &args.prefer_bare_for,
        )?)
        .with_treat_bare_as_text_for(parse_policy_expr(
            "--treat-bare-as-text-for",
            &args.treat_bare_as_text_for,
        )?))
}

fn parse_policy_expr(flag: &str, input: &str) -> Result<expr::Expr, ()> {
    match expr::parse(input) {
        Ok(result) => {
            for warning in &result.warnings {
                eprintln!("{PROG}: {flag}: {warning}");
            }
            Ok(result.expr)
        }
        Err(error) => {
            eprintln!("{PROG}: {flag}: {error}");
            Err(())
        }
    }
}

fn process_stdin(policy: &Policy, check: bool, had_error: &mut bool) -> Option<bool> {
    match read_stdin() {
        Ok(content) => {
            let changed = emit_result(
                "<stdin>",
                &content,
                formatter::format_text(&content, policy),
                check,
            );
            Some(changed)
        }
        Err(error) => {
            eprintln!("{PROG}: <stdin>: {error}");
            *had_error = true;
            None
        }
    }
}

fn process_file(path: &Path, policy: &Policy, check: bool, had_error: &mut bool) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("{PROG}: {}: {error}", path.display());
            *had_error = true;
            return false;
        }
    };

    let display_name = path.display().to_string();
    let result = formatter::format_text(&content, policy);
    if check {
        return emit_result(&display_name, &content, result, true);
    }

    match result {
        FormatResult::Unchanged => false,
        FormatResult::Changed(new_content) => {
            if let Err(error) = atomic_write(path, &new_content) {
                eprintln!("{PROG}: {display_name}: {error}");
                *had_error = true;
            }
            true
        }
    }
}

fn emit_result(label: &str, original: &str, result: FormatResult, check: bool) -> bool {
    match result {
        FormatResult::Unchanged => {
            if !check && label == "<stdin>" {
                print!("{original}");
            }
            false
        }
        FormatResult::Changed(new_content) => {
            if check {
                eprintln!("{PROG}: {label} would be reformatted");
            } else if label == "<stdin>" {
                print!("{new_content}");
            }
            true
        }
    }
}

fn expand_paths(operands: &[PathBuf], no_ignore: bool, had_error: &mut bool) -> Vec<PathBuf> {
    let fs_operands: Vec<&PathBuf> = operands.iter().filter(|f| f.as_os_str() != "-").collect();

    if fs_operands.is_empty() {
        return Vec::new();
    }

    let mut builder = WalkBuilder::new(fs_operands[0]);
    for operand in &fs_operands[1..] {
        builder.add(operand);
    }

    builder.sort_by_file_path(Ord::cmp);

    if no_ignore {
        builder
            .ignore(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false);
    } else {
        builder.add_custom_ignore_filename(format!(".{PROG}ignore"));
    }

    let mut files = Vec::new();
    for entry in builder.build() {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    files.push(entry.into_path());
                }
            }
            Err(error) => {
                eprintln!("{PROG}: {error}");
                *had_error = true;
            }
        }
    }

    files
}

fn read_stdin() -> Result<String, io::Error> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

/// Write content to a file atomically via a temp file + rename.
///
/// AUDIT NOTE: write-then-rename avoids partial writes on crash. The temp file
/// is in the same directory to guarantee same-filesystem rename (no cross-device
/// copy). On failure, the temp file is cleaned up. File permissions/ownership
/// are NOT preserved — rename inherits the temp file's defaults.
fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let dir = path.parent().unwrap_or(path);
    let temp_path = dir.join(format!(
        ".{PROG}-tmp-{}",
        path.file_name()
            .map_or_else(|| "file".to_owned(), |n| n.to_string_lossy().to_string())
    ));

    if let Err(error) = fs::write(&temp_path, content) {
        return Err(format!("write error: {error}"));
    }

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("rename error: {error}"));
    }

    Ok(())
}
