# Rationale: Formatting Model

Read when: auditing, challenging, or replacing the core formatting model described in [`../designs/core/formatting.markdown`](../designs/core/formatting.markdown).

Defines: non-normative evidence, assumptions, gaps, and tradeoff reasoning behind the formatter's cross-family selector-canonicalization model.

Does not define: current formatter behavior, recognition behavior, slot-classification behavior, or policy semantics. For behavior, see [`../designs/core/formatting.markdown`](../designs/core/formatting.markdown), feature design notes, code, tests, and public API documentation. For recognition, classification, and policy rationale, see [`recognition.markdown`](recognition.markdown), [`classification.markdown`](classification.markdown), and [`policy.markdown`](policy.markdown).

This file follows the rationale authoring and manual review conventions in [`authoring.markdown`](authoring.markdown).

## Inferences

### RGI-Compatible Default Formatting

Manually reviewed: yes.

Facts: RGI emoji data records exact emoji sequences recommended for general interchange. Some RGI spellings include `FE0F`; other RGI spellings are bare at selector-sensitive slots.

Principle-based inference: **default formatting should preserve exact RGI emoji input and prefer RGI spelling for emoji-presentation output.** RGI spellings are Unicode's recommended emoji spellings for general interchange, so exact RGI input must remain within the formatter's reasonable-state model and default formatting must not rewrite it to text presentation. Separately, when default policy chooses a canonical form with emoji presentation and RGI data answers that selector spelling choice, the chosen form should be the RGI spelling.

### Only-Selector Edit Invariant

Manually reviewed: yes.

Facts: `evfmt` formats `FE0E` and `FE0F`. The surrounding source text can include semantic characters, ZWJ links, combining marks, tags, regional indicators, and other emoji-related structure.

Principle-based inference: **formatting should insert, remove, or replace only `FE0E` and `FE0F`.** That boundary keeps the tool predictable on source files and limits the risk from permissive scanner recognition.

Evidence gap: this invariant is a product safety boundary. Unicode would permit tools with broader normalization goals, but that would be a different formatter.
