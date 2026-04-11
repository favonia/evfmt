// tests/cli.rs — CLI integration tests.
//
// These tests exercise the evfmt binary end-to-end using assert_cmd
// and assert_fs. They cover the full public CLI contract:
//   - Format mode (in-place rewrite), check mode (exit 1 if changes needed)
//   - stdin/stdout via `-`
//   - Error cases: multiple dashes, invalid UTF-8, no files, partial failure
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
fn help_describes_stateful_options() {
    evfmt()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Policy [prefer-bare]:"))
        .stdout(predicates::str::contains("Policy [bare-as-text]:"))
        .stdout(predicates::str::contains(
            "--set-prefer-bare <CHARSET[,CHARSET]...>",
        ))
        .stdout(predicates::str::contains(
            "--set-bare-as-text <CHARSET[,CHARSET]...>",
        ))
        .stdout(predicates::str::contains(
            "--set-ignore <FILTER[,FILTER]...>",
        ))
        .stdout(predicates::str::contains(
            "CHARSET: ascii, emoji-defaults, rights-marks, arrows, card-suits, u(HEX), or a single character.",
        ))
        .stdout(predicates::str::contains(
            "FILTER: git, evfmt, or hidden.",
        ))
        .stdout(predicates::str::contains(
            "Use all for every CHARSET or FILTER. Use none to clear a set with --set-*.",
        ))
        .stdout(predicates::str::contains("Unicode defaults").not())
        .stdout(predicates::str::contains("--help-expression").not());
}

#[test]
fn check_help_describes_stateful_options() {
    evfmt()
        .arg("check")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Policy [prefer-bare]:"))
        .stdout(predicates::str::contains("Policy [bare-as-text]:"))
        .stdout(predicates::str::contains(
            "--set-prefer-bare <CHARSET[,CHARSET]...>",
        ))
        .stdout(predicates::str::contains(
            "--set-ignore <FILTER[,FILTER]...>",
        ))
        .stdout(predicates::str::contains(
            "CHARSET: ascii, emoji-defaults, rights-marks, arrows, card-suits, u(HEX), or a single character.",
        ))
        .stdout(predicates::str::contains(
            "FILTER: git, evfmt, or hidden.",
        ))
        .stdout(predicates::str::contains(
            "Use all for every CHARSET or FILTER. Use none to clear a set with --set-*.",
        ))
        .stdout(predicates::str::contains("Unicode defaults").not())
        .stdout(predicates::str::contains("--help-expression").not());
}

// --- Format mode ---

#[test]
fn format_rewrites_file() {
    let tmp = assert_fs::TempDir::new().unwrap();
    // ©️ is eligible, text-default in Unicode, not bare-preferred → gets FE0F with default policy
    // (bare-as-text='ascii' does not match non-ASCII ©️, so bare resolves to emoji).
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    evfmt().arg(file.path()).assert().success().code(0);

    file.assert("\u{00A9}\u{FE0F}");
}

#[test]
fn format_already_canonical_unchanged() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("Hello, world!").unwrap();

    evfmt().arg(file.path()).assert().success().code(0);

    file.assert("Hello, world!");
}

#[cfg(unix)]
#[test]
fn format_preserves_mode_bits() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.sh");
    file.write_str("\u{00A9}").unwrap();
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o751)).unwrap();

    evfmt().arg(file.path()).assert().success().code(0);

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

    evfmt().arg(file.path()).assert().success().code(0);

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

    evfmt()
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

    evfmt().arg("--check").arg(file.path()).assert().code(1);

    // File should not be modified in check mode.
    file.assert("\u{00A9}");
}

#[test]
fn check_canonical_exits_0() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("Hello, world!").unwrap();

    evfmt()
        .arg("--check")
        .arg(file.path())
        .assert()
        .success()
        .code(0);
}

#[test]
fn check_subcommand_non_canonical_exits_1() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    evfmt().arg("check").arg(file.path()).assert().code(1);

    file.assert("\u{00A9}");
}

// --- stdin/stdout mode ---

#[test]
fn stdin_stdout_via_dash() {
    evfmt()
        .arg("-")
        .write_stdin("\u{00A9}")
        .assert()
        .success()
        .stdout("\u{00A9}\u{FE0F}");
}

#[test]
fn stdin_stdout_canonical_passthrough() {
    evfmt()
        .arg("-")
        .write_stdin("Hello, world!")
        .assert()
        .success()
        .stdout("Hello, world!");
}

#[test]
fn check_stdin_non_canonical_exits_1_without_stdout() {
    evfmt()
        .arg("--check")
        .arg("-")
        .write_stdin("\u{00A9}")
        .assert()
        .code(1)
        .stdout("");
}

