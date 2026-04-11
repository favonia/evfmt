# Changelog

## Unreleased

Changes:

- Reorganized the library API so high-level helpers live at the crate root.
- Reduced visibility of internal scanner and slot helpers.
- Tightened and clarified module documentation.
- Preserve file permissions and metadata during in-place formatting.
- Replaced the old one-shot CLI policy flags with ordered `set/add/remove` operations for policy and ignore filters.
- Replaced the library's old string expression parser with typed `evfmt::charset` smart constructors.

## 0.1.0 (2026-04-09)

Initial release.

Features:

- Command-line formatter for normalizing text and emoji variation selectors.
- Recursive file formatting with `.gitignore` and `.evfmtignore` support.
- Check mode for CI and pre-commit use.
- Library API for scanning, classification and formatting.
- Policy controls via `--prefer-bare-for` and `--treat-bare-as-text-for`.
