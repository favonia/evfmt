# Changelog

## 0.3.0 (2026-08-03)

Breaking changes:

- Changed the default policy so bare text-default characters in non-keycap selector slots normalize to explicit text presentation instead of explicit emoji presentation. ([#29](https://github.com/favonia/evfmt/pull/29), [#32](https://github.com/favonia/evfmt/pull/32))
- Renamed `VariationSet` and `evfmt::variation_set` to `PolicyKeySet` and `evfmt::policy_key_set` to make the policy-key domain explicit. ([#28](https://github.com/favonia/evfmt/pull/28), [#31](https://github.com/favonia/evfmt/pull/31))
- Reworked the policy-set namespace: `keycap-chars` is now `keycap:variation-bases`, `non-keycap-chars` is now `variation-bases`, `keycap-emojis` is now `keycap:rgi`, `keycap:text-defaults` and `keycap:emoji-defaults` were added, and the convenience subsets `rights-marks`, `arrows`, and `card-suits` were removed. ([#34](https://github.com/favonia/evfmt/pull/34), [#35](https://github.com/favonia/evfmt/pull/35))
- Renamed the `findings` module to `analysis` and replaced the analysis API's `Violation` categories, `DecisionSlot`s, and `ReplacementDecision`s with compositional `NonCanonicality` summaries and `Presentation`-based replacement choices. ([#26](https://github.com/favonia/evfmt/pull/26), [#36](https://github.com/favonia/evfmt/pull/36), [#42](https://github.com/favonia/evfmt/pull/42))
- Reshaped `Finding` for the new analysis model: `Finding::violation` became `Finding::non_canonicality`, `Finding::decision_slots` was removed, `Finding::default_decision` became the `Presentation` iterator `Finding::default_decisions`, `Finding::default_replacement` became `Finding::default_canonical_replacement` and now returns `String`, and `Finding::replacement` became `Finding::canonical_replacement_with_decisions`. ([#26](https://github.com/favonia/evfmt/pull/26))
- Moved the shared `Presentation` type from the scanner API to the crate root. ([#26](https://github.com/favonia/evfmt/pull/26))

Fixed:

- Preserved sanctioned text presentation before emoji modifiers while continuing to remove legacy defective emoji-presentation selectors before modifiers. ([#26](https://github.com/favonia/evfmt/pull/26))

## 0.2.0 (2026-04-22)

Changes:

- Reworked the CLI around an explicit `evfmt format` subcommand, ordered `set/add/remove` policy operations, and metadata-preserving in-place formatting. ([#5](https://github.com/favonia/evfmt/pull/5), [#7](https://github.com/favonia/evfmt/pull/7), [#13](https://github.com/favonia/evfmt/pull/13))
- Rebuilt emoji analysis and formatting on Unicode 17.0 data, with more accurate handling for keycaps, modifiers, tags, flags, ZWJ-related structures, and presentation-selector runs. Keycap emoji formatting is now configurable, and emoji-default characters are kept bare by default. ([#11](https://github.com/favonia/evfmt/pull/11), [#14](https://github.com/favonia/evfmt/pull/14), [#16](https://github.com/favonia/evfmt/pull/16), [#17](https://github.com/favonia/evfmt/pull/17), [#18](https://github.com/favonia/evfmt/pull/18))
- Reshaped the library API around crate-root helpers, typed `evfmt::variation_set` constructors, iterator-based scanning, the exposed `Scanner` type, and `findings` APIs with per-slot replacement decisions. ([#3](https://github.com/favonia/evfmt/pull/3), [#9](https://github.com/favonia/evfmt/pull/9), [#11](https://github.com/favonia/evfmt/pull/11), [#18](https://github.com/favonia/evfmt/pull/18))
- Expanded examples and clarified stability and policy configuration guidance. ([#19](https://github.com/favonia/evfmt/pull/19))

## 0.1.0 (2026-04-09)

Initial release.

Features:

- Command-line formatter for normalizing text and emoji variation selectors.
- Recursive file formatting with `.gitignore` and `.evfmtignore` support.
- Check mode for CI and pre-commit use.
- Library API for scanning, classification and formatting.
- Policy controls via `--prefer-bare-for` and `--treat-bare-as-text-for`.
