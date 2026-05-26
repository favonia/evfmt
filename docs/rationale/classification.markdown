# Rationale: Classification

Read when: auditing, challenging, or replacing the selector-slot classification behavior described in [`../designs/features/classification.markdown`](../designs/features/classification.markdown).

Defines: non-normative evidence, assumptions, gaps, and tradeoff reasoning behind reasonable-state assignment and fixed cleanup.

Does not define: current classification behavior, recognition behavior, or policy semantics. For behavior, see [`../designs/features/classification.markdown`](../designs/features/classification.markdown), code, tests, and public API documentation. For recognition and policy rationale, see [`recognition.markdown`](recognition.markdown) and [`policy.markdown`](policy.markdown).

This file follows the rationale authoring and manual review conventions in [`authoring.markdown`](authoring.markdown).

## Inferences

### Policy Boundary For Ambiguous Contexts

Manually reviewed: yes.

Facts: `FE0E` and `FE0F` request text or emoji presentation only in contexts that can interpret them. Some contexts have more than one sanctioned selector state after local structure is recognized; other contexts leave only removal, insertion, or replacement of a selector as the plausible canonical repair.

Principle-based inference: **policy belongs where a user-facing presentation choice remains.** Exposing policy for a deterministic repair would add configuration surface without creating a real choice, which weakens usability and maintainability.

Product goal: when only one reasonable selector state remains, `evfmt` should generate a broadly supported canonical spelling instead of asking users to opt into the only spelling the formatter can defend. That goal explains why fixed-cleanup sequence contexts and unsanctioned selectors are handled as fixed cleanup rather than as separate policy families.

Evidence gap: this is a product boundary, not a Unicode theorem. There is no user-study evidence here showing that this exact boundary is the one most users expect.

### Deterministic Fixed Cleanup

Manually reviewed: yes.

Facts: selector runs can be redundant, defective, or unsupported after the local Unicode-related structure is known. `evfmt` can rewrite those cases by changing only `FE0E` and `FE0F`.

Principle-based inference: **deterministic cleanup is favored when the formatter can identify a single canonical selector state.** It improves reproducibility and keeps formatting idempotent without asking users to choose between states that are not meaningful presentation preferences.

Product goal: fixed cleanup should produce selector spellings that are canonical for the recognized local structure and likely to be accepted by mainstream renderers. The goal is not to preserve every historical or byte-level spelling, nor to validate the entire emoji sequence as RGI.

The weak point is whether automatic repair is better than warning-only behavior for defects, legacy spellings, and compatibility-sensitive files. Determinism, canonical output, and a narrow selector-only repair surface support automatic cleanup. User evidence showing that warnings without repair create more churn or confusion than automatic selector cleanup would strengthen the argument; compatibility evidence that the cleaned selector bytes are meaningful to important consumers would overturn it for some cases.

### Context-Specific Rules

Manually reviewed: yes.

Inference: **modifier, tag, and ZWJ-related contexts justify separate classification rules because they make different selector states meaningful, defective, or irrelevant.** A flatter rule set would be easier to maintain, but it would either erase those distinctions or move hidden exceptions into policy, code, or tests.

### Modifier-Defect Cleanup

Manually reviewed: yes.

Facts: UTS #51 says emoji presentation selectors are not needed or recommended before emoji modifiers, should not be used in newly generated emoji modifier sequences, and are ignored in the defective legacy spelling where `FE0F` appears between a base and a modifier. The modifier still belongs to the surrounding emoji modifier sequence.

Inference: **removing that `FE0F` cleans ignored selector state.** It preserves the modifier sequence while preventing canonical output from retaining a known defective selector.

Important distinction: `base FE0F modifier` is the UTS #51 defective legacy form. `base FE0E modifier` is not the same category when `base FE0E` is a sanctioned text variation sequence. It is a sanctioned text variation sequence on the base followed by an emoji modifier; that combined text is not an emoji modifier sequence. The rationale should not label that `FE0E` form as defective merely because it appears before a modifier.

Evidence gap: UTS #51 strongly supports not generating the defective `FE0F` spelling, but it does not require `evfmt` to choose formatting over warning-only diagnostics. The `FE0E` distinction also leaves a design question: preserving a sanctioned text selector is source-faithful while still producing output that is not an emoji modifier sequence.

### Tag-Sequence Fixed Cleanup

Manually reviewed: yes.

