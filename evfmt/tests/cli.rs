// tests/cli.rs — CLI integration tests.
//
// These tests exercise the evfmt binary end-to-end using assert_cmd
// and assert_fs. They cover the full public CLI contract:
//   - `format` mode (in-place rewrite), `check` mode (exit 1 if changes needed)
//   - stdin/stdout via `-`
//   - Error cases: invalid UTF-8, partial failure
//   - Ordered set operations for policy and ignore filters
//   - Directory traversal, .evfmtignore, hidden files

// This integration test file uses `unwrap` pervasively for fixture setup.
// A setup failure should fail the test immediately, and localizing each call
// would obscure the CLI behavior each test is asserting.
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::PredicateBooleanExt;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

fn evfmt() -> Command {
    Command::cargo_bin("evfmt").unwrap()
}

fn format_command() -> Command {
    let mut command = evfmt();
    command.arg("format");
    command
}

fn check_command() -> Command {
    let mut command = evfmt();
    command.arg("check");
    command
}

#[cfg(unix)]
struct RestoreMode {
    path: std::path::PathBuf,
    mode: u32,
}

#[cfg(unix)]
impl Drop for RestoreMode {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(self.mode));
    }
}

// --- Help output ---

#[test]
fn root_help_lists_subcommands() {
    evfmt()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("format"))
        .stdout(predicates::str::contains("check"))
        .stdout(predicates::str::contains("--set-prefer-bare").not());
}

#[test]
fn format_help_describes_stateful_options() {
    format_command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Policy [prefer-bare]:"))
        .stdout(predicates::str::contains("Policy [bare-as-text]:"))
        .stdout(predicates::str::contains(
            "--set-prefer-bare <VARIATIONSET[,VARIATIONSET]...>",
        ))
        .stdout(predicates::str::contains(
            "--set-bare-as-text <VARIATIONSET[,VARIATIONSET]...>",
        ))
        .stdout(predicates::str::contains(
            "--set-ignore <FILTER[,FILTER]...>",
        ))
        .stdout(predicates::str::contains(
            "VARIATIONSET: ascii, text-defaults, emoji-defaults, rights-marks, arrows, card-suits,",
        ))
        .stdout(predicates::str::contains(
            "keycap-chars, non-keycap-chars, keycap-emojis, u(HEX), or a single character.",
        ))
        .stdout(predicates::str::contains("FILTER: git, evfmt, or hidden."))
        .stdout(predicates::str::contains(
            "Use all for every VARIATIONSET or FILTER. Use none to clear a set with --set-*.",
        ))
        .stdout(predicates::str::contains("Unicode defaults").not())
        .stdout(predicates::str::contains("--help-expression").not());
}

#[test]
fn check_help_describes_stateful_options() {
    check_command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Policy [prefer-bare]:"))
        .stdout(predicates::str::contains("Policy [bare-as-text]:"))
        .stdout(predicates::str::contains(
            "--set-prefer-bare <VARIATIONSET[,VARIATIONSET]...>",
        ))
        .stdout(predicates::str::contains(
            "--set-ignore <FILTER[,FILTER]...>",
        ))
        .stdout(predicates::str::contains(
            "VARIATIONSET: ascii, text-defaults, emoji-defaults, rights-marks, arrows, card-suits,",
        ))
        .stdout(predicates::str::contains(
            "keycap-chars, non-keycap-chars, keycap-emojis, u(HEX), or a single character.",
        ))
        .stdout(predicates::str::contains("FILTER: git, evfmt, or hidden."))
        .stdout(predicates::str::contains(
            "Use all for every VARIATIONSET or FILTER. Use none to clear a set with --set-*.",
        ))
        .stdout(predicates::str::contains("Unicode defaults").not())
        .stdout(predicates::str::contains("--help-expression").not());
}

// --- `format` mode ---

