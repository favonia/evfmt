# Rationale: Sequence Handling

Read when: auditing, challenging, or replacing the sequence-handling behavior described in [`../designs/features/sequence-handling.markdown`](../designs/features/sequence-handling.markdown).

Defines: non-normative evidence, assumptions, gaps, and tradeoff reasoning behind the current sequence-family policy boundaries.

Does not define: current formatter behavior. For behavior, see [`../designs/features/sequence-handling.markdown`](../designs/features/sequence-handling.markdown), code, tests, and public API documentation.

## Source Roles

This file separates different kinds of support so the rationale can be challenged without circular reasoning.

- Unicode / pinned data facts: facts from Unicode data files checked into the repository, Unicode terminology, or UTS #51 descriptions. These facts constrain the formatter, but they do not by themselves choose every product default.
- Project principles: the value system for tradeoffs. Correctness, determinism, usability, and maintainability explain why one viable behavior is preferred over another.
- Product assumptions: beliefs about what `evfmt` users need from canonical source text. These assumptions are weaker than Unicode facts and should remain visible when not independently supported.
- Tests: verification and drift guards. Tests can show that data-derived facts, invariants, and intended examples still hold; they do not prove that a semantic choice is the right product choice.
- Current implementation notes: context for how the behavior is achieved today. Implementation structure is not evidence that the behavior is semantically correct.
- Evidence gaps: places where the project does not currently have renderer studies, user research, or upstream prose strong enough to close the question.

## Manual Review Markers

Some arguments are explicitly marked for human review. Use one of these exact markers:

- `Manually reviewed: no.`
- `Manually reviewed: yes.`

Every inference subsection and every Q&A answer must have one explicit marker immediately after its subsection heading. The marker applies to the whole subsection until the next subsection heading. A human reviewer may change the marker to `Manually reviewed: yes.` after reading the marked argument. If an AI changes any marked argument text, the AI must set that argument to `Manually reviewed: no.`

Human reviewers may add temporary `Favonia:` or `Reviewer note:` lines inside a subsection to challenge, revise, or redirect an argument. These lines are editing scaffolding, not final public rationale. An AI must treat them as reviewer instructions or objections before changing the argument text: answer the note in chat, then either incorporate it, reject it with a reason, or leave it clearly unresolved. If the AI edits any part of the subsection, including the note itself, it must set the subsection marker to `Manually reviewed: no.` Resolved reviewer notes should be removed from the public document; unresolved notes may remain only when explicitly labeled `Reviewer note: unresolved - ...`. Before publication, this file should have no `Favonia:` lines.

Each subsection should own one claim or challenge. Prefer prose over a rigid template, but make the owned claim easy to spot by bolding the claim sentence or phrase itself. Evidence gaps should focus only on missing support for the claim made in that subsection. If a gap belongs to a different claim, move it to the subsection that owns that claim or make the alternative a separate Q&A entry. Avoid pointers between adjacent sections unless omitting the pointer would make the local gap misleading; when a pointer is needed, use plain prose rather than a cross-reference system.

## Inferences

### Only-Selector Edit Invariant

Manually reviewed: no.

Facts: `evfmt` formats `FE0E` and `FE0F`. The surrounding source text can include semantic characters, ZWJ links, combining marks, tags, regional indicators, and other emoji-related structure.

Principle-based inference: **formatting should insert, remove, or replace only `FE0E` and `FE0F`.** That boundary keeps the tool predictable on source files and limits the risk from permissive scanner recognition.

Evidence gap: this invariant is a product safety boundary. Unicode would permit tools with broader normalization goals, but that would be a different formatter.

### Policy Boundary For Ambiguous Contexts

Manually reviewed: no.

Facts: `FE0E` and `FE0F` request text or emoji presentation only in contexts that can interpret them. Some contexts have more than one sanctioned selector state after local structure is recognized; other contexts leave only removal, insertion, or replacement of a selector as the plausible canonical repair.

Principle-based inference: **policy belongs where a user-facing presentation choice remains.** Exposing policy for a deterministic repair would add configuration surface without creating a real choice, which weakens usability and maintainability.

Product goal: when only one reasonable selector state remains, `evfmt` should generate a broadly supported canonical spelling instead of asking users to opt into the only spelling the formatter can defend. That goal explains why modifiers, ZWJ-related component cleanup, and unsanctioned selectors are handled as fixed cleanup rather than as separate policy families.

Evidence gap: this is a product boundary, not a Unicode theorem. There is no user-study evidence here showing that this exact boundary is the one most users expect.

### Deterministic Fixed Cleanup

Manually reviewed: yes.

Facts: selector runs can be redundant, defective, or unsupported after the local Unicode-related structure is known. `evfmt` can rewrite those cases by changing only `FE0E` and `FE0F`.

