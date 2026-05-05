# Rationale: Sequence Handling

Read when: auditing, challenging, or replacing the sequence-handling behavior described in [`../designs/features/sequence-handling.markdown`](../designs/features/sequence-handling.markdown).

Defines: non-normative evidence, assumptions, gaps, and tradeoff reasoning behind the current sequence-family policy boundaries.

Does not define: current formatter behavior. For behavior, see [`../designs/features/sequence-handling.markdown`](../designs/features/sequence-handling.markdown), code, tests, and public API documentation.

This file follows the rationale authoring and manual review conventions in [`authoring.markdown`](authoring.markdown).

## Inferences

### UTS #51 Semantic Ownership

Manually reviewed: yes.

Facts: UTS #51 defines emoji properties, emoji sequences, qualification, and RGI emoji data. It also owns some semantics delegated to it by the core Unicode Standard, such as emoji tag sequences. It does not regulate Unicode text presentation in general; for example, text-presentation variation sequences are not emoji sequences merely because they use characters that also participate in emoji data.

Principle-based inference: **UTS #51 should be applied where it owns the relevant emoji semantics, not as a general rule for all neighboring Unicode text.** In UTS #51-owned domains, an unqualified sequence is not fully valid as emoji. Outside those domains, especially for text presentation, the question is not whether the text is unqualified emoji; it is whether the text has non-emoji Unicode semantics that `evfmt` should preserve.

Evidence gap: this ownership boundary does not prove that every non-emoji text form is useful to preserve. It only prevents the formatter from treating UTS #51 emoji qualification as governing semantics that UTS #51 does not own.

### Deterministic Fixed Cleanup

Manually reviewed: yes.

Facts: selector runs can be redundant, defective, or unsupported after the local Unicode-related structure is known. `evfmt` can rewrite those cases by changing only `FE0E` and `FE0F`.

Principle-based inference: **deterministic cleanup is favored when the formatter can identify a single canonical selector state.** It improves reproducibility and keeps formatting idempotent without asking users to choose between states that are not meaningful presentation preferences.

Product goal: fixed cleanup should produce selector spellings that are canonical for the recognized local structure and likely to be accepted by mainstream renderers. The goal is not to preserve every historical or byte-level spelling, nor to validate the entire emoji sequence as RGI.

The weak point is whether automatic repair is better than warning-only behavior for defects, legacy spellings, and compatibility-sensitive files. The current rationale for fixed cleanup is determinism, canonical output, and a narrow selector-only repair surface. It would be stronger with user evidence showing that warnings without repair create more churn or confusion than automatic selector cleanup. It would be overturned for some cases by compatibility evidence that the cleaned selector bytes are meaningful to important consumers.

### Default RGI Byte Preservation

Manually reviewed: no.

Facts: RGI emoji data records exact emoji sequences recommended for general interchange. An input that is already exactly an RGI emoji sequence already has the selector spelling recommended for that emoji sequence.

Inference: **default formatting should preserve exact RGI emoji sequences byte-for-byte.** RGI preservation is the no-op case, not the reason for permissive scanner recognition.

Evidence gap: this default does not prove that every document wants RGI spellings preserved under every explicit policy. It only records the default formatter expectation.

### Keycap Policy Domain

Manually reviewed: yes.

Facts: keycap sequences use a base followed by `U+20E3` COMBINING ENCLOSING KEYCAP, optionally with a presentation selector between them. `U+20E3` predates Unicode emoji, and bare keycap spellings such as `[0-9#*] U+20E3` existed as ordinary Unicode text before modern emoji qualification. Older emoji mappings also used bare keycap spellings for carrier emoji compatibility. Modern emoji qualification uses `[0-9#*] FE0F U+20E3` for fully qualified emoji keycaps, while emoji data can still record bare keycap forms as unqualified emoji data.

Inference: **keycap-character positions need their own policy domain because their bare spelling has different practical meaning from ordinary standalone base positions.** Treating keycap positions as ordinary positions would hide the historical text spelling and modern emoji spelling behind a general policy set.