#[test]
fn format_rewrites_file() {
    let tmp = assert_fs::TempDir::new().unwrap();
    // ©️ is eligible, text-default in Unicode, not bare-preferred → gets FE0F with default policy
    // (bare-as-text='ascii' does not match non-ASCII ©️, so bare resolves to emoji).
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command().arg(file.path()).assert().success().code(0);

    file.assert("\u{00A9}\u{FE0F}");
}

#[test]
fn format_already_canonical_unchanged() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("Hello, world!").unwrap();

    format_command().arg(file.path()).assert().success().code(0);

    file.assert("Hello, world!");
}

#[cfg(unix)]
#[test]
fn format_reports_unreadable_file() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();
    let mode = std::fs::metadata(file.path()).unwrap().permissions().mode() & 0o777;
    let _restore = RestoreMode {
        path: file.path().to_owned(),
        mode,
    };
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(file.path()).is_ok() {
        return;
    }

    format_command()
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("test.txt"))
        .stderr(predicates::str::contains("Permission denied"));
}

#[cfg(unix)]
#[test]
fn format_preserves_mode_bits() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.sh");
    file.write_str("\u{00A9}").unwrap();
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o751)).unwrap();

    format_command().arg(file.path()).assert().success().code(0);

    file.assert("\u{00A9}\u{FE0F}");
    let mode = std::fs::metadata(file.path()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o751);
}

#[cfg(target_os = "linux")]
#[test]
fn format_preserves_extended_attributes_when_supported() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    if let Err(error) = xattr::set(file.path(), "user.evfmt-test", b"kept") {
        assert!(
            matches!(error.kind(), std::io::ErrorKind::Unsupported),
            "failed to set test xattr: {error}"
        );
        return;
    }

    format_command().arg(file.path()).assert().success().code(0);

    file.assert("\u{00A9}\u{FE0F}");
    let value = xattr::get(file.path(), "user.evfmt-test").unwrap();
    assert_eq!(value.as_deref(), Some(b"kept".as_slice()));
}

#[cfg(target_os = "linux")]
#[test]
fn format_warns_when_extended_attributes_cannot_be_preserved() {
    let tmp = assert_fs::TempDir::new().unwrap();

    let probe = tmp.child("probe.txt");
    probe.write_str("probe").unwrap();
    if let Err(error) = xattr::set(probe.path(), "user.evfmt-probe", b"kept") {
        assert!(
            matches!(error.kind(), std::io::ErrorKind::Unsupported),
            "failed to set test xattr: {error}"
        );
        return;
    }
    std::fs::set_permissions(probe.path(), std::fs::Permissions::from_mode(0o444)).unwrap();
    if xattr::set(probe.path(), "user.evfmt-probe-writable", b"probe").is_ok() {
        return;
    }

    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();
    xattr::set(file.path(), "user.evfmt-test", b"kept").unwrap();
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o444)).unwrap();

    format_command()
        .arg(file.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("warning: xattr preserve error"));

    file.assert("\u{00A9}\u{FE0F}");
}

// --- Check mode ---

#[test]
fn check_non_canonical_exits_1() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    check_command().arg(file.path()).assert().code(1);

    // File should not be modified in check mode.
    file.assert("\u{00A9}");
}

#[test]
fn check_canonical_exits_0() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("Hello, world!").unwrap();

    check_command().arg(file.path()).assert().success().code(0);
}

#[cfg(unix)]
#[test]
fn check_reports_unreadable_file() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();
    let mode = std::fs::metadata(file.path()).unwrap().permissions().mode() & 0o777;
    let _restore = RestoreMode {
        path: file.path().to_owned(),
        mode,
    };
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(file.path()).is_ok() {
        return;
    }

    check_command()
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("test.txt"))
        .stderr(predicates::str::contains("Permission denied"));
}

// --- stdin/stdout mode ---

#[test]
fn stdin_stdout_via_dash() {
    format_command()
        .arg("-")
        .write_stdin("\u{00A9}")
        .assert()
        .success()
        .stdout("\u{00A9}\u{FE0F}");
}

