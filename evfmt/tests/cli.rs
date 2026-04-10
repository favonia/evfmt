// tests/cli.rs — CLI integration tests.
//
// These tests exercise the evfmt binary end-to-end using assert_cmd
// and assert_fs. They cover the full public CLI contract:
//   - Format mode (in-place rewrite), check mode (exit 1 if changes needed)
//   - stdin/stdout via `-`, --help-expression
//   - Error cases: multiple dashes, invalid UTF-8, no files, partial failure
//   - Directory traversal, .evfmtignore, --no-ignore, --ignore, hidden files
//   - Help text / design doc consistency

// Tests use unwrap for concise assertions — a panic IS the failure signal.
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::PredicateBooleanExt;

use evfmt::expr;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

fn evfmt() -> Command {
    Command::cargo_bin("evfmt").unwrap()
}

fn read_expression_doc() -> String {
    let doc_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/designs/features/expression-language.markdown");
    std::fs::read_to_string(doc_path).unwrap()
}

// --- Format mode ---

#[test]
fn format_rewrites_file() {
    let tmp = assert_fs::TempDir::new().unwrap();
    // ©️ is eligible, text-default in Unicode, not bare-preferred → gets FE0F with default policy
    // (treat-bare-as-text-for='ascii' does not match non-ASCII ©️, so bare resolves to emoji).
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

// --- Help expression ---

#[test]
fn help_expression_prints_reference() {
    evfmt()
        .arg("--help-expression")
        .assert()
        .success()
        .stdout(predicates::str::contains("Expression Language"));
}

#[test]
fn help_expression_works_without_files() {
    // --help-expression should not require file arguments.
    evfmt().arg("--help-expression").assert().success();
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

#[test]
fn invalid_policy_expression_exits_2() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file = tmp.child("test.txt");
    file.write_str("\u{00A9}").unwrap();

    evfmt()
        .arg("--prefer-bare-for")
        .arg("ascii_suffix")
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("--prefer-bare-for"));

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
fn no_ignore_overrides_ignore_files() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child(".evfmtignore").write_str("skip.txt\n").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    evfmt()
        .arg("--no-ignore")
        .arg(tmp.path())
        .assert()
        .success();

    tmp.child("skip.txt").assert("\u{00A9}\u{FE0F}");
    tmp.child("keep.txt").assert("\u{00A9}\u{FE0F}");
}

#[test]
fn ignore_overrides_no_ignore() {
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child(".evfmtignore").write_str("skip.txt\n").unwrap();
    tmp.child("skip.txt").write_str("\u{00A9}").unwrap();
    tmp.child("keep.txt").write_str("\u{00A9}").unwrap();

    // --ignore after --no-ignore: last flag wins, ignore files are respected.
    evfmt()
        .arg("--no-ignore")
        .arg("--ignore")
        .arg(tmp.path())
        .assert()
        .success();

    tmp.child("skip.txt").assert("\u{00A9}");
    tmp.child("keep.txt").assert("\u{00A9}\u{FE0F}");
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

// --- Help text / design doc consistency ---

#[test]
fn help_text_and_design_doc_agree_on_named_sets() {
    let help = expr::EXPRESSION_HELP;
    let doc = read_expression_doc();

    // Every named set display name must appear in both.
    let named_sets = [
        "ascii",
        "emoji-defaults",
        "rights-marks",
        "arrows",
        "card-suits",
    ];
    for name in &named_sets {
        assert!(
            help.contains(name),
            "EXPRESSION_HELP missing named set {name:?}"
        );
        assert!(
            doc.contains(name),
            "expression-language.markdown missing named set {name:?}"
        );
    }
}

#[test]
fn help_text_and_design_doc_agree_on_combinators() {
    let help = expr::EXPRESSION_HELP;
    let doc = read_expression_doc();

    for keyword in ["union(", "subtract(", "except("] {
        assert!(
            help.contains(keyword),
            "EXPRESSION_HELP missing combinator {keyword:?}"
        );
        assert!(
            doc.contains(keyword),
            "expression-language.markdown missing combinator {keyword:?}"
        );
    }
}
