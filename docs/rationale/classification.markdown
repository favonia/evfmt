# Rationale: Classification

Read when: auditing, challenging, or replacing the selector-slot classification behavior described in [`../designs/features/classification.markdown`](../designs/features/classification.markdown).

Defines: non-normative evidence, assumptions, gaps, and tradeoff reasoning behind reasonable-state assignment, context-specific selector accounting, and fixed cleanup.

Does not define: current classification behavior, recognition behavior, or policy semantics. For behavior, see [`../designs/features/classification.markdown`](../designs/features/classification.markdown), code, tests, and public API documentation. For recognition and policy rationale, see [`recognition.markdown`](recognition.markdown) and [`policy.markdown`](policy.markdown).

This file follows the rationale authoring and manual review conventions in [`authoring.markdown`](authoring.markdown).

## Inferences

### Policy Boundary For Ambiguous Contexts

Manually reviewed: yes.

Facts: `FE0E` and `FE0F` request text or emoji presentation only in contexts that can interpret them. Some contexts have more than one sanctioned selector state after local structure is recognized; other contexts leave only removal, insertion, or replacement of a selector as the plausible canonical repair.

Principle-based inference: **policy belongs where a user-facing presentation choice remains.** Exposing policy for a deterministic repair would add configuration surface without creating a real choice, which weakens usability and maintainability.

Product goal: when classification leaves no meaningful user-facing presentation choice, `evfmt` should generate the canonical spelling owned by that context instead of asking users to opt into a spelling the formatter cannot defend as an independent presentation preference. That goal explains why modifier defects, tag-context presentation, and unsanctioned selectors are handled outside policy.

Evidence gap: this is a product boundary, not a Unicode theorem. There is no user-study evidence here showing that this exact boundary is the one most users expect.

### Single-State Cleanup

Manually reviewed: yes.

Facts: selector runs can be redundant, conflicting, defective, forced, or unsanctioned after the local Unicode-related structure is known. `evfmt` can rewrite those cases by changing only `FE0E` and `FE0F`.

Principle-based inference: **cleanup is favored when the formatter can identify a single canonical selector state.** It improves reproducibility and keeps formatting idempotent without asking users to choose between states that are not meaningful presentation preferences.

Product goal: fixed cleanup should produce selector spellings that are canonical for the recognized local structure and likely to be accepted by mainstream renderers. The goal is not to preserve every historical or byte-level spelling, nor to validate the entire emoji sequence as RGI.

The weak point is whether automatic repair is better than warning-only behavior for defects, legacy spellings, and compatibility-sensitive files. Determinism, canonical output, and a narrow selector-only repair surface support automatic cleanup. User evidence showing that warnings without repair create more churn or confusion than automatic selector cleanup would strengthen the argument; compatibility evidence that the cleaned selector bytes are meaningful to important consumers would overturn it for some cases.

### Context-Specific Rules

Manually reviewed: yes.

Inference: **modifier, tag, and ZWJ-related contexts justify separate classification rules because they make different selector states meaningful, defective, or irrelevant.** A flatter rule set would be easier to maintain, but it would either erase those distinctions or move hidden exceptions into policy, code, or tests.

### Modifier-Defect Cleanup

Manually reviewed: yes.

Facts: current UTS #51 defines an emoji modifier sequence as an `emoji_modifier_base` followed by an `emoji_modifier`. Its presentation guidance says the modifier automatically implies emoji presentation. It directs implementations to ignore `FE0F` in the defective legacy spelling and recommends omitting that selector from newly generated modifier sequences. The `Emoji_Modifier_Base` property supplies the normative boundary for this sequence class.

Historical boundary: the published UTR #51 Revisions 3 and 5, along with the surrounding draft Revisions 2, 4, and 6, defined ED-13 as `(emoji_modifier_base | emoji_base_variation_sequence) emoji_modifier`. The `emoji_base_variation_sequence` branch required the base and `FE0F` to form a valid variation sequence; the accompanying guidance classified support for any other base-selector pair as non-conformant. Revision 7, published with Unicode Emoji 3.0, narrowed ED-13 to `emoji_modifier_base emoji_modifier` and introduced the compatibility note about older data containing an intervening selector. The documented legacy grammar therefore covers sanctioned `Emoji_Modifier_Base FE0F Emoji_Modifier` spellings.

Archive evidence: the official Emoji 2.0 `emoji-sequences.txt` uses bare base-plus-modifier code points in its machine-readable fields. Ten rendered comment glyphs contain literal `Emoji_Modifier_Base FE0F Emoji_Modifier` strings for `U+261D` and `U+270C`; Unicode 8.0 registered both base-plus-`FE0F` pairs as emoji-style variation sequences. A scan of all text files in the public Emoji archive snapshots and the corresponding `unicodetools` emoji-data history found no machine-readable `Emoji_Modifier_Base FE0F Emoji_Modifier` entry. Every literal occurrence used a sanctioned base-plus-`FE0F` variation sequence.

Inference: **removing the sanctioned `FE0F` in the documented legacy modifier spelling cleans ignored selector state.** It preserves the modifier sequence and produces the spelling that UTS #51 recommends for newly generated text.

