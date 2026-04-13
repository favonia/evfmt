use std::fmt::Write as _;
use std::fs;
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use tempfile::NamedTempFile;

use evfmt::Policy;
use evfmt::charset;
use evfmt::charset::CharSet;
use evfmt::charset::is_variation_sequence_character;
use evfmt::formatter::{self, FormatResult};

use crate::cli_args::{
    OperationId, OrderedOperation, ParsedCommand, RESERVED_COMMANDS, SharedArgs,
};

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

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
pub(crate) fn run(command: &ParsedCommand) -> ExitStatus {
    let args = &command.args;

    if let Err(message) = validate_reserved_names(args, command.allow_reserved_files) {
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
        if command.check && !command.allow_reserved_files {
            eprintln!(
                "{PROG}: no files specified \
                 (if you meant a file named `check`, use `evfmt -- check`)"
            );
        } else {
            eprintln!("{PROG}: no files specified");
        }
        return ExitStatus::UsageOrIoError;
    }

    let mut had_error = false;
    let Ok(settings) = build_runtime_settings(&command.ordered_operations) else {
        return ExitStatus::UsageOrIoError;
    };
    let files = expand_paths(&args.files, settings.ignore, &mut had_error);

    let mut any_changed = false;

    if has_stdin
        && let Some(changed) = process_stdin(&settings.policy, command.check, &mut had_error)
    {
        any_changed |= changed;
    }

    for path in &files {
        any_changed |= process_file(path, &settings.policy, command.check, &mut had_error);
    }

    if had_error {
        ExitStatus::UsageOrIoError
    } else if command.check && any_changed {
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
            "`{reserved}` is reserved as a subcommand; use `--` before file operands, \
             for example `evfmt -- {reserved}`"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeOperation {
    PreferBare(UpdateKind),
    BareAsText(UpdateKind),
    Ignore(UpdateKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateKind {
    Set,
    Add,
    Remove,
}

struct RuntimeSettings {
    policy: Policy,
    ignore: IgnoreSettings,
}

fn build_runtime_settings(operations: &[OrderedOperation]) -> Result<RuntimeSettings, ()> {
    let mut prefer_bare = charset::ASCII;
    let mut bare_as_text = charset::ASCII;
    let mut ignore = IgnoreSettings::default();

    for operation in operations {
        match operation.id.runtime_operation() {
            RuntimeOperation::PreferBare(kind) => {
                let parsed = parse_charset_list(kind, &operation.value)
                    .map_err(|error| report_usage_error(operation.id.flag_name(), &error))?;
                prefer_bare = apply_charset_update(prefer_bare, kind, parsed);
            }
            RuntimeOperation::BareAsText(kind) => {
                let parsed = parse_charset_list(kind, &operation.value)
                    .map_err(|error| report_usage_error(operation.id.flag_name(), &error))?;
                bare_as_text = apply_charset_update(bare_as_text, kind, parsed);
            }
            RuntimeOperation::Ignore(kind) => {
                let parsed = parse_ignore_list(kind, &operation.value)
                    .map_err(|error| report_usage_error(operation.id.flag_name(), &error))?;
                apply_ignore_filter_update(&mut ignore, kind, &parsed);
            }
        }
    }

    Ok(RuntimeSettings {
        policy: Policy::default()
            .with_prefer_bare(prefer_bare)
            .with_bare_as_text(bare_as_text),
        ignore,
    })
}

fn apply_charset_update(current: CharSet, kind: UpdateKind, parsed: CharSet) -> CharSet {
    match kind {
        UpdateKind::Set => parsed,
        UpdateKind::Add => current | parsed,
        UpdateKind::Remove => current - parsed,
    }
}

fn apply_ignore_filter_update(
    settings: &mut IgnoreSettings,
    kind: UpdateKind,
    parsed: &[IgnoreLabel],
) {
    match kind {
        UpdateKind::Set => *settings = IgnoreSettings::from_labels(parsed),
        UpdateKind::Add => settings.enable(parsed),
        UpdateKind::Remove => settings.disable(parsed),
    }
}

fn report_usage_error(flag: &str, error: &CliParseError) {
    eprintln!("{PROG}: {flag}: {error}");
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliParseError {
    message: String,
}

impl std::fmt::Display for CliParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IgnoreLabel {
    Git,
    Evfmt,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IgnoreSettings {
    git: bool,
    evfmt: bool,
    hidden: bool,
}

impl Default for IgnoreSettings {
    fn default() -> Self {
        Self {
            git: true,
            evfmt: true,
            hidden: true,
        }
    }
}

impl IgnoreSettings {
    fn from_labels(labels: &[IgnoreLabel]) -> Self {
        let mut settings = Self {
            git: false,
            evfmt: false,
            hidden: false,
        };
        settings.enable(labels);
        settings
    }

    fn enable(&mut self, labels: &[IgnoreLabel]) {
        for label in labels {
            match label {
                IgnoreLabel::Git => self.git = true,
                IgnoreLabel::Evfmt => self.evfmt = true,
                IgnoreLabel::Hidden => self.hidden = true,
            }
        }
    }

    fn disable(&mut self, labels: &[IgnoreLabel]) {
        for label in labels {
            match label {
                IgnoreLabel::Git => self.git = false,
                IgnoreLabel::Evfmt => self.evfmt = false,
                IgnoreLabel::Hidden => self.hidden = false,
            }
        }
    }
}

fn parse_charset_list(kind: UpdateKind, input: &str) -> Result<CharSet, CliParseError> {
    let items = split_list_items(input)?;
    if items.len() == 1 {
        match items[0] {
            "all" => return Ok(CharSet::all()),
            "none" if kind == UpdateKind::Set => return Ok(CharSet::none()),
            "none" => {
                return Err(CliParseError {
                    message: "`none` is only allowed with `--set-*`".to_owned(),
                });
            }
            _ => {}
        }
    }

    if items.iter().any(|item| *item == "all" || *item == "none") {
        return Err(CliParseError {
            message: "`all` and `none` must appear alone".to_owned(),
        });
    }

    let mut set = CharSet::none();
    for item in items {
        set |= parse_charset_item(item)?;
    }

    Ok(set)
}

fn split_list_items(input: &str) -> Result<Vec<&str>, CliParseError> {
    if input.trim().is_empty() {
        return Err(CliParseError {
            message: "empty list".to_owned(),
        });
    }

    let mut items = Vec::new();
    for raw_item in input.split(',') {
        let item = raw_item.trim();
        if item.is_empty() {
            return Err(CliParseError {
                message: "empty list item".to_owned(),
            });
        }
        items.push(item);
    }

    Ok(items)
}

fn parse_charset_item(item: &str) -> Result<CharSet, CliParseError> {
    if let Some(named_set) = parse_named_set(item) {
        return Ok(named_set);
    }

    if item.starts_with("u(") {
        return parse_code_point_item(item);
    }

    if let Some(ch) = parse_naked_single(item) {
        return parse_singleton_item(item, ch);
    }

    if looks_like_identifier(item) {
        let mut message = format!("unknown charset preset `{item}`");
        if let Some(suggestion) = suggest_name(item, &named_set_names()) {
            let _ = write!(message, "; did you mean `{suggestion}`?");
        }
        return Err(CliParseError { message });
    }

    Err(CliParseError {
        message: format!("invalid charset item `{item}`"),
    })
}

fn parse_ignore_list(kind: UpdateKind, input: &str) -> Result<Vec<IgnoreLabel>, CliParseError> {
    let items = split_list_items(input)?;
    let mut labels = Vec::with_capacity(items.len());

    if items.len() == 1 && items[0] == "none" {
        if kind == UpdateKind::Set {
            return Ok(labels);
        }
        return Err(CliParseError {
            message: "`none` is only allowed with `--set-ignore`".to_owned(),
        });
    }

    if items.len() == 1 && items[0] == "all" {
        return Ok(vec![
            IgnoreLabel::Git,
            IgnoreLabel::Evfmt,
            IgnoreLabel::Hidden,
        ]);
    }

    for item in items {
        let label = match item {
            "git" => IgnoreLabel::Git,
            "evfmt" => IgnoreLabel::Evfmt,
            "hidden" => IgnoreLabel::Hidden,
            _ => {
                let mut message = format!("unknown ignore label `{item}`");
                if let Some(suggestion) = suggest_name(item, &["git", "evfmt", "hidden"]) {
                    let _ = write!(message, "; did you mean `{suggestion}`?");
                }
                return Err(CliParseError { message });
            }
        };
        labels.push(label);
    }

    Ok(labels)
}

fn parse_named_set(item: &str) -> Option<CharSet> {
    match item {
        "ascii" => Some(charset::ASCII),
        "text-defaults" => Some(charset::TEXT_DEFAULTS),
        "emoji-defaults" => Some(charset::EMOJI_DEFAULTS),
        "rights-marks" => Some(charset::RIGHTS_MARKS),
        "arrows" => Some(charset::ARROWS),
        "card-suits" => Some(charset::CARD_SUITS),
        _ => None,
    }
}

fn parse_code_point_item(item: &str) -> Result<CharSet, CliParseError> {
    let Some(hex) = item
        .strip_prefix("u(")
        .and_then(|rest| rest.strip_suffix(')'))
    else {
        return Err(CliParseError {
            message: format!("invalid code point item `{item}`"),
        });
    };
    if !(4..=6).contains(&hex.len()) || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(CliParseError {
            message: format!("invalid code point item `{item}`"),
        });
    }
    #[allow(clippy::expect_used)]
    let value =
        u32::from_str_radix(hex, 16).expect("validated 4-6 ASCII hex digits always fit in u32");
    let Some(ch) = char::from_u32(value) else {
        return Err(CliParseError {
            message: format!("invalid code point item `{item}`"),
        });
    };
    parse_singleton_item(item, ch)
}

fn parse_singleton_item(item: &str, ch: char) -> Result<CharSet, CliParseError> {
    if !is_variation_sequence_character(ch) {
        return Err(CliParseError {
            message: format!("character `{item}` is not eligible for emoji variation selectors"),
        });
    }
    Ok(CharSet::singleton(ch))
}

fn named_set_names() -> [&'static str; 6] {
    [
        "ascii",
        "text-defaults",
        "emoji-defaults",
        "rights-marks",
        "arrows",
        "card-suits",
    ]
}

fn parse_naked_single(item: &str) -> Option<char> {
    let mut base = None;
    for ch in item.chars() {
        if ch == '\u{FE0E}' || ch == '\u{FE0F}' {
            continue;
        }
        if base.replace(ch).is_some() {
            return None;
        }
    }
    base
}

fn looks_like_identifier(item: &str) -> bool {
    item.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn suggest_name<'a>(input: &str, choices: &'a [&str]) -> Option<&'a str> {
    let mut best = None;
    let mut best_distance = usize::MAX;
    for &choice in choices {
        let distance = edit_distance(input, choice);
        if distance < best_distance {
            best_distance = distance;
            best = Some(choice);
        }
    }

    if best_distance <= 3 { best } else { None }
}

/// Note: swapping counts as distance 2
fn edit_distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let mut prev: Vec<usize> = (0..=right.len()).collect();
    let mut curr = vec![0; right.len() + 1];

    for (i, lch) in left.iter().enumerate() {
        curr[0] = i + 1;
        for (j, rch) in right.iter().enumerate() {
            let substitution_cost = usize::from(lch != rch);
            curr[j + 1] = (prev[j + 1] + 1)
                .min(curr[j] + 1)
                .min(prev[j] + substitution_cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[right.len()]
}

impl OperationId {
    const fn runtime_operation(self) -> RuntimeOperation {
        match self {
            Self::SetPreferBare => RuntimeOperation::PreferBare(UpdateKind::Set),
            Self::AddPreferBare => RuntimeOperation::PreferBare(UpdateKind::Add),
            Self::RemovePreferBare => RuntimeOperation::PreferBare(UpdateKind::Remove),
            Self::SetBareAsText => RuntimeOperation::BareAsText(UpdateKind::Set),
            Self::AddBareAsText => RuntimeOperation::BareAsText(UpdateKind::Add),
            Self::RemoveBareAsText => RuntimeOperation::BareAsText(UpdateKind::Remove),
            Self::SetIgnore => RuntimeOperation::Ignore(UpdateKind::Set),
            Self::AddIgnore => RuntimeOperation::Ignore(UpdateKind::Add),
            Self::RemoveIgnore => RuntimeOperation::Ignore(UpdateKind::Remove),
        }
    }

    const fn flag_name(self) -> &'static str {
        match self {
            Self::SetPreferBare => "--set-prefer-bare",
            Self::AddPreferBare => "--add-prefer-bare",
            Self::RemovePreferBare => "--remove-prefer-bare",
            Self::SetBareAsText => "--set-bare-as-text",
            Self::AddBareAsText => "--add-bare-as-text",
            Self::RemoveBareAsText => "--remove-bare-as-text",
            Self::SetIgnore => "--set-ignore",
            Self::AddIgnore => "--add-ignore",
            Self::RemoveIgnore => "--remove-ignore",
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
            match atomic_write(path, &new_content) {
                Ok(warnings) => {
                    for warning in warnings {
                        eprintln!("{PROG}: {display_name}: warning: {warning}");
                    }
                }
                Err(error) => {
                    eprintln!("{PROG}: {display_name}: {error}");
                    *had_error = true;
                }
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

fn expand_paths(
    operands: &[PathBuf],
    ignore_settings: IgnoreSettings,
    had_error: &mut bool,
) -> Vec<PathBuf> {
    let fs_operands: Vec<&PathBuf> = operands.iter().filter(|f| f.as_os_str() != "-").collect();

    if fs_operands.is_empty() {
        return Vec::new();
    }

    let mut builder = WalkBuilder::new(fs_operands[0]);
    for operand in &fs_operands[1..] {
        builder.add(operand);
    }

    builder.sort_by_file_path(Ord::cmp);

    builder.hidden(ignore_settings.hidden);

    if ignore_settings.git {
        builder.git_ignore(true).git_global(true).git_exclude(true);
    } else {
        builder
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false);
    }

    if ignore_settings.evfmt {
        builder.add_custom_ignore_filename(format!(".{PROG}ignore"));
    } else {
        builder.ignore(false);
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

// Write content to a file atomically via a temp file + rename.
//
// AUDIT NOTE: write-then-rename avoids partial writes on crash. The temp file
// is in the same directory to guarantee same-filesystem rename. On failure,
// the temp file is cleaned up.
//
// DESIGN NOTE: the goal is to approximate the observable behavior of an
// in-place rewrite while keeping atomic replacement. In practice that means
// preserving access-control and security-relevant metadata that an in-place
// write would typically leave alone, while still letting normal rewrite
// effects such as updated modification/change times happen naturally. This
// cannot fully match true in-place writing because rename-based replacement
// swaps the inode; hard-link identity and other inode-bound behavior are not
// preserved.
//
// Metadata is copied onto the temp file before the rename so the replacement
// preserves the original file's permissions and, on Unix, best-effort
// ownership and extended attributes.
fn atomic_write(path: &Path, content: &str) -> Result<Vec<String>, String> {
    let dir = path.parent().unwrap_or(path);
    let mut temp_file =
        create_temp_file(dir).map_err(|error| format!("temp-file create error: {error}"))?;

    if let Err(error) = temp_file.as_file_mut().write_all(content.as_bytes()) {
        return Err(format!("write error: {error}"));
    }

    let warnings = preserve_metadata(path, temp_file.path())?;

    if let Err(error) = temp_file.persist(path) {
        return Err(format!("rename error: {error}"));
    }

    Ok(warnings)
}

fn create_temp_file(dir: &Path) -> io::Result<NamedTempFile> {
    tempfile::Builder::new()
        .prefix(&format!(".{PROG}-tmp-"))
        .tempfile_in(dir)
}

fn preserve_metadata(path: &Path, temp_path: &Path) -> Result<Vec<String>, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("metadata read error: {error}"))?;

    fs::set_permissions(temp_path, metadata.permissions())
        .map_err(|error| format!("permission preserve error: {error}"))?;

    #[cfg(unix)]
    return Ok(preserve_unix_metadata(path, temp_path, &metadata));

    #[cfg(not(unix))]
    Ok(Vec::new())
}

#[cfg(unix)]
fn preserve_unix_metadata(path: &Path, temp_path: &Path, metadata: &fs::Metadata) -> Vec<String> {
    use std::os::unix::fs::MetadataExt as _;

    let mut warnings = Vec::new();

    if let Err(warning) = preserve_xattrs(path, temp_path) {
        warnings.push(warning);
    }

    if let Err(error) = unix_fs::chown(temp_path, Some(metadata.uid()), Some(metadata.gid())) {
        warnings.push(format!("ownership preserve failed: {error}"));
    }

    warnings
}

#[cfg(unix)]
fn preserve_xattrs(path: &Path, temp_path: &Path) -> Result<(), String> {
    for attr in xattr::list(path).map_err(|error| format!("xattr list error: {error}"))? {
        let attr_display = Path::new(&attr).display();
        let value = xattr::get(path, &attr)
            .map_err(|error| format!("xattr read error for {attr_display}: {error}"))?;
        xattr::set(temp_path, &attr, value.as_deref().unwrap_or_default())
            .map_err(|error| format!("xattr preserve error for {attr_display}: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