Evidence gap: the data and history support a distinct domain, but they do not prove that this is the simplest maintainable policy structure.

### Bare Keycap Default

Manually reviewed: yes.

Facts: bare keycap inputs such as `# 20E3` are present in Unicode emoji data as unqualified forms, while fully qualified emoji keycaps include `FE0F`. The keycap bases `[0-9#*]` are text-default characters and have both text and emoji variation-sequence data in the pinned Unicode data.

Product assumption: **the default formatter behavior treats a bare keycap-character form as text-style source unless policy says otherwise.** The keycap bases are text-default characters, so a bare spelling follows their current Unicode default. The fact that Unicode emoji data also lists the bare keycap form as unqualified emoji data describes its emoji qualification status; it does not by itself transfer the text-default bare spelling into emoji intent. This favors preserving an observed text-default base spelling over silently promoting it to fully qualified emoji.

The weak point is user intent for contemporary bare keycap inputs. The supporting facts are that the bare form is historically valid Unicode text, the bases are text-default, and modern fully qualified emoji keycaps use `FE0F 20E3`. The counterargument is early emoji interchange practice: Unicode Emoji 1.0 recorded bare keycaps with carrier source data, and Unicode Emoji 2.0 listed bare keycaps directly as emoji sequences. That history weakens any claim that bare keycaps are naturally text-only. The missing evidence is renderer and user evidence about how often bare keycaps in contemporary source files mean emoji rather than text-style keycaps. Evidence from common tools showing that bare keycaps are overwhelmingly produced or perceived as emoji would weaken this default; evidence that users rely on text-styled enclosed keycaps would strengthen it.

### Text-Styled Keycap Preservation

Manually reviewed: yes.

Facts: `[0-9#*] FE0E` is a sanctioned text variation sequence for the base characters in the pinned data. In a keycap spelling, that selector appears before `U+20E3`; it is not an orphaned selector after the keycap mark. Older standardization discussions considered text-styled keycap spellings, even though current emoji qualification centers the emoji form on `FE0F 20E3`.

Inference: **preserving an explicit text selector in keycap context treats it as a local text-presentation request on the base before the enclosing keycap mark.** Converting it to `FE0F` would erase an explicit sanctioned selector state rather than merely repairing malformed text.

The weak point is whether the preserved text-styled keycap form is useful in practice. The support is selector sanctioning on the base and historical plausibility for text-style keycaps. The missing evidence is current renderer behavior and user intent: this rationale does not prove that `[0-9#*] FE0E U+20E3` renders consistently, usefully, or intentionally on supported platforms. Renderer evidence showing that the sequence collapses into broken or invisible output everywhere would weaken preservation; renderer and user examples showing stable text-style keycap use would strengthen it.

### Emoji-Default Bare Policy

Manually reviewed: yes.

Facts: some standalone variation-sequence bases are emoji-default, so their bare spelling already carries emoji presentation by Unicode default.

Inference: **emoji-default bases should use their bare canonical spelling.** For emoji-default bases, a bare canonical spelling matches the Unicode default while avoiding redundant `FE0F`. It is also the RGI spelling for the sequence; adding redundant `FE0F` does not produce the RGI form. For future renderers, fonts, and platforms, honoring the Unicode default is the least-surprising long-term behavior.

Evidence gap: this supports bare canonical spelling for emoji-default bases, but it does not prove every document type prefers Unicode-default compactness over explicit selector spelling.

### Modifier-Defect Cleanup

Manually reviewed: yes.

Facts: UTS #51 says emoji presentation selectors are not needed or recommended before emoji modifiers, should not be used in newly generated emoji modifier sequences, and are ignored in the defective legacy spelling where `FE0F` appears between a base and a modifier. The modifier still belongs to the surrounding emoji modifier sequence.

Inference: **removing that `FE0F` cleans ignored selector state.** It preserves the modifier sequence while preventing canonical output from retaining a known defective selector.

