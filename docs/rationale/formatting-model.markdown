# Rationale: Formatting Model

Read when: auditing, challenging, or replacing the core formatting model described in [`../designs/core/formatting-model.markdown`](../designs/core/formatting-model.markdown).

Defines: non-normative evidence, assumptions, gaps, and tradeoff reasoning behind the formatter's core canonicalization model.

Does not define: current formatter behavior. For behavior, see [`../designs/core/formatting-model.markdown`](../designs/core/formatting-model.markdown), feature design notes, code, tests, and public API documentation.

This file follows the rationale authoring and manual review conventions in [`authoring.markdown`](authoring.markdown).

## Inferences

### Only-Selector Edit Invariant

Manually reviewed: yes.

Facts: `evfmt` formats `FE0E` and `FE0F`. The surrounding source text can include semantic characters, ZWJ links, combining marks, tags, regional indicators, and other emoji-related structure.

Principle-based inference: **formatting should insert, remove, or replace only `FE0E` and `FE0F`.** That boundary keeps the tool predictable on source files and limits the risk from permissive scanner recognition.

Evidence gap: this invariant is a product safety boundary. Unicode would permit tools with broader normalization goals, but that would be a different formatter.

### Policy Boundary For Ambiguous Contexts

Manually reviewed: no.

Facts: `FE0E` and `FE0F` request text or emoji presentation only in contexts that can interpret them. Some contexts have more than one sanctioned selector state after local structure is recognized; other contexts leave only removal, insertion, or replacement of a selector as the plausible canonical repair.

Principle-based inference: **policy belongs where a user-facing presentation choice remains.** Exposing policy for a deterministic repair would add configuration surface without creating a real choice, which weakens usability and maintainability.

Product goal: when only one reasonable selector state remains, `evfmt` should generate a broadly supported canonical spelling instead of asking users to opt into the only spelling the formatter can defend. That goal explains why fixed-cleanup sequence contexts and unsanctioned selectors are handled as fixed cleanup rather than as separate policy families.

Evidence gap: this is a product boundary, not a Unicode theorem. There is no user-study evidence here showing that this exact boundary is the one most users expect.

### Bare Display Assumption

Manually reviewed: no.

Product assumption: **for a policy position where bare spelling and an explicit selector spelling are both reasonable, bare rendering aligns with either the text-selector rendering or the emoji-selector rendering.** `evfmt` treats that as a display assumption only. It does not decide whether bare spelling is canonical, and it does not apply to fixed-cleanup cases that resolve before policy.

This assumption lets the two policy predicates describe a two-sided presentation choice: `prefer_bare` decides whether bare spelling may be kept, and `bare_as_text` decides which presentation side bare spelling is assumed to align with.

Evidence gap: renderers, fonts, editors, terminals, or platforms may display a bare policy position in a way that is meaningfully distinct from both explicit selector forms. Evidence of that behavior for important environments should trigger a revisit of the affected default policy membership or policy model.

### Small Policy Context Surface

Manually reviewed: no.

Facts: after local selector context classification and fixed cleanup, the current policy surface uses two domains indexed by variation-sequence base character: ordinary non-keycap positions and keycap-character positions.

Product assumption: **the public policy context surface should stay small, stable, named, and user-understandable.** "Small" does not mean a fixed count. It means policy domains should not be derived from arbitrary surrounding sequence topology.

Domain-qualified base-indexing is the current public realization of that constraint, not the fundamental principle. The current ordinary/keycap domains keep policy compact while preserving the keycap distinction that current sequence rationale treats as semantically meaningful.

Revisit this policy-shape choice if future Unicode data introduces selector contexts that do not fit the current domains, or if real user needs show that the current ordinary/keycap policy surface cannot express an important formatting choice. Possible outcomes include adding a small named domain, changing fixed-cleanup behavior, narrowing recognition, or choosing another small policy model; richer keys are not the automatic answer.

## Skeptical Q&A

### Why not always prefer explicit selectors?

Manually reviewed: yes.

**Explicit selectors are not always better.** Some bases already have the desired default presentation, and always adding selectors would make canonical output noisier without improving determinism for those cases. This tradeoff should be revisited if explicitness proves more valuable than compact canonical spelling in real source files.