Facts: the core Unicode Standard delegates emoji tag-sequence semantics to UTS #51, and the only valid use of tag characters is the use specified there. UTS #51 owns the semantics of the tag sequence, while ordinary text presentation is largely outside UTS #51.

Inference: **tag contexts should use fixed cleanup rather than policy because there is no independent text-presentation meaning for `evfmt` to preserve inside a tag sequence.** The formatter can still be permissive about recognizing tag-bearing structure, but once a tag context is recognized, selector cleanup should follow the UTS #51-owned emoji-tag semantics. That is the rationale for normalizing tag base presentation and for the stronger policy of dropping `FE0E` in tag context.

Evidence gap: this argues for fixed cleanup once recognition has already classified the surrounding text as a tag context. It does not justify the scanner's boundary for deciding which tag-bearing structures should be recognized in the first place.

### ZWJ Component Locality

Manually reviewed: yes.

Facts: ZWJ-related sequences are built from components joined by `U+200D`. A presentation selector can be local to a component, while a selector attached to a ZWJ link itself has no component base to own it. Malformed ZWJ-related text can still contain the same local selector slots even when the surrounding sequence is not a valid emoji sequence.

Inference: **cleanup should preserve non-selector ZWJ structure and apply selector handling to each recognized component.** This keeps component-local presentation requests visible without rewriting the sequence topology. For malformed ZWJ-related structures, the formatter's claim is only that it can make local `FE0E`/`FE0F` decisions while leaving the non-selector text unchanged.

Policy-surface rationale: **ZWJ component-local handling also avoids exposing policy over arbitrary ZWJ topology.** The same selector-bearing component can appear in many surrounding ZWJ shapes. Resolving each component through its local non-keycap or keycap context keeps ZWJ handling aligned with the small policy context surface described in [`policy.markdown`](policy.markdown).

#### Cross-layer check

**component-wise cleanup is reasonable because UTS #51 qualification is already component-sensitive.** A fully qualified emoji sequence is one where each emoji character in the sequence is qualified. If cleanup preserves each already qualified emoji character as qualified, then preserving the non-selector sequence structure is enough to preserve fully qualified emoji sequences.

The default policy satisfies that check. It does not turn an already qualified emoji character into text presentation: emoji-default bare characters stay bare, and non-emoji-default characters that need emoji presentation already have `FE0F` in a fully qualified input. Therefore default component-wise cleanup preserves fully qualified emoji sequences.

The strongest objection is that the same component-wise rule can degrade minimally qualified or unqualified emoji sequences into non-emoji text. That objection is real. If a sequence contains an unqualified bare text-default emoji character, default local policy may resolve that component to text presentation by inserting `FE0E`. The result can stop being an emoji sequence even though the input was a minimally qualified or unqualified emoji sequence.

The design accepts that cost because fully qualified emoji preservation is the stronger cross-layer invariant. Minimally qualified and unqualified emoji sequences already lack required emoji-presentation spelling somewhere, so the rationale treats local presentation normalization as allowed to win over preserving that weaker emoji-sequence status. If that tradeoff proves wrong, the remedy would be a sequence-aware exception for minimally qualified or unqualified emoji sequences, not denying that component-wise cleanup has this degradation mode.

### Unsanctioned Selector Removal

Manually reviewed: yes.

Facts: `FE0E` and `FE0F` have meaning only when attached to a supported local selector-bearing context. Outside such a context, the selector has no sanctioned presentation choice to express.

Inference: **removing unsanctioned selector usage fits `evfmt`'s role as a presentation-selector formatter.** Preserving unsupported selector state would keep invisible control characters in canonical output without a well-defined presentation benefit.

The weak point is compatibility with consumers that attach meaning to unsupported selector bytes. The support is local selector sanctioning and the product goal of canonical source text. The missing evidence is compatibility research: some downstream consumers might attach out-of-band meaning to unsupported selectors or rely on byte stability. This argument would be strengthened by evidence that such consumers are rare in the project's target files; it would be overturned for a domain if unsupported selectors are known to carry meaningful compatibility state there.

## Skeptical Q&A

### Should fixed cleanup be configurable?

Manually reviewed: yes.

**Not just because someone wants to keep the old bytes.** Fixed cleanup is for selector spellings that have no good presentation choice left after the local context is known. If evidence shows another spelling is meaningful, the right response is to explain that meaning and stop treating the case as fixed cleanup. Otherwise, a configuration flag would only preserve non-canonical bytes, not choose between real presentation states.
