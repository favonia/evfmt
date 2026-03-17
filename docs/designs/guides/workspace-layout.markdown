# Design Note: Workspace Layout

Read when: changing crate boundaries, module placement, build-time Unicode data generation, or workspace layout.

Defines: the crate layout, responsibility split, and build-time Unicode data pipeline.

## Crate structure

The project is a Cargo workspace with one crate:

- `evfmt`: the rule engine and non-interactive formatter CLI

The library owns all sequence classification, rule-engine behavior, policy evaluation, and text rewriting. The binary (`main.rs`) is only the batch-oriented CLI surface over that library — it contains no formatting logic. The top-level entry point for formatting is `formatter::format_text()`.

The command family uses aligned names: `evfmt [options] FILES...` and `evfmt check [options] FILES...`.

## Unicode data pipeline

`evfmt/build.rs` parses pinned Unicode data files from `evfmt/data/` at compile time and generates `unicode_data.rs`:

- `emoji-variation-sequences.txt` → which characters have text and emoji variation sequences
- `emoji-data.txt` → which characters have the `Emoji_Presentation` property

The generated file contains a sorted `VARIATION_ENTRIES` array. Runtime lookup uses binary search. Broader sequence-aware validation builds on top of this data layer rather than replacing it.

The repository's pinned Unicode inputs are broader than the compile-time tables alone. Upgrade and verification work may also depend on:

- `emoji-sequences.txt` for keycap and other sequence-family structure
- `emoji-zwj-sequences.txt` for ZWJ-family structure
- `emoji-test.txt` for qualification and family cross-checks

These files are part of the pinned local evidence set even when they do not all feed directly into `build.rs`.

Upgrade work may additionally fetch upstream prose documents whose relevant passages are tracked as review anchors. Those upstream documents are for monitored assumptions and human-reviewed diffs; routine correctness checks should continue to trust the pinned local copies in `evfmt/data/` first.