Principle-based inference: **deterministic cleanup is favored when the formatter can identify a single canonical selector state.** It improves reproducibility and keeps formatting idempotent without asking users to choose between states that are not meaningful presentation preferences.

Product goal: fixed cleanup should produce selector spellings that are canonical for the recognized local structure and likely to be accepted by mainstream renderers. The goal is not to preserve every historical or byte-level spelling, nor to validate the entire emoji sequence as RGI.

The weak point is whether automatic repair is better than warning-only behavior for defects, legacy spellings, and compatibility-sensitive files. The current rationale for fixed cleanup is determinism, canonical output, and a narrow selector-only repair surface. It would be stronger with user evidence showing that warnings without repair create more churn or confusion than automatic selector cleanup. It would be overturned for some cases by compatibility evidence that the cleaned selector bytes are meaningful to important consumers.

### Keycap Policy Domain

Manually reviewed: yes.

Facts: keycap sequences use a base followed by `U+20E3` COMBINING ENCLOSING KEYCAP, optionally with a presentation selector between them. `U+20E3` predates Unicode emoji, and bare keycap spellings such as `[0-9#*] U+20E3` existed as ordinary Unicode text before modern emoji qualification. Older emoji mappings also used bare keycap spellings for carrier emoji compatibility. Modern emoji qualification uses `[0-9#*] FE0F U+20E3` for fully qualified emoji keycaps, while emoji data can still record bare keycap forms as unqualified emoji data.

Inference: **keycap-character positions need their own policy domain because their bare spelling has different practical meaning from ordinary standalone base positions.** Treating keycap positions as ordinary positions would hide the historical text spelling and modern emoji spelling behind a general policy set.

Evidence gap: the data and history support a distinct domain, but they do not prove that this is the simplest maintainable policy structure.

### Bare Keycap Default

Manually reviewed: no.

Facts: bare keycap inputs such as `# 20E3` are present in Unicode emoji data as unqualified forms, while fully qualified emoji keycaps include `FE0F`. The keycap bases `[0-9#*]` are text-default characters and have both text and emoji variation-sequence data in the pinned Unicode data.

Product assumption: **the default formatter behavior treats a bare keycap-character form as text-style source unless policy says otherwise.** Because the keycap bases are text-default characters, this follows their current default presentation instead of treating an unqualified emoji listing as a prediction of future Unicode direction. This favors preserving an observed text-default base spelling over silently promoting it to fully qualified emoji.

The weak point is user intent for contemporary bare keycap inputs. The supporting facts are that the bare form is historically valid Unicode text, the bases are text-default, and modern fully qualified emoji keycaps use `FE0F 20E3`. The counterargument is early emoji interchange practice: Unicode Emoji 1.0 recorded bare keycaps with carrier source data, and Unicode Emoji 2.0 listed bare keycaps directly as emoji sequences. That history weakens any claim that bare keycaps are naturally text-only. The missing evidence is renderer and user evidence about how often bare keycaps in contemporary source files mean emoji rather than text-style keycaps. Evidence from common tools showing that bare keycaps are overwhelmingly produced or perceived as emoji would weaken this default; evidence that users rely on text-styled enclosed keycaps would strengthen it.

### Text-Styled Keycap Preservation

Manually reviewed: yes.

Facts: `[0-9#*] FE0E` is a sanctioned text variation sequence for the base characters in the pinned data. In a keycap spelling, that selector appears before `U+20E3`; it is not an orphaned selector after the keycap mark. Older standardization discussions considered text-styled keycap spellings, even though current emoji qualification centers the emoji form on `FE0F 20E3`.

Inference: **preserving an explicit text selector in keycap context treats it as a local text-presentation request on the base before the enclosing keycap mark.** Converting it to `FE0F` would erase an explicit sanctioned selector state rather than merely repairing malformed text.

The weak point is whether the preserved text-styled keycap form is useful in practice. The support is selector sanctioning on the base and historical plausibility for text-style keycaps. The missing evidence is current renderer behavior and user intent: this rationale does not prove that `[0-9#*] FE0E U+20E3` renders consistently, usefully, or intentionally on supported platforms. Renderer evidence showing that the sequence collapses into broken or invisible output everywhere would weaken preservation; renderer and user examples showing stable text-style keycap use would strengthen it.

### Emoji-Default Bare Policy

Manually reviewed: no.

Facts: some standalone variation-sequence bases are emoji-default, so their bare spelling already carries emoji presentation by Unicode default. Text-default bases do not carry that same signal when bare.

Inference: **keeping emoji-default bases bare is different from guessing renderer behavior for text-default bases.** For emoji-default bases, a bare canonical spelling can match the Unicode default while avoiding redundant `FE0F`. For text-default bases where emoji presentation is intended, an explicit selector carries information that bare text does not.

