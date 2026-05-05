# Rationale: Documentation Source Stability

Read when: auditing, challenging, or replacing the checked-in text rules described in [`../designs/guides/source-text-stability.markdown`](../designs/guides/source-text-stability.markdown).

Defines: non-normative evidence gaps, assumptions, and tradeoff reasoning behind the repository text source-stability rules.

Does not define: current documentation, comment, diagnostic, or formatter behavior. For behavior, see [`../designs/guides/source-text-stability.markdown`](../designs/guides/source-text-stability.markdown), local documentation, code, tests, and public API documentation.

This file follows the rationale authoring and manual review conventions in [`authoring.markdown`](authoring.markdown).

## Inferences

### Checked-In Source Byte Stability

Manually reviewed: no.

Facts: repository text can contain Unicode characters whose intended presentation may depend on `FE0E`, `FE0F`, renderer defaults, fonts, editors, terminals, or platform behavior. `evfmt` exists to make presentation-selector spelling stable under a pinned formatter policy.

Principle-based inference: **checked-in text source stability is a necessary repository writing constraint.** In documentation, comments, test strings, and operator messages, stable selector spelling or code-point notation preserves intended meaning better than accidental byte choices for dual-presentation characters.

Evidence gap: this rationale assumes source stability is more valuable than minimizing invisible selectors in repository prose. That assumption should be revisited if maintainers find the explicit spellings harder to audit than the formatter churn they prevent.

### Reader Meaning And Source Spelling Are Separate

Manually reviewed: no.

Facts: the clearest rendered explanation for a human reader does not always match the most stable literal source spelling. A document may need to discuss a bare code point such as `U+00A9` while the checked-in source uses an explicit selector on a rendered glyph, or avoids the glyph with code-point notation.

Principle-based inference: **reader meaning and source spelling are separate layers.** Rendered prose should make the user's task clear; raw source spelling is a maintenance mechanism, not necessarily the concept being explained.

Evidence gap: separating reader meaning from source spelling can confuse future editors who inspect only the raw file. The current mitigation is to name code points explicitly when the distinction matters and to use escapes only when they improve stability or auditability.

## Skeptical Q&A

### Why not escape every dual-presentation character?

Manually reviewed: no.

**Maximum escaping would trade one kind of stability problem for another.** Escapes are useful when a raw glyph is fragile, unclear, or difficult to audit, but escaping every possible character would make prose and examples harder to read and maintain. The current rule prefers natural prose first, then stable source spelling.

### Could hidden selectors or source/rendered mismatch confuse maintainers?

Manually reviewed: no.

**Yes, hidden selectors and source/rendered mismatch are real maintenance costs.** The rule accepts that cost only when it protects checked-in text from formatter churn or ambiguous rendering. When the exact code point matters, prose should name it explicitly so future editors do not have to infer meaning from glyph shape or invisible selector bytes.