#[test]
fn format_without_files_reads_stdin() {
    format_command()
        .write_stdin("\u{00A9}")
        .assert()
        .success()
        .stdout("\u{00A9}\u{FE0F}");
}

#[test]
fn stdin_stdout_canonical_passthrough() {
    format_command()
        .arg("-")
        .write_stdin("Hello, world!")
        .assert()
        .success()
        .stdout("Hello, world!");
}

#[test]
fn check_stdin_non_canonical_exits_1_without_stdout() {
    check_command()
        .arg("-")
        .write_stdin("\u{00A9}")
        .assert()
        .code(1)
        .stdout("");
}

#[test]
fn check_stdin_canonical_does_not_echo_stdout() {
    check_command()
        .arg("-")
        .write_stdin("Hello, world!")
        .assert()
        .success()
        .stdout("");
}

#[test]
fn check_without_files_reads_stdin() {
    check_command()
        .write_stdin("\u{00A9}")
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicates::str::contains("<stdin> would be reformatted"));
}

#[test]
fn repeated_dash_reads_same_stdin_stream_to_eof() {
    format_command()
        .arg("-")
        .arg("-")
        .write_stdin("\u{00A9}")
        .assert()
        .success()
        .stdout("\u{00A9}\u{FE0F}");
}

#[test]
fn format_processes_file_stdin_file_operands() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let first = tmp.child("a.txt");
    let second = tmp.child("b.txt");
    first.write_str("\u{00A9}").unwrap();
    second.write_str("\u{00AE}").unwrap();

    format_command()
        .arg(first.path())
        .arg("-")
        .arg(second.path())
        .write_stdin("#\u{FE0E}")
        .assert()
        .success()
        .stdout("#");

    first.assert("\u{00A9}\u{FE0F}");
    second.assert("\u{00AE}\u{FE0F}");
}

#[test]
fn check_reports_file_stdin_file_operands_in_order() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let first = tmp.child("a.txt");
    let second = tmp.child("b.txt");
    first.write_str("\u{00A9}").unwrap();
    second.write_str("\u{00AE}").unwrap();

    let output = check_command()
        .arg(first.path())
        .arg("-")
        .arg(second.path())
        .write_stdin("#\u{FE0E}")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    let first_index = stderr.find("a.txt would be reformatted").unwrap();
    let stdin_index = stderr.find("<stdin> would be reformatted").unwrap();
    let second_index = stderr.find("b.txt would be reformatted").unwrap();
    assert!(first_index < stdin_index);
    assert!(stdin_index < second_index);
}

#[test]
fn dash_path_names_file_named_dash() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("-");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .current_dir(tmp.path())
        .arg("./-")
        .assert()
        .success()
        .stdout("");

    file.assert("\u{00A9}\u{FE0F}");
}

#[test]
fn stdin_preserves_missing_final_newline() {
    format_command()
        .write_stdin("\u{00A9}\n\u{00AE}")
        .assert()
        .success()
        .stdout("\u{00A9}\u{FE0F}\n\u{00AE}\u{FE0F}");
}

#[test]
fn file_formatting_preserves_crlf() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}\r\n\u{00AE}\r\n").unwrap();

    format_command().arg(file.path()).assert().success();

    file.assert("\u{00A9}\u{FE0F}\r\n\u{00AE}\u{FE0F}\r\n");
}

#[cfg(unix)]
#[test]
fn stdin_read_error_exits_2() {
    use assert_cmd::prelude::{CommandCargoExt as _, OutputAssertExt as _};

    let tmp = assert_fs::TempDir::new().unwrap();
    let stdin = std::fs::File::open(tmp.path()).unwrap();
    let mut command = std::process::Command::cargo_bin("evfmt").unwrap();
    command.arg("format").arg("-").stdin(stdin);

    command
        .assert()
        .code(2)
        .stderr(predicates::str::contains("<stdin>"));
}

// --- Error cases ---

