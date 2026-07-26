# Design Note: Classification

Read when: changing selector-slot classification, context-specific selector accounting, or policy-domain selection.

Defines: how recognized structure classifies selector slots into reasonable states, context-specific selector accounting, and policy domains.

Does not define: selector slots or recognition invariants. Selector slots live in [formatting.markdown](../core/formatting.markdown). Recognition invariants live in [recognition.markdown](recognition.markdown). Policy predicates and defaults live in [policy.markdown](policy.markdown).

## Inputs

Recognized structures may expose more information than these rules currently need. They must at least answer these questions for each selector slot:

- whether the slot has no base, or a base without sanctioned variation sequences
- if the slot has a base, whether following structure is an emoji modifier, a tag specification, or `U+20E3 COMBINING ENCLOSING KEYCAP`
- whether a base followed by an emoji modifier has the Unicode `Emoji_Modifier_Base` property
- whether the base is emoji-default or text-default
- which selector, if any, is first in the slot

## Slot Classification

Classification may use the recognized context, the neighboring non-selector scalars, and the first selector in the slot when present.

The scalar before a slot, if present, is the base. A base has sanctioned variation sequences when it appears in the pinned variation-sequence data.

Apply the first matching rule:

| Rule | Condition                                                            | Reasonable states      |
| ---- | -------------------------------------------------------------------- | ---------------------- |
| 1    | No base, or base has no sanctioned variation sequences               | `none`                 |
| 2    | Slot is followed by an emoji modifier and starts with `FE0E`         | `FE0E`                 |
| 3    | Slot is followed by an emoji modifier and does not start with `FE0E` | `none`                 |
| 4    | Slot is followed by a tag specification and base is emoji-default    | `none`                 |
| 5    | Slot is followed by a tag specification and base is text-default     | `FE0F`                 |
| 6    | Slot starts with `FE0E`                                              | `none`, `FE0E`         |
| 7    | Slot starts with `FE0F`                                              | `none`, `FE0F`         |
| 8    | Slot has no selector                                                 | `none`, `FE0E`, `FE0F` |

Rules are ordered only to resolve overlapping conditions. Fixed cleanup is not defined by rule number; it is the case where the resulting reasonable-state set has exactly one state.

Only a selector that immediately follows the base can be part of the reasonable-state choice.

## Selector Accounting

Classification also decides how non-canonical selector changes are counted.

A sanctioned selector is a presentation selector that immediately follows a base with sanctioned variation sequences. Any other presentation selector is unsanctioned.

Generic accounting:

- unsanctioned selectors count as `unsanctioned_selectors`
- sanctioned `FE0F` before an emoji modifier counts as `modifier_defective_selectors` when the modifier still attaches to the bare base and the base has `Emoji_Modifier_Base`
- the same selector removal counts as `additional_defective_selectors` when the base lacks `Emoji_Modifier_Base`
- sanctioned selectors dropped because policy chooses the bare state count as `policy_redundant_selectors`
- selector slots with no selector that policy resolves to an explicit selector count as `presentation_decisions`

Tag-context accounting is separate from ordinary policy redundancy and emoji-modifier defects:

- sanctioned `FE0F` before a tag specification on an emoji-default base: `tag_redundant_selectors`
- sanctioned `FE0E` before a tag specification on an emoji-default base: `tag_conflicting_selectors`
- no selector before a tag specification on a text-default base: `tag_forced_presentations`
- sanctioned `FE0E` before a tag specification on a text-default base: `tag_conflicting_selectors` and `tag_forced_presentations`

These tag counters describe `evfmt`'s canonicalization of a recognized tag context. UTS #51 admits other base-and-tag spellings that `evfmt` treats as non-canonical, and these counters do not create a user-facing policy choice for tag presentation.

`additional_defective_selectors` is an `evfmt` formatter classification. It records the narrow recognized shape `Emoji FE0F Emoji_Modifier` when the sanctioned selector follows a base without `Emoji_Modifier_Base`. The name describes an additional fixed cleanup owned by the formatter. UTS #51 and its `Emoji_Modifier_Base` boundary continue to determine Unicode conformance status.

## Policy Domains

Policy applies only when classification leaves more than one reasonable state and the base has sanctioned variation sequences.

The policy domain is local:

- keycap-character domain when the selector slot is immediately followed by `U+20E3 COMBINING ENCLOSING KEYCAP`
- ordinary domain, for non-keycap selector slots, otherwise

The policy decision itself is defined in [policy.markdown](policy.markdown).

## ZWJ-Related Text

ZWJ-related recognition preserves non-selector ZWJ structure. Classification is component-wise for selector slots on recognized components. Selectors attached to ZWJ links rather than to a component slot have only the `none` reasonable state.
