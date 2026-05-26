# Rationale: Policy

Read when: auditing, challenging, or replacing the formatter policy predicates, policy defaults, or policy-key domains described in [`../designs/features/policy.markdown`](../designs/features/policy.markdown).

Defines: non-normative evidence, assumptions, gaps, and tradeoff reasoning behind user-facing policy choices, bare-display interpretation, default policy behavior, and keycap policy attention.

Does not define: current policy behavior, recognition behavior, or slot-classification behavior. For behavior, see [`../designs/features/policy.markdown`](../designs/features/policy.markdown), code, tests, and public API documentation. For recognition and classification rationale, see [`recognition.markdown`](recognition.markdown) and [`classification.markdown`](classification.markdown).

This file follows the rationale authoring and manual review conventions in [`authoring.markdown`](authoring.markdown).

## Inferences

### Bare Display Assumption

Manually reviewed: yes.

Product assumption: **when a selector slot is represented by a policy key and both bare and explicit selector spellings are reasonable, bare rendering aligns with either the text-selector rendering or the emoji-selector rendering.** `evfmt` treats that as a display assumption only. It does not decide whether bare spelling is canonical, and it does not apply to fixed-cleanup cases that resolve before policy.

This assumption lets `bare_as_text` be a binary presentation-side predicate: bare spelling is treated as text presentation when the policy key is in `bare_as_text`, and as emoji presentation otherwise. That is the same kind of policy-model tie-breaker as the spelling-shape choice below; it is not a claim that Unicode presentation semantics outside `evfmt` are only binary.

Evidence gap: renderers, fonts, editors, terminals, or platforms may display the bare spelling for a policy key in a way that is meaningfully distinct from both explicit selector forms. Evidence of that behavior for important environments should trigger a revisit of the affected default policy membership or policy model.

### Final Selector Tie-Breaker

Manually reviewed: yes.

Facts: policy is consulted only after classification has left more than one reasonable selector state. At that point, `evfmt` still formats toward one canonical representative instead of preserving whichever reasonable spelling appeared in the input.

Product assumption: **when bare spelling is not preferred for a policy key, the explicit selector for the selected presentation side is the canonical spelling.** This is why the public predicate is `prefer_bare`: within this canonical policy model, `prefer_bare = false` means bare is not the canonical spelling, so the selected side is written explicitly. A hypothetical negative `prefer_selector` flag would not express the same model, because `prefer_selector = false` would not by itself choose bare as canonical.

This tie-breaker mainly rejects the no-op alternative, where every reasonable spelling is left unchanged. A canonical policy must choose whether bare or explicit spelling wins for the key; it cannot leave both as equally canonical. That choice applies only within the active policy setting: if there is an important reason to prefer bare for a key, the default policy or the user's policy should put that key in `prefer_bare`; otherwise, explicit spelling is the final tie-breaker.

### Small Policy Context Surface

Manually reviewed: yes.

Facts: after local selector context classification and fixed cleanup, the current policy surface uses two domains indexed by variation-sequence base character: non-keycap policy keys and keycap-character policy keys.

Product assumption: **the public policy context surface should expose only durable, named distinctions that users can reason about.** "Small" does not mean a fixed count. It means policy domains should not be derived from arbitrary surrounding sequence topology.

Domain-qualified base-indexing is the current public realization of that constraint, not the fundamental principle. The current non-keycap/keycap domains keep policy compact while preserving the keycap distinction argued by the keycap-specific rationale below.

Revisit this policy-shape choice if future Unicode data introduces policy-relevant selector contexts that do not fit the current domains, or if real user needs show that the current non-keycap/keycap policy surface cannot express an important formatting choice. The response should still expose a named policy distinction that users can reason about, such as adding a small domain or replacing the current domain split with another small policy model, rather than encoding arbitrary surrounding sequence topology into policy keys.

### Keycap Policy Domain

Manually reviewed: yes.

Facts: keycap sequences use a base followed by `U+20E3` COMBINING ENCLOSING KEYCAP, optionally with a presentation selector between them. `U+20E3` predates Unicode emoji, and bare keycap spellings such as `[0-9#*] U+20E3` existed as ordinary Unicode text before modern emoji qualification. Older emoji mappings also used bare keycap spellings for carrier emoji compatibility. Modern emoji qualification uses `[0-9#*] FE0F U+20E3` for fully qualified emoji keycaps, while emoji data can still record bare keycap forms as unqualified emoji data.