#[test]
fn format_invalid_utf8_after_changed_line_does_not_rewrite_file() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("bad.bin");
    file.write_binary("\u{00A9}\n".as_bytes()).unwrap();
    let mut content = std::fs::read(file.path()).unwrap();
    content.extend_from_slice(&[0xFF, 0xFE, 0x80]);
    file.write_binary(&content).unwrap();

    format_command()
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "stream did not contain valid UTF-8",
        ));

    assert_eq!(std::fs::read(file.path()).unwrap(), content);
}

#[test]
fn invalid_utf8_exits_2() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("bad.bin");
    // Write invalid UTF-8 bytes.
    file.write_binary(&[0xFF, 0xFE, 0x80]).unwrap();

    format_command().arg(file.path()).assert().code(2);
}

#[test]
fn root_without_subcommand_exits_2() {
    evfmt()
        .assert()
        .code(2)
        .stderr(predicates::str::contains("format"))
        .stderr(predicates::str::contains("check"));
}

#[test]
fn check_invalid_utf8_after_changed_line_exits_2() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("bad.bin");
    file.write_binary("\u{00A9}\n".as_bytes()).unwrap();
    let mut content = std::fs::read(file.path()).unwrap();
    content.push(0xFF);
    file.write_binary(&content).unwrap();

    check_command()
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "stream did not contain valid UTF-8",
        ));
}

#[test]
fn format_subcommand_accepts_check_as_file_name() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("check");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .current_dir(tmp.path())
        .arg("check")
        .assert()
        .success()
        .code(0);

    file.assert("\u{00A9}\u{FE0F}");
}

#[test]
fn check_subcommand_accepts_check_as_file_name() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("check");
    file.write_str("\u{00A9}").unwrap();

    check_command()
        .current_dir(tmp.path())
        .arg("check")
        .assert()
        .code(1);

    file.assert("\u{00A9}");
}

#[test]
fn option_like_file_requires_separator_in_format_mode() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("--set-ignore");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .current_dir(tmp.path())
        .arg("--set-ignore")
        .assert()
        .code(2);

    file.assert("\u{00A9}");
}

#[test]
fn option_like_file_allowed_after_separator_in_format_mode() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("--set-ignore");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .current_dir(tmp.path())
        .arg("--")
        .arg("--set-ignore")
        .assert()
        .success()
        .code(0);

    file.assert("\u{00A9}\u{FE0F}");
}

#[test]
fn partial_failure_exits_2() {
    let tmp = assert_fs::TempDir::new().unwrap();

    // One valid file, one nonexistent file.
    let good = tmp.child("good.txt");
    good.write_str("hello").unwrap();

    format_command()
        .arg(good.path())
        .arg(tmp.child("nonexistent.txt").path())
        .assert()
        .code(2);
}

#[cfg(unix)]
#[test]
fn rewrite_reports_temp_file_create_error_when_directory_unwritable() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dir = tmp.child("locked");
    std::fs::create_dir(dir.path()).unwrap();
    let file = dir.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    let mode = std::fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777;
    let _restore = RestoreMode {
        path: dir.path().to_owned(),
        mode,
    };
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

    format_command()
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("temp-file create error"));

    file.assert("\u{00A9}");
}

// --- Ordered set operations ---

#[test]
fn set_prefer_bare_keeps_rights_mark_bare() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-prefer-bare=ascii,rights-marks")
        .arg("--set-bare-as-text=ascii,rights-marks")
        .arg(file.path())
        .assert()
        .success();

    file.assert("\u{00A9}");
}

#[test]
fn left_to_right_prefer_bare_operations_are_respected() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}\u{FE0F}").unwrap();

    format_command()
        .arg("--add-prefer-bare=rights-marks")
        .arg("--remove-prefer-bare=rights-marks")
        .arg("--add-bare-as-text=rights-marks")
        .arg(file.path())
        .assert()
        .success();

    file.assert("\u{00A9}\u{FE0F}");
}

