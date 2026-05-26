# Changelog

## 0.3.0 (unreleased)

Changes:

- Renamed the `findings` module to `analysis` and replaced the interactive analysis API's `Violation` categories, `DecisionSlot`s, and `ReplacementDecision`s with compositional `NonCanonicality` summaries and `Presentation`-based replacement choices.
- Renamed `Finding::default_replacement` to `Finding::default_canonical_replacement` and `Finding::replacement` to `Finding::canonical_replacement_with_decisions` to clarify that successful replacements are canonical under the selected decisions.
- Moved the shared `Presentation` type from the scanner API to the crate root.
- Preserved sanctioned text presentation before emoji modifiers while continuing to remove legacy defective emoji-presentation selectors before modifiers.

## 0.2.0 (2026-04-22)

Changes:

- Reworked the CLI around an explicit `evfmt format` subcommand, ordered `set/add/remove` policy operations, and metadata-preserving in-place formatting. ([#5], [#7], [#13])
- Rebuilt emoji analysis and formatting on Unicode 17.0 data, with more accurate handling for keycaps, modifiers, tags, flags, ZWJ-related structures, and presentation-selector runs. Keycap emoji formatting is now configurable, and emoji-default characters are kept bare by default. ([#11], [#14], [#16], [#17], [#18])
- Reshaped the library API around crate-root helpers, typed `evfmt::variation_set` constructors, iterator-based scanning, the exposed `Scanner` type, and `findings` APIs with per-slot replacement decisions. ([#3], [#9], [#11], [#18])
- Expanded examples and clarified stability and policy configuration guidance. ([#19])

## 0.1.0 (2026-04-09)

Initial release.

Features:

- Command-line formatter for normalizing text and emoji variation selectors.
- Recursive file formatting with `.gitignore` and `.evfmtignore` support.
- Check mode for CI and pre-commit use.
- Library API for scanning, classification and formatting.
- Policy controls via `--prefer-bare-for` and `--treat-bare-as-text-for`.

[#11]: https://github.com/favonia/evfmt/pull/11
[#13]: https://github.com/favonia/evfmt/pull/13
[#14]: https://github.com/favonia/evfmt/pull/14
[#16]: https://github.com/favonia/evfmt/pull/16
[#17]: https://github.com/favonia/evfmt/pull/17
[#18]: https://github.com/favonia/evfmt/pull/18
[#19]: https://github.com/favonia/evfmt/pull/19
[#3]: https://github.com/favonia/evfmt/pull/3
[#5]: https://github.com/favonia/evfmt/pull/5
[#7]: https://github.com/favonia/evfmt/pull/7
[#9]: https://github.com/favonia/evfmt/pull/9