Evidence gap: this explains the distinction, but it does not prove every default set is best for every document type.

### Modifier-Defect Cleanup

Manually reviewed: no.

Facts: UTS #51 describes the legacy spelling with `FE0F` immediately before an emoji modifier as defective and says the emoji presentation selector is ignored. The modifier still belongs to the surrounding emoji modifier sequence.

Inference: **removing that `FE0F` cleans ignored selector state.** It preserves the modifier sequence while preventing canonical output from retaining a known defective selector.

Important distinction: `base FE0F modifier` is the UTS #51 defective legacy form. `base FE0E modifier` is not the same category when `base FE0E` is a sanctioned text variation sequence. It is better described as a sanctioned text selector on the base followed by a modifier that may be presentation-incompatible or non-canonical for an emoji-normalizing formatter. The rationale should not label that `FE0E` form as defective merely because it appears before a modifier.

Evidence gap: the Unicode description supports removing ignored `FE0F`, but it does not require `evfmt` to choose formatting over warning-only diagnostics. The `FE0E` distinction also leaves a policy question: preserving a sanctioned text selector can be source-faithful while still producing output unlikely to form a normal emoji modifier sequence.

### ZWJ Component Locality

Manually reviewed: no.

Facts: ZWJ-related sequences are built from components joined by `U+200D`. A presentation selector can be local to a component, while a selector attached to a ZWJ link itself has no component base to own it.

Inference: **cleanup should preserve non-selector ZWJ structure and apply selector handling to each recognized component.** This keeps component-local presentation requests visible without rewriting the sequence topology.

The weak point is renderer behavior for component text selectors inside joined emoji. A component text request may prevent combined emoji rendering on some platforms, and UTS #51 sequence-breaking interpretations may not match `evfmt`'s source-preserving grouping model. The argument is source-level ownership and selector-only repair, not evidence that all preserved component selectors render well. Renderer evidence showing frequent destructive presentation results would force a revisit.

### Permissive Scanner Recognition

Manually reviewed: no.

Facts: malformed and non-RGI sequences can still contain selector-bearing components, ZWJ links, keycap marks, modifiers, tags, or unsupported selector runs. If scanning recognizes only RGI emoji, later cleanup may fail to see selector state that still needs a local decision.

Inference: **scanner recognition should be permissive enough to expose repairable selector structure to analysis.** This supports one-pass cleanup and protects the invariant that formatting does not need to rewrite non-selector text to find the same structure later.

The weak point is whether permissive recognition earns its complexity. The benefit is idempotent selector cleanup across malformed and partially recognized text. The cost is scanner complexity and the risk that ordinary text is grouped as emoji-like structure for analysis. The argument would be stronger with a compact set of examples proving each permissive grouping boundary is necessary. It would be weakened if the scanner becomes hard to reason about or if simpler RGI-strict scanning plus local fallback cleanup can preserve the same selector-only guarantees.

### Unsanctioned Selector Removal

Manually reviewed: yes.

Facts: `FE0E` and `FE0F` have meaning only when attached to a supported local selector-bearing context. Outside such a context, the selector has no sanctioned presentation choice to express.

Inference: **removing unsanctioned selector usage fits `evfmt`'s role as a presentation-selector formatter.** Preserving unsupported selector state would keep invisible control characters in canonical output without a well-defined presentation benefit.

The weak point is compatibility with consumers that attach meaning to unsupported selector bytes. The support is local selector sanctioning and the product goal of canonical source text. The missing evidence is compatibility research: some downstream consumers might attach out-of-band meaning to unsupported selectors or rely on byte stability. This argument would be strengthened by evidence that such consumers are rare in the project's target files; it would be overturned for a domain if unsupported selectors are known to carry meaningful compatibility state there.

## Skeptical Q&A

### Is fixed cleanup really not hidden policy?

Manually reviewed: no.

**Fixed cleanup is still a product choice.** The distinction is that fixed cleanup handles cases where the rationale sees no remaining meaningful presentation preference after local structure is known. The honest support is determinism and canonicalization; the gap is that warning-only behavior has not been ruled out by user evidence.

### Would warning-only modifier or cleanup behavior be safer?

Manually reviewed: no.

**Warning-only behavior would be more conservative about bytes, especially in compatibility-sensitive files.** The cost is that the formatter would diagnose known selector defects or unsupported selector state while leaving non-canonical output behind. That may be the right choice for a validator or linter mode, but it is weaker for a formatter whose output is expected to settle on a canonical selector spelling.

### Are old bare-keycap emoji inputs being misread as text?

Manually reviewed: no.

