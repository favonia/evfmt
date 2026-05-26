# Rationale: Recognition

Read when: auditing, challenging, or replacing the scanner recognition boundaries described in [`../designs/features/recognition.markdown`](../designs/features/recognition.markdown).

Defines: non-normative evidence, assumptions, gaps, and tradeoff reasoning behind scanner structure, selector visibility, valid emoji skeleton recognition, and related recognition boundaries.

Does not define: current scanner behavior or slot-classification behavior. For behavior, see [`../designs/features/recognition.markdown`](../designs/features/recognition.markdown), code, tests, and public API documentation. For classification and policy rationale, see [`classification.markdown`](classification.markdown) and [`policy.markdown`](policy.markdown).

This file follows the rationale authoring and manual review conventions in [`authoring.markdown`](authoring.markdown).

## Inferences

### UTS #51 Semantic Ownership

Manually reviewed: yes.

Facts: UTS #51 defines emoji properties, emoji sequences, qualification, and RGI emoji data. It also owns some semantics delegated to it by the core Unicode Standard, such as emoji tag sequences. It does not regulate Unicode text presentation in general; for example, text-presentation variation sequences are not emoji sequences merely because they use characters that also participate in emoji data.

Principle-based inference: **UTS #51 should be applied where it owns the relevant emoji semantics, not as a general rule for all neighboring Unicode text.** In UTS #51-owned domains, an unqualified sequence is not fully valid as emoji. Outside those domains, especially for text presentation, the question is not whether the text is unqualified emoji; it is whether the text has non-emoji Unicode semantics that `evfmt` should preserve.

Evidence gap: this ownership boundary does not prove that every non-emoji text form is useful to preserve. It only prevents the formatter from treating UTS #51 emoji qualification as governing semantics that UTS #51 does not own.

### Presentation Selector Coverage

Manually reviewed: yes.

Facts: `FE0E` and `FE0F` can appear in sanctioned variation sequences, inside emoji-related structures, or as orphaned, repeated, or unsupported selector usage. A formatter pass that fails to surface a selector to analysis cannot make a local keep, rewrite, or remove decision for that selector.

Inference: **every `FE0E` and `FE0F` must be seen before the formatter can be trusted to leave no selector work behind.** A hidden selector can survive unchanged, be cleaned only on a later pass, or make neighboring selector decisions depend on scanner accident. Full selector visibility is what lets selector cleanup be local and idempotent.

### Permissive Scanner Recognition

Manually reviewed: yes.

UTS #51 defines valid emoji sequence structures, and RGI emoji data is only a smaller recommended-for-interchange subset of those structures. Presentation selectors are part of that structure in some cases: some valid emoji sequences require `FE0F`, while others allow or omit selectors. This rationale calls the non-`FE0E`/`FE0F` structure shared with a valid emoji sequence a valid emoji skeleton.

A scanner that uses valid emoji skeletons rather than exact RGI membership can recognize useful local structure even when the `FE0E`/`FE0F` spelling is different, redundant, missing, or misplaced. That gives later analysis the context it needs to decide whether each selector is owned, redundant, defective, or unsupported.

This is a structural-recognition claim, not the selector-coverage claim. Selector coverage says every `FE0E` and `FE0F` must be surfaced somewhere for analysis. Permissive scanner recognition says which larger emoji-related structures are worth preserving around those selectors.

Emoji-related extended grapheme clusters are useful guidelines for local selector analysis. Keeping those clusters together is useful because a selector inside one is normally part of the same local context as the surrounding modifier, ZWJ, keycap, regional-indicator flag pair, variation-selector, or similar structure. This is a locality argument, not a UAX #29 conformance claim. A scanner may still cut through an emoji-related extended grapheme cluster when the cluster includes combining marks or other material outside its recognized emoji-related vocabulary.

The weak point is whether this structural permissiveness earns its complexity. It makes scanner behavior harder to audit, can group ordinary-looking text as emoji-related structure, and can create unbounded scan items. The buffering risk is not limited to ZWJ-related sequences: ZWJ-related items, tag runs, and other repeated selector-bearing structures can all grow without a fixed bound. Linear scanning prevents backtracking and repeated work, but it does not by itself guarantee constant-size buffering. The argument would weaken if a simpler scanner could expose every selector and preserve the same one-pass, selector-only idempotence without representing these valid emoji skeletons.