#[test]
fn remove_prefer_bare_can_force_ascii_to_text() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("#").unwrap();

    format_command()
        .arg("--remove-prefer-bare=ascii")
        .arg(file.path())
        .assert()
        .success();

    file.assert("#\u{FE0E}");
}

#[test]
fn clearing_bare_as_text_and_prefer_bare_force_ascii_to_emoji() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("#").unwrap();

    format_command()
        .arg("--set-prefer-bare=none")
        .arg("--set-bare-as-text=none")
        .arg(file.path())
        .assert()
        .success();

    file.assert("#\u{FE0F}");
}

#[test]
fn add_prefer_bare_accepts_naked_single_with_selector() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--add-prefer-bare=\u{00A9}\u{FE0F}")
        .arg("--add-bare-as-text=\u{00A9}")
        .arg(file.path())
        .assert()
        .success();

    file.assert("\u{00A9}");
}

#[test]
fn unknown_preset_reports_suggestion() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-prefer-bare=arowws")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("did you mean `arrows`?"));

    file.assert("\u{00A9}");
}

#[test]
fn none_is_rejected_for_add_operations() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--add-prefer-bare=none")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("`none` is only allowed"));

    file.assert("\u{00A9}");
}

#[test]
fn ineligible_character_items_are_rejected() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--add-prefer-bare=u(0041)")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "not eligible for emoji variation selectors",
        ));

    file.assert("\u{00A9}");
}

#[test]
fn ineligible_bare_as_text_items_are_rejected() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-bare-as-text=u(0041)")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "not eligible for emoji variation selectors",
        ));

    file.assert("\u{00A9}");
}

#[test]
fn empty_variation_set_list_is_rejected_before_any_file_is_rewritten() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-prefer-bare=")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("--set-prefer-bare"))
        .stderr(predicates::str::contains("empty list"));

    file.assert("\u{00A9}");
}

#[test]
fn all_and_none_variation_set_shortcuts_must_appear_alone() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-bare-as-text=all,ascii")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "`all` and `none` must appear alone",
        ));

    file.assert("\u{00A9}");
}

#[test]
fn invalid_code_point_item_is_rejected() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-prefer-bare=u(110000)")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("invalid code point item"));

    file.assert("\u{00A9}");
}

#[test]
fn bare_as_text_operations_apply_left_to_right() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("#").unwrap();

    format_command()
        .arg("--set-prefer-bare=none")
        .arg("--set-bare-as-text=all")
        .arg("--remove-bare-as-text=ascii")
        .arg(file.path())
        .assert()
        .success();

    file.assert("#\u{FE0F}");
}

#[test]
fn add_bare_as_text_can_force_explicit_text_selector() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-prefer-bare=none")
        .arg("--set-bare-as-text=none")
        .arg("--add-bare-as-text=rights-marks")
        .arg(file.path())
        .assert()
        .success();

    file.assert("\u{00A9}\u{FE0E}");
}

// --- Directory traversal ---