**Old bare-keycap emoji inputs may be misread as text.** Bare keycap spellings have both historical text legitimacy and historical emoji compatibility use. The default assumes canonical source text should not upgrade a text-default bare spelling to emoji without policy. That assumption should be revisited if contemporary evidence shows that bare keycap inputs in target files usually come from emoji-producing systems and users expect `FE0F 20E3`.

### Are text-styled keycaps worth preserving if renderers handle them poorly?

Manually reviewed: no.

**Text-styled keycaps may not be worth preserving if renderers handle them poorly.** Preservation is based on sanctioned selector ownership and historical plausibility, not on proof of useful rendering. If common renderers consistently show broken output, or if users report that text-styled keycap preservation surprises them more than it helps, converting or warning could become a better product choice.

### Does preserving `FE0E` on a ZWJ component contradict emoji rendering expectations?

Manually reviewed: no.

**Preserving `FE0E` on a ZWJ component can contradict emoji rendering expectations.** `evfmt` treats the selector as local source text attached to a component; renderers may treat that same selector as preventing a combined emoji image. This is a deliberate source-stability tradeoff, but renderer evidence could show that preservation creates more practical harm than it avoids.

### Does permissive scanning risk recognizing too much?

Manually reviewed: no.

**A permissive scanner can recognize too much.** It has maintainability cost and can make the analysis model harder to audit. The reason to accept that cost is to find selector state in malformed or non-RGI structures while keeping repairs selector-only and idempotent. If a simpler scanner can provide the same guarantees, the simpler design should win.

### Why not make the tool a strict Unicode sequence validator?

Manually reviewed: no.

**A strict validator would answer a different question: whether a sequence belongs to a chosen Unicode acceptance set.** `evfmt` instead formats presentation selectors in source text. That means it must sometimes repair local selector state in text that is not a fully valid or RGI emoji sequence, while avoiding broader normalization of non-selector characters.

### Why not make the tool RGI-only?

Manually reviewed: no.

**RGI-only handling would be simpler to explain, but it would miss repairable selector structure in malformed, legacy, or partially recognized text.** The open challenge is whether the extra permissiveness is worth its implementation and explanation cost.

### Why not make the tool lint-only and avoid repairs?

Manually reviewed: no.

**Lint-only behavior avoids automatic byte changes, but it gives up the main formatter benefit: repeatable canonical output.** A lint-only mode could be useful for compatibility-sensitive workflows; it would not replace the rationale for the formatter path unless automatic selector cleanup proves too surprising or destructive.

### Why not preserve every sanctioned selector exactly as written?

Manually reviewed: no.

**A sanctioned selector can still be redundant under the active policy.** Preserving all sanctioned selectors would make the tool closer to a validator than a formatter and would weaken canonicalization. The challenging case is when "redundant" selectors are kept intentionally for readability, compatibility with old renderers, or code review clarity.

### Why not always prefer explicit selectors?

Manually reviewed: no.

**Explicit selectors are not always better.** Some bases already have the desired default presentation, and always adding selectors would make canonical output noisier without improving determinism for those cases. This tradeoff should be revisited if explicitness proves more valuable than compact canonical spelling in real source files.

### Why not prefer bare whenever possible?

Manually reviewed: no.

**Bare text-default and emoji-default characters do not carry the same practical signal.** A universal bare preference would remove useful disambiguation for text-default characters where emoji presentation is intended. It would also make keycap defaults harder to explain because bare keycap spellings carry historical ambiguity.

### Are compatibility risks from unsanctioned selectors acceptable?

Manually reviewed: no.

**Compatibility risks from unsanctioned selectors are acceptable only under the product assumption that canonical source text is more important than preserving unsupported invisible state.** That assumption is weakest for files consumed by tools with out-of-band selector conventions or byte-level expectations. Evidence from such domains should narrow or revise cleanup.

### Is the maintainability cost of separate ordinary, keycap, modifier, and ZWJ handling justified?

Manually reviewed: no.

**The maintainability cost of separate ordinary, keycap, modifier, and ZWJ handling is justified only if those domains continue to represent genuinely different selector semantics.** If future code or documentation needs special cases that users cannot predict from local Unicode structure, the design should be simplified even if that means weaker cleanup.

### What would force a revisit?

Manually reviewed: no.

**This rationale should be revisited when its Unicode facts, product assumptions, compatibility assumptions, or maintainability assumptions change materially.** Revisit if pinned Unicode data changes keycap base properties, if UTS #51 changes modifier or keycap qualification descriptions, if renderer or user evidence undermines the bare-keycap or text-styled-keycap assumptions, if compatibility evidence shows unsanctioned selector removal is too destructive, or if implementation complexity shows that the current policy split is no longer maintainable.
