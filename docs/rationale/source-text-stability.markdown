# Rationale: Documentation Source Stability

Read when: auditing, challenging, or replacing the checked-in text rules described in [`../design/guides/source-text-stability.markdown`](../design/guides/source-text-stability.markdown).

Defines: non-normative evidence gaps, assumptions, and tradeoff reasoning behind the repository text source-stability rules.

Does not define: current documentation, comment, diagnostic, or formatter behavior. For behavior, see [`../design/guides/source-text-stability.markdown`](../design/guides/source-text-stability.markdown), local documentation, code, tests, and public API documentation.

This file follows the rationale authoring and manual review conventions in [`authoring.markdown`](authoring.markdown).

## Inferences

### Checked-In Source Byte Stability

Manually reviewed: yes.

Facts: repository text can contain Unicode characters whose intended presentation may depend on `FE0E`, `FE0F`, renderer defaults, fonts, editors, terminals, or platform behavior. This repository includes the same kinds of checked-in prose that `evfmt` is meant to keep stable: documentation, comments, tests, and operator messages.

Principle-based inference: **checked-in source byte stability is a product viability test for `evfmt`.** The formatter should be usable on its own repository without forcing maintainers to avoid real checked-in prose, accept unstable selector churn, or hide the formatter from the places where its policy matters. Self-application shows whether the policy is practical for ordinary repository text, not just for isolated examples.

Evidence gap: this rationale would be stronger with recorded cases where applying `evfmt` to this repository changed the tool, the policy, or the source-stability guidance. Without that feedback loop, self-application can still be a reasonable product principle, but the repository would not yet show that dogfooding is improving `evfmt` rather than merely demonstrating it.

### Reader Meaning And Source Spelling Are Separate

Manually reviewed: yes.

Facts: the clearest rendered explanation for a human reader does not always match the most stable literal source spelling. A document may need to discuss a bare code point such as `U+00A9` while the checked-in source uses an explicit selector on a rendered glyph, or avoids the glyph with code-point notation.

Principle-based inference: **reader meaning and source spelling are separate layers.** Rendered prose should make the user's task clear; raw source spelling is a maintenance mechanism, not necessarily the concept being explained.

Evidence gap: this rationale would be stronger with examples showing that future editors can maintain this split from raw source without misreading the intended character identity.

## Skeptical Q&A

### Why not escape every dual-presentation character?

Manually reviewed: yes.

**Maximum escaping would trade one kind of stability problem for another.** Escapes are useful when a raw glyph is fragile, unclear, or difficult to audit, but escaping every possible character would make prose and examples harder to read and maintain. Natural prose should stay natural unless raw glyphs make the source unstable or hard to audit.