Important distinction: `base FE0F modifier` is the UTS #51 defective legacy form. `base FE0E modifier` is not the same category when `base FE0E` is a sanctioned text variation sequence. It is a sanctioned text variation sequence on the base followed by an emoji modifier; that combined text is not an emoji modifier sequence. The rationale should not label that `FE0E` form as defective merely because it appears before a modifier.

Evidence gap: UTS #51 strongly supports not generating the defective `FE0F` spelling, but it does not require `evfmt` to choose formatting over warning-only diagnostics. The `FE0E` distinction also leaves a policy question: preserving a sanctioned text selector is source-faithful while still producing output that is not an emoji modifier sequence.

### Tag-Sequence Fixed Cleanup

Manually reviewed: no.

Facts: the core Unicode Standard delegates emoji tag-sequence semantics to UTS #51, and the only valid use of tag characters is the use specified there. UTS #51 owns the semantics of the tag sequence, while ordinary text presentation is largely outside UTS #51.

Inference: **tag contexts should use fixed cleanup rather than policy because there is no independent text-presentation meaning for `evfmt` to preserve inside a tag sequence.** The formatter can still be permissive about recognizing tag-bearing structure, but once a tag context is recognized, selector cleanup should follow the UTS #51-owned emoji-tag semantics. That is the rationale for normalizing tag base presentation and for the stronger policy of dropping `FE0E` in tag context.

Evidence gap: this supports selector cleanup after a tag context has been recognized; it does not prove the current scanner recognizes the best possible set of tag-bearing structures.

### ZWJ Component Locality

Manually reviewed: no.

Facts: ZWJ-related sequences are built from components joined by `U+200D`. A presentation selector can be local to a component, while a selector attached to a ZWJ link itself has no component base to own it. Malformed ZWJ-related text can still contain the same local selector positions even when the surrounding sequence is not a valid emoji sequence.

Inference: **cleanup should preserve non-selector ZWJ structure and apply selector handling to each recognized component.** This keeps component-local presentation requests visible without rewriting the sequence topology. For malformed ZWJ-related structures, the formatter's claim is only that it can make local `FE0E`/`FE0F` decisions while leaving the non-selector text unchanged.

Policy-surface rationale: **ZWJ component-local handling also avoids exposing policy over arbitrary ZWJ topology.** The same selector-bearing component can appear in many surrounding ZWJ shapes. Resolving each component through its local ordinary/keycap context keeps ZWJ handling aligned with the small policy context surface described in [`formatting-model.markdown`](formatting-model.markdown).

The weak point is renderer behavior for component text selectors inside joined emoji. A component text request may prevent combined emoji rendering on some platforms, and UTS #51 sequence-breaking interpretations may not match `evfmt`'s source-preserving grouping model. The argument is source-level ownership and selector-only repair, not evidence that all preserved component selectors render well. Renderer evidence showing frequent destructive presentation results would force a revisit.

### Presentation Selector Coverage

Manually reviewed: no.

Facts: `FE0E` and `FE0F` can appear in sanctioned variation sequences, inside emoji-related structures, or as orphaned, repeated, or unsupported selector usage. A formatter pass that fails to surface a selector to analysis cannot make a local keep, rewrite, or remove decision for that selector.

Inference: **the scanner contract needs every `FE0E` and `FE0F` exposed to selector analysis.** This is a coverage rationale for the design-owned scanner contract, not a separate structural rule: it explains why all presentation selectors must be seen, but it does not decide which larger sequence structure should represent the surrounding text.

Evidence gap: this supports idempotent selector formatting, but it does not prove that every exposed selector should be changed automatically rather than warned about.

### Permissive Scanner Recognition

Manually reviewed: no.

UTS #51 defines valid emoji sequence structures, and RGI emoji data is only a smaller recommended-for-interchange subset of those structures. Presentation selectors are part of that structure in some cases: some valid emoji sequences require `FE0F`, while others allow or omit selectors. This rationale calls the non-`FE0E`/`FE0F` structure shared with a valid emoji sequence a valid emoji skeleton.