#[test]
fn check_stdin_canonical_does_not_echo_stdout() {
    evfmt()
        .arg("--check")
        .arg("-")
        .write_stdin("Hello, world!")
        .assert()
        .success()
        .stdout("");
}

#[cfg(unix)]
#[test]
fn stdin_read_error_exits_2() {
    use assert_cmd::prelude::{CommandCargoExt as _, OutputAssertExt as _};

    let tmp = assert_fs::TempDir::new().unwrap();
    let stdin = std::fs::File::open(tmp.path()).unwrap();
    let mut command = std::process::Command::cargo_bin("evfmt").unwrap();
    command.arg("-").stdin(stdin);

    command
        .assert()
        .code(2)
        .stderr(predicates::str::contains("<stdin>"));
}

// --- Error cases ---

#[test]
fn multiple_dash_rejected() {
    evfmt()
        .arg("-")
        .arg("-")
        .write_stdin("")
        .assert()
        .code(2)
        .stderr(predicates::str::contains("at most one `-`"));
}

#[test]
fn invalid_utf8_exits_2() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("bad.bin");
    // Write invalid UTF-8 bytes.
    file.write_binary(&[0xFF, 0xFE, 0x80]).unwrap();

    evfmt().arg(file.path()).assert().code(2);
}

#[test]
fn no_files_exits_2() {
    evfmt()
        .assert()
        .code(2)
        .stderr(predicates::str::contains("no files"))
        .stderr(predicates::str::contains("if you meant").not());
}

#[test]
fn reserved_command_name_requires_separator() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("check");
    file.write_str("\u{00A9}").unwrap();

    evfmt()
        .current_dir(tmp.path())
        .arg("check")
        .assert()
        .code(2)
        .stderr(predicates::str::contains("use `evfmt -- check`"));
}

#[test]
fn reserved_file_operand_in_format_mode_requires_separator() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child("plain.txt").write_str("Hello").unwrap();
    tmp.child("check").write_str("\u{00A9}").unwrap();

    evfmt()
        .current_dir(tmp.path())
        .arg("plain.txt")
        .arg("check")
        .assert()
        .code(2)
        .stderr(predicates::str::contains("for example `evfmt -- check`"));
}

#[test]
fn reserved_file_operand_in_check_subcommand_requires_separator() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child("plain.txt").write_str("Hello").unwrap();
    tmp.child("check").write_str("\u{00A9}").unwrap();

    evfmt()
        .current_dir(tmp.path())
        .arg("check")
        .arg("plain.txt")
        .arg("check")
        .assert()
        .code(2)
        .stderr(predicates::str::contains("for example `evfmt -- check`"));
}

#[test]
fn reserved_command_name_allowed_after_separator() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("check");
    file.write_str("\u{00A9}").unwrap();

    evfmt()
        .current_dir(tmp.path())
        .arg("--")
        .arg("check")
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

    evfmt()
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

    evfmt()
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

    evfmt()
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

    evfmt()
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

    evfmt()
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

    evfmt()
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

    evfmt()
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

    evfmt()
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

    evfmt()
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

    evfmt()
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

    evfmt()
        .arg("--set-bare-as-text=u(0041)")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "not eligible for emoji variation selectors",
        ));

    file.assert("\u{00A9}");
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

    evfmt().arg(tmp.path()).assert().success().code(0);

    a.assert("\u{00A9}\u{FE0F}");
    tmp.child("sub").child("b.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn evfmtignore_skips_matched_files() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child(".evfmtignore").write_str("skip.txt\n").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    evfmt().arg(tmp.path()).assert().success();

    tmp.child("skip.txt").assert("\u{00A9}");
    tmp.child("keep.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn remove_ignore_evfmt_overrides_evfmtignore() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child(".evfmtignore").write_str("skip.txt\n").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    evfmt()
        .arg("--remove-ignore=evfmt")
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

    evfmt()
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

    evfmt()
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

    evfmt()
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

    evfmt()
        .arg("--set-ignore=hdden")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("did you mean `hidden`?"));
}

#[test]
fn hidden_files_skipped() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child("visible.txt").write_str("\u{00A9}").unwrap();
    tmp.child(".hidden.txt").write_str("\u{00A9}").unwrap();

    evfmt().arg(tmp.path()).assert().success();

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

    evfmt().arg(solo.path()).arg(dir.path()).assert().success();

    solo.assert("\u{00A9}\u{FE0F}");
    dir.child("a.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn check_mode_with_directory() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child("a.txt").write_str("\u{00A9}").unwrap();

    evfmt().arg("--check").arg(tmp.path()).assert().code(1);

    // File should not be modified in check mode.
    tmp.child("a.txt").assert("\u{00A9}");
}

#[test]
fn empty_directory_succeeds() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dir = tmp.child("empty");
    std::fs::create_dir(dir.path()).unwrap();

    evfmt().arg(dir.path()).assert().success().code(0);
}
