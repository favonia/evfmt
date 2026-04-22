# Changelog

## Unreleased

Changes:

- Reorganized the library API so high-level helpers live at the crate root.
- Reduced visibility of internal scanner and slot helpers.
- Tightened and clarified module documentation.
- Preserve file permissions and metadata during in-place formatting.
- Changed in-place formatting to use the explicit `evfmt format` subcommand.
- Replaced the old one-shot CLI policy flags with ordered `set/add/remove` operations for policy and ignore filters.
- Made standalone keycap formatting configurable with `keycap-emojis`.
- Replaced the library's old string expression parser with typed `evfmt::variation_set` smart constructors.
- Replaced the public `review` API with `findings` analysis APIs for scanned items.
- Changed `scan` to return an iterator and exposed the `Scanner` type.
- Reworked scanner recognition around emoji-like state-machine structure, including malformed ZWJ-related structures and unsanctioned presentation-selector runs.
- Updated Unicode-derived classification to use emoji property ranges, regional indicators, and `Emoji_Presentation` data.
- Refined fixed cleanup for keycap, modifier, tag, flag, and ZWJ-related structures while keeping formatting limited to presentation-selector changes.

## 0.1.0 (2026-04-09)

Initial release.

Features:

- Command-line formatter for normalizing text and emoji variation selectors.
- Recursive file formatting with `.gitignore` and `.evfmtignore` support.
- Check mode for CI and pre-commit use.
- Library API for scanning, classification and formatting.
- Policy controls via `--prefer-bare-for` and `--treat-bare-as-text-for`.