The design-owned scanner contract uses valid emoji skeletons rather than exact RGI membership as its structural boundary. Recognizing text that matches a valid emoji skeleton, even when the `FE0E`/`FE0F` spelling is different, redundant, missing, or misplaced, gives later analysis the context it needs to decide whether each selector is owned, redundant, defective, or unsupported.

This is a structural-recognition claim, not the selector-coverage claim. Selector coverage says every `FE0E` and `FE0F` must be surfaced somewhere for analysis. Permissive scanner recognition says which larger emoji-related structures are worth preserving around those selectors.

Emoji-related extended grapheme clusters are useful guidelines for local selector analysis. Keeping those clusters together is useful because a selector inside one is normally part of the same local context as the surrounding modifier, ZWJ, keycap, regional-indicator flag pair, variation-selector, or similar structure. This is a locality argument, not a UAX #29 conformance claim. The current scanner may still cut through an emoji-related extended grapheme cluster when the cluster includes combining marks or other material outside the scanner's recognized emoji-related vocabulary.

The weak point is whether this structural permissiveness earns its complexity. It makes scanner behavior harder to audit, can group ordinary-looking text as emoji-related structure, and can create unbounded scan items. The buffering risk is not limited to ZWJ-related sequences: ZWJ-related items, tag runs, and other repeated selector-bearing structures can all grow without a fixed bound. Linear scanning prevents backtracking and repeated work, but it does not by itself guarantee constant-size buffering. The argument would weaken if a simpler scanner could expose every selector and preserve the same one-pass, selector-only idempotence without representing these valid emoji skeletons.

### Unsanctioned Selector Removal

Manually reviewed: yes.

Facts: `FE0E` and `FE0F` have meaning only when attached to a supported local selector-bearing context. Outside such a context, the selector has no sanctioned presentation choice to express.

Inference: **removing unsanctioned selector usage fits `evfmt`'s role as a presentation-selector formatter.** Preserving unsupported selector state would keep invisible control characters in canonical output without a well-defined presentation benefit.

The weak point is compatibility with consumers that attach meaning to unsupported selector bytes. The support is local selector sanctioning and the product goal of canonical source text. The missing evidence is compatibility research: some downstream consumers might attach out-of-band meaning to unsupported selectors or rely on byte stability. This argument would be strengthened by evidence that such consumers are rare in the project's target files; it would be overturned for a domain if unsupported selectors are known to carry meaningful compatibility state there.

## Skeptical Q&A

### Should fixed cleanup be configurable?

Manually reviewed: no.

**Fixed cleanup should become configurable only when a real use case justifies the configuration cost.** These rewrites are product choices, but the current rationale treats them as fixed formatter rules because no clean user-facing policy model has been identified for the affected selector states. Evidence that users need to preserve or choose among these states should first be weighed against the complexity of exposing that choice.

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

**A permissive scanner can recognize too much.** It has maintainability cost and can make the analysis model harder to audit. The reason to accept that cost is to represent valid emoji sequence skeletons even when their `FE0E`/`FE0F` spelling differs from the valid sequence spelling or from exact RGI data, while keeping repairs selector-only and idempotent. If a simpler scanner can provide the same guarantees, the simpler design should win.

### Why not prefer bare whenever possible?

Manually reviewed: no.

**Bare text-default and emoji-default characters do not carry the same practical signal.** A universal bare preference would remove useful disambiguation for text-default characters where emoji presentation is intended. It would also make keycap defaults harder to explain because bare keycap spellings carry historical ambiguity.

### Is the maintainability cost of separate ordinary, keycap, modifier, and ZWJ handling justified?

Manually reviewed: no.

**The maintainability cost of separate ordinary, keycap, modifier, and ZWJ handling is justified only if those domains continue to represent genuinely different selector semantics.** If future code or documentation needs special cases that users cannot predict from local Unicode structure, the design should be simplified even if that means weaker cleanup.