Accounting inference: the historical standard defined the legacy modifier spelling through a sanctioned variation sequence and classified other base-selector pairs as non-conformant. The archived examples follow the same boundary. `evfmt` therefore uses `modifier_defective_selectors` when the base has `Emoji_Modifier_Base` and the intervening `FE0F` is sanctioned for that base. When `FE0F` is not sanctioned for that base, `evfmt` uses `unsanctioned_selectors`. Both paths remove the selector and preserve the same base-plus-modifier output.

Explicit text case: `base FE0E modifier` consists of a sanctioned text presentation sequence followed by an emoji modifier. `evfmt` preserves `FE0E` and the modifier scalar.

Evidence gap: UTS #51 strongly supports omitting the sanctioned legacy `FE0F` spelling from newly generated text. Automatic repair remains an `evfmt` product choice; warning-only diagnostics would also comply with that guidance. Preserving `FE0E` retains an explicit source presentation request and leaves the following modifier outside an emoji modifier sequence. Evidence that this preserved spelling causes harmful renderer or interchange behavior should reopen that choice.

Sources: [current UTS #51 §2.4](https://www.unicode.org/reports/tr51/tr51-29.html#Diversity) states the presentation implication and defective-sequence handling, while [ED-13](https://www.unicode.org/reports/tr51/tr51-29.html#def_emoji_modifier_sequence) defines the current sequence through `Emoji_Modifier_Base`. [UTR #51 Revision 5 definitions](https://www.unicode.org/reports/tr51/tr51-5-archive.html#Emoji_Definitions) record the earlier ED-13 grammar, and its [modifier guidance](https://www.unicode.org/reports/tr51/tr51-5-archive.html#Diversity) requires a valid variation sequence before the modifier. [UTR #51 Revision 7 §2.4](https://www.unicode.org/reports/tr51/tr51-7.html#Emoji_Implementation_Notes) records the narrowed grammar's older-data compatibility rule. The official [Emoji 2.0 `emoji-sequences.txt`](https://www.unicode.org/Public/emoji/2.0/emoji-sequences.txt) supplies the archived data and rendered comments described above, and [Unicode 8.0 `StandardizedVariants.txt`](https://www.unicode.org/Public/8.0.0/ucd/StandardizedVariants.txt) records the two variation sequences. The pinned [`emoji-data.txt`](../../evfmt/data/emoji-data.txt) and [`emoji-variation-sequences.txt`](../../evfmt/data/emoji-variation-sequences.txt) supply the exact modifier-base and sanctioned-selector sets used by the implementation.

### Additional Modifier-Context Cleanup

Manually reviewed: yes.

Facts: current UTS #51 defines `# FE0F` as an emoji presentation sequence and `U+1F3FB` as an `Emoji_Modifier`. The combined `# FE0F U+1F3FB` is not an emoji modifier sequence because `#` lacks `Emoji_Modifier_Base`. The `FE0F` remains sanctioned as part of the presentation sequence. UTS #51 assigns automatic emoji-presentation force to a following modifier within the `Emoji_Modifier_Base Emoji_Modifier` sequence class.

Historical evidence: the modifier implication rule has treated recognized modifier structure as emoji-presentation evidence since at least UTS #51 Version 3.0. The 2016 tag-sequence proposal likewise stated that tag sequences request emoji presentation and consequently need no variation selector. Both records assign presentation meaning to recognized suffix structure.

Product inference: **a following `Emoji_Modifier` provides enough emoji-presentation evidence for `evfmt` to prefer the bare-base spelling even when the base lacks `Emoji_Modifier_Base`.** `evfmt` applies this inference only to a sanctioned `FE0F` immediately before an `Emoji_Modifier` on a recognized emoji base. It rewrites `# FE0F U+1F3FB` to `# U+1F3FB`, preserving the base and modifier scalars. `additional_defective_selectors` records this formatter-defined cleanup separately from UTS-defined modifier defects.

Scope clarification for the suffix-evidence analogy: bare `base U+20E3` has an independently legitimate Unicode-text interpretation, as documented in the [keycap-character policy rationale](policy.markdown#keycap-character-policy-domain). A following `U+20E3` therefore leaves presentation intent unresolved. The modifier and tag records above supply presentation evidence for their respective suffix contexts.

Sources: [Unicode 17.0 `emoji-data.txt`](https://www.unicode.org/Public/17.0.0/ucd/emoji/emoji-data.txt) defines `Emoji_Modifier` and `Emoji_Modifier_Base`. The [2016 tag-sequence proposal](https://www.unicode.org/L2/L2016/16008-custom-emoji.pdf) records the explicit presentation-request rationale for tag suffixes. The keycap-character policy rationale collects the primary sources for its historical claims.

Evidence gap: extending modifier evidence beyond `Emoji_Modifier_Base` remains an `evfmt` prediction. Direct renderer and interchange evidence for these additional shapes is incomplete. Evidence that a consumer distinguishes `base FE0F Emoji_Modifier` from `base Emoji_Modifier` on a non-modifier base should reopen the cleanup. A future Unicode grammar that gives retained `FE0F` a distinct role in this context should do the same.

### Tag-Context Selector Accounting

Manually reviewed: yes.

Facts: the core Unicode Standard delegates emoji tag-sequence semantics to UTS #51. Current UTS #51 defines `emoji_tag_sequence` with a broad `tag_base` grammar: an emoji character, emoji modifier sequence, or emoji presentation sequence can precede the tag specification. It also defines `emoji_zwj_element` as either an emoji core sequence or an emoji tag sequence, so an emoji tag sequence can be a component of an emoji ZWJ sequence. The same version's valid flag tag sequences, however, restrict flag `tag_base` to `U+1F3F4 BLACK FLAG`. The Unicode 17.0 RGI tag-sequence data uses that emoji-default base, and its RGI ZWJ-sequence data contains no tag characters. The grammar therefore permits tag sequences in ZWJ sequences even though the current RGI repertoire does not exercise that form.

Draft-history evidence: the 2019 proposed update for UTS #51 experimented with making emoji sequence grammar more general. It placed `tag_modifier` in the same `emoji_modification` family as emoji modifiers and keycap modifiers, and its review notes considered adding `emoji_tag_sequence` to `emoji_zwj_element`. UTC #161 separately recorded actions to update UTS #51 based on David Corbett's PRI #405 feedback, remove the review notes about allowing tag sequences within ZWJ sequences, and produce a separate document on the costs and benefits of making emoji ZWJ and modifier grammar more general. These are related review records, but the action items about tag-in-ZWJ review notes and a separate grammar analysis should not be attributed to Corbett as his own proposals.

Normative version history: Unicode Emoji 14.0 and earlier limited `emoji_zwj_element` to an emoji character, emoji presentation sequence, or emoji modifier sequence. Unicode Emoji 15.0 subsequently changed the normative grammar to admit `emoji_tag_sequence` as an `emoji_zwj_element`, and Unicode Emoji 17.0 retains that rule. The 2019 removal of review notes therefore does not describe the current grammar.

Inference: **tag contexts need their own selector accounting because the standard history leaves base presentation in tag contexts broader than RGI practice and different from emoji-modifier defect handling.** `FE0F` before a tag specification on an emoji-default base counts as `tag_redundant_selectors` because the recognized tag context canonicalizes that base as bare emoji-default. This cleanup is independent of the configurable presentation policy. `FE0E` before a tag specification is a sanctioned selector whose requested text presentation conflicts with the formatter's canonical base presentation in tag context. UTS #51 permits a bare text-default base before a tag specification; in that recognized tag context, `evfmt` supplies emoji presentation for its canonical output.

Sources: [UTS #51 Version 14.0 ED-15a](https://www.unicode.org/reports/tr51/tr51-21.html#def_emoji_zwj_element) has the earlier restricted ZWJ-element grammar. [UTS #51 Version 15.0 ED-15a](https://www.unicode.org/reports/tr51/tr51-23.html#def_emoji_zwj_element) formally admits emoji tag sequences as ZWJ elements. [UTS #51 Version 17.0 ED-15a](https://www.unicode.org/reports/tr51/tr51-29.html#def_emoji_zwj_element) defines the current ZWJ-element grammar, while its [ED-14a](https://www.unicode.org/reports/tr51/tr51-29.html#def_emoji_tag_sequence) and [Annex C](https://www.unicode.org/reports/tr51/tr51-29.html#valid-emoji-tag-sequences) define the current tag grammar and valid flag tag-sequence constraints. [Unicode 17.0 `emoji-sequences.txt`](https://www.unicode.org/Public/17.0.0/emoji/emoji-sequences.txt) lists the current RGI tag sequences, while [Unicode 17.0 `emoji-zwj-sequences.txt`](https://www.unicode.org/Public/17.0.0/emoji/emoji-zwj-sequences.txt) lists the current RGI ZWJ sequences. The 2019 proposed update shows both the [review notes about adding tag sequences to the ZWJ grammar](https://www.unicode.org/L2/L2019/19351-uts51-17-draft.pdf#page=11) and the broader [`emoji_modification` grammar](https://www.unicode.org/L2/L2019/19351-uts51-17-draft.pdf#page=14). The UTC #161 minutes record the action to [update UTS #51 based on David Corbett's feedback](https://www.unicode.org/L2/L2019/19323.htm#161-A6), [remove the tag-in-ZWJ review notes](https://www.unicode.org/L2/L2019/19323.htm#161-A10), and [produce a separate grammar analysis](https://www.unicode.org/L2/L2019/19323.htm#161-A11); [Corbett's PRI #405 feedback](https://www.unicode.org/review/pri405/feedback.html) is also available directly.

Evidence gap: this supports separate tag-specific accounting and the current canonicalization choice, but admitting tag sequences in the ZWJ grammar does not establish renderer support or user-authored interchange practice, and the current RGI ZWJ repertoire does not exercise the form. The draft-history material documents proposals and committee actions, not historical implementation behavior. The evidence also does not prove that every well-formed broad base-and-tag spelling should normalize this way forever. Future UTS #51 or RGI changes that assign interchange value to another base presentation before a tag specification should reopen this rule.

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