Inference: **keycap-character selector slots need a separate policy-key domain because the independently justified defaults for ordinary ASCII slots and keycap slots require different canonical spelling choices for the same base characters.** Ordinary `#`, `*`, and digit slots should prefer bare spelling: these characters are common source text, and the default formatter should not introduce invisible selectors into ordinary ASCII. Bare keycap sequences, however, are structured `base U+20E3` spellings with historical text use and emoji-data ambiguity. When the default policy treats them as text-side rather than emoji intent, the canonical text-side spelling should be explicit `FE0E` before `U+20E3`, not the ambiguous bare keycap spelling. A base-character-only policy could not express both defaults for `#`, `*`, and digits. The keycap-character domain is therefore needed to keep the ordinary ASCII bare default and the keycap text-side default simultaneously expressible.

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

### ASCII Non-Keycap Bare Default

Manually reviewed: yes.

Facts: ASCII digits, `#`, and `*` are common source characters, and they also have sanctioned text and emoji variation sequences. They frequently appear as syntax, numeric data, Markdown markers, operators, and other ordinary text rather than as deliberate glyph-presentation requests.

Product assumption: **ASCII selector slots outside keycap context should default to bare text-style spelling.** The strongest reason is to avoid generating `FE0E` in common ASCII source text. Under the canonical policy model, choosing explicit `FE0E` as the text-side canonical spelling would generate invisible presentation controls for bare non-keycap ASCII input. Choosing bare as the canonical text-side spelling avoids that generation, but also normalizes existing non-keycap ASCII `FE0E` to bare.

This is a policy-model tradeoff. Preserving an existing non-keycap ASCII `FE0E` without generating new `FE0E` would require a more source-sensitive policy shape rather than a different default membership in the current canonical policy sets.

### Text-Default Non-Keycap Bare Interpretation

Manually reviewed: yes.

Facts: some non-keycap policy keys are text-default bases outside the ASCII set. Their bare spelling follows Unicode text default, and both `FE0E` and `FE0F` are sanctioned selector states.

Product assumption: **the default policy treats bare text-default input in non-keycap selector slots as text presentation unless another rationale gives that key a stronger emoji-side default.** This is the default membership choice behind putting `text-defaults` in `bare_as_text`. It follows Unicode default presentation for bare source text: for non-ASCII text-default keys that do not prefer bare, the canonical explicit spelling is `FE0E`.

RGI data does not decide this text-vs-emoji default for non-keycap, non-RGI source text. RGI preference only constrains the chosen canonical form when default policy chooses a form with emoji presentation and the RGI spelling is reasonable.

Evidence gap: user intent for bare non-ASCII text-default characters is uncertain. A useful counterargument is ZWJ repair tolerance: if bare text-default components in ZWJ-related input often represent missing `FE0F`, an emoji-side default would avoid introducing `FE0E` and would preserve emoji rendering more often. Evidence that common source files use bare text-default forms primarily as emoji presentation, for example because tools often display emoji while copying or storing the bare text-default form, should move the affected keys out of `bare_as_text` or into another more specific policy default.

### Emoji-Default Bare Policy

Manually reviewed: yes.

Facts: some standalone variation-sequence bases are emoji-default, so their bare spelling already carries emoji presentation by Unicode default.

Inference: **emoji-default bases should use their bare canonical spelling.** For emoji-default bases, a bare canonical spelling matches the Unicode default while avoiding redundant `FE0F`. It is also the RGI spelling for the sequence; adding redundant `FE0F` does not produce the RGI form. For future renderers, fonts, and platforms, honoring the Unicode default is the least-surprising long-term behavior.

Evidence gap: this supports bare canonical spelling for emoji-default bases, but it does not prove every document type prefers Unicode-default compactness over explicit selector spelling.

## Skeptical Q&A

### Are old bare-keycap emoji inputs being misread as text?

Manually reviewed: yes.

**Old bare-keycap emoji inputs may be misread as text.** Bare keycap spellings have both historical text legitimacy and historical emoji compatibility use, so the intended presentation can be hard to tell. The current Unicode default presentation is therefore a reasonable tie-breaker. Modern emoji palettes also do not generate bare-keycap emoji. This should be revisited if contemporary evidence favors treating bare keycaps as emoji.
