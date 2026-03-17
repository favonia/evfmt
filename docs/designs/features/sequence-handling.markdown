# Design Note: Sequence Handling

Read when: changing selector handling across sequence families, or changing which contexts are fixed cleanup versus policy.

Defines: the durable sequence-family contracts for variation-selector handling.

## Scope

`evfmt` is responsible for `FE0E` / `FE0F` handling in the selector-bearing contexts that matter to canonical source text:

- standalone variation-sequence contexts
- keycap sequences
- modifier sequences
- ZWJ-related sequences
- orphaned, extra, or unsanctioned selector usage

It is not a general emoji validator. Sequence recognition exists to support selector canonicalization, not to validate every Unicode emoji property exhaustively.

## Core decision boundary

Policy is only for genuinely ambiguous standalone variation-selector slots.

Sequence-specific cleanup rules must resolve before policy.

## Sequence-family contracts

### Standalone variation-sequence contexts

For a standalone base with variation-sequence data, the formatter may need policy to choose among bare, `FE0E`, and `FE0F`.

Only standalone variation-sequence slots may remain policy-ambiguous after context-aware cleanup.

### Keycap sequences

In true keycap context, the canonical form is:

```text
[0-9#*] FE0F 20E3
```

Keycap handling is fixed cleanup, not policy.

### Modifier sequences

Legacy `FE0F` immediately before an emoji modifier is non-canonical and must be removed.

Modifier handling is fixed cleanup, not policy.

### ZWJ-related sequences

ZWJ-related selector handling follows fully-qualified generation discipline.

In canonical output:

- `FE0E` is not kept where it breaks the generated ZWJ emoji form
- selectors required for the fully-qualified form are preserved or inserted
- selectors that are redundant or unsupported under that discipline are removed

ZWJ handling is fixed cleanup, not policy.

### Orphaned or unsanctioned selectors

Selectors that are not part of a sanctioned local selector-bearing context are removed.

## Slot model

The durable slot distinction is:

- standalone EVS slot
- fixed-cleanup sequence slot
- not-a-slot

Each real slot records:

- current selector state
- whether extra selectors were present
- which of `none`, `FE0E`, and `FE0F` remain reasonable after context-aware reduction

## Violation model

The important split is:

- unsanctioned or structurally broken selector usage
- fixed-cleanup sequence defects
- standalone selector-state mismatches that are already non-canonical under the current policy
- genuinely ambiguous standalone slots that need policy

`Redundant selector` belongs to the third class, not the first. A redundant selector is still sanctioned Unicode structure; it is simply non-canonical under the active formatter policy because the same slot canonically stays bare.

This split matters because only the last class is a real policy choice. The other three classes already have a canonical repair.

## Relation to policy

| Context                  | Policy applies?                           | Behavior                                                       |
| ------------------------ | ----------------------------------------- | -------------------------------------------------------------- |
| Standalone EVS           | Yes, if multiple reasonable states remain | Governed by `--prefer-bare-for` and `--treat-bare-as-text-for` |
| Keycap                   | No                                        | Fixed canonical form: `base FE0F 20E3`                         |
| Modifier sequence defect | No                                        | Remove legacy `FE0F` before the modifier                       |
| ZWJ-related sequence     | No                                        | Apply fixed fully-qualified sequence discipline                |
| Not-a-slot               | No                                        | Remove illegal selector usage                                  |