#[test]
fn directory_walk_formats_recursively() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let a = tmp.child("a.txt");
    a.write_str("\u{00A9}").unwrap();
    tmp.child("sub")
        .child("b.txt")
        .write_str("\u{00A9}")
        .unwrap();

    format_command().arg(tmp.path()).assert().success().code(0);

    a.assert("\u{00A9}\u{FE0F}");
    tmp.child("sub").child("b.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn evfmtignore_skips_matched_files() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child(".evfmtignore").write_str("skip.txt\n").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    format_command().arg(tmp.path()).assert().success();

    tmp.child("skip.txt").assert("\u{00A9}");
    tmp.child("keep.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn remove_ignore_evfmt_overrides_evfmtignore() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child(".evfmtignore").write_str("skip.txt\n").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--remove-ignore=evfmt")
        .arg(tmp.path())
        .assert()
        .success();

    tmp.child("skip.txt").assert("\u{00A9}\u{FE0F}");
    tmp.child("keep.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn gitignore_skips_matched_files() {
    let tmp = assert_fs::TempDir::new().unwrap();
    std::fs::create_dir(tmp.child(".git").path()).unwrap();
    tmp.child(".gitignore").write_str("skip.txt\n").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    format_command().arg(tmp.path()).assert().success();

    tmp.child("skip.txt").assert("\u{00A9}");
    tmp.child("keep.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn remove_ignore_git_overrides_gitignore() {
    let tmp = assert_fs::TempDir::new().unwrap();
    std::fs::create_dir(tmp.child(".git").path()).unwrap();
    tmp.child(".gitignore").write_str("skip.txt\n").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--remove-ignore=git")
        .arg(tmp.path())
        .assert()
        .success();

    tmp.child("skip.txt").assert("\u{00A9}\u{FE0F}");
    tmp.child("keep.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn all_shortcut_applies_to_add_and_remove_ignore() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child(".evfmtignore").write_str("skip.txt\n").unwrap();
    tmp.child(".hidden.txt").write_str("\u{00A9}").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--remove-ignore=all")
        .arg("--add-ignore=git")
        .arg(tmp.path())
        .assert()
        .success();

    tmp.child(".hidden.txt").assert("\u{00A9}\u{FE0F}");
    tmp.child("skip.txt").assert("\u{00A9}\u{FE0F}");
    tmp.child("keep.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn set_ignore_none_then_add_hidden_reenables_only_hidden_filtering() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child(".evfmtignore").write_str("skip.txt\n").unwrap();
    tmp.child(".hidden.txt").write_str("\u{00A9}").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-ignore=none")
        .arg("--add-ignore=hidden")
        .arg(tmp.path())
        .assert()
        .success();

    tmp.child(".hidden.txt").assert("\u{00A9}");
    tmp.child("skip.txt").assert("\u{00A9}\u{FE0F}");
    tmp.child("keep.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn ignore_labels_apply_left_to_right() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child(".hidden.txt").write_str("\u{00A9}").unwrap();
    tmp.child("visible.txt").write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--remove-ignore=hidden")
        .arg("--add-ignore=hidden")
        .arg(tmp.path())
        .assert()
        .success();

    tmp.child(".hidden.txt").assert("\u{00A9}");
    tmp.child("visible.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn unknown_ignore_label_reports_suggestion() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-ignore=hdden")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("did you mean `hidden`?"));
}

#[test]
fn usage_errors_are_reported_in_cli_order() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    format_command()
        .arg("--set-prefer-bare=arowws")
        .arg("--set-ignore=hdden")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("--set-prefer-bare"))
        .stderr(predicates::str::contains("did you mean `arrows`?"))
        .stderr(predicates::str::contains("--set-ignore").not());
}

#[test]
fn hidden_files_skipped() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child("visible.txt").write_str("\u{00A9}").unwrap();
    tmp.child(".hidden.txt").write_str("\u{00A9}").unwrap();

    format_command().arg(tmp.path()).assert().success();

    tmp.child("visible.txt").assert("\u{00A9}\u{FE0F}");
    tmp.child(".hidden.txt").assert("\u{00A9}");
}

#[test]
fn mixed_file_and_directory_operands() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let solo = tmp.child("solo.txt");
    solo.write_str("\u{00A9}").unwrap();
    let dir = tmp.child("dir");
    dir.child("a.txt").write_str("\u{00A9}").unwrap();

    format_command()
        .arg(solo.path())
        .arg(dir.path())
        .assert()
        .success();

    solo.assert("\u{00A9}\u{FE0F}");
    dir.child("a.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn check_mode_with_directory() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child("a.txt").write_str("\u{00A9}").unwrap();

    check_command().arg(tmp.path()).assert().code(1);

    // File should not be modified in check mode.
    tmp.child("a.txt").assert("\u{00A9}");
}

#[test]
fn empty_directory_succeeds() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dir = tmp.child("empty");
    std::fs::create_dir(dir.path()).unwrap();

    format_command().arg(dir.path()).assert().success().code(0);
}
