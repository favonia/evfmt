# Design Note: Classification

Read when: changing selector-slot classification, fixed-cleanup conditions, or policy-domain selection.

Defines: how recognized structure classifies selector slots into reasonable states and policy domains.

Does not define: selector slots or recognition invariants. Selector slots live in [formatting.markdown](../core/formatting.markdown). Recognition invariants live in [recognition.markdown](recognition.markdown). Policy predicates and defaults live in [policy.markdown](policy.markdown).

## Inputs

Recognized structures may expose more information than these rules currently need. They must at least answer these questions for each selector slot:

- whether the slot has no base, or a base without sanctioned variation sequences
- if the slot has a base, whether following structure is an emoji modifier, a tag specification, or `U+20E3 COMBINING ENCLOSING KEYCAP`
- whether the base is emoji-default
- which selector, if any, is first in the slot

## Slot Classification

Classification may use the recognized context, the neighboring non-selector scalars, and the first selector in the slot when present.

The scalar before a slot, if present, is the base. A base has sanctioned variation sequences when it appears in the pinned variation-sequence data.

Apply the first matching rule:

| Rule | Condition                                                             | Reasonable states      |
| ---- | --------------------------------------------------------------------- | ---------------------- |
| 1    | No base, or base has no sanctioned variation sequences                | `none`                 |
| 2    | Slot is followed by an emoji modifier and starts with `FE0E`          | `FE0E`                 |
| 3    | Slot is followed by an emoji modifier and does not start with `FE0E`  | `none`                 |
| 4    | Slot is followed by a tag specification and base is emoji-default     | `none`                 |
| 5    | Slot is followed by a tag specification and base is not emoji-default | `FE0F`                 |
| 6    | Slot starts with `FE0E`                                               | `none`, `FE0E`         |
| 7    | Slot starts with `FE0F`                                               | `none`, `FE0F`         |
| 8    | Slot has no selector                                                  | `none`, `FE0E`, `FE0F` |

Rules are ordered only to resolve overlapping conditions. Fixed cleanup is not defined by rule number; it is the case where the resulting reasonable-state set has exactly one state.

Selectors after the first selector in a slot are never part of the reasonable-state choice. They are removed as unsupported selector usage unless another narrower rule accounts for them.

## Policy Domains

Policy applies only when classification leaves more than one reasonable state and the base has sanctioned variation sequences.

The policy domain is local:

- keycap-character domain when the selector slot is immediately followed by `U+20E3 COMBINING ENCLOSING KEYCAP`
- ordinary domain, for non-keycap selector slots, otherwise

The policy decision itself is defined in [policy.markdown](policy.markdown).

## ZWJ-Related Text

ZWJ-related recognition preserves non-selector ZWJ structure. Classification is component-wise for selector slots on recognized components. Selectors attached to ZWJ links rather than to a component slot have only the `none` reasonable state.
