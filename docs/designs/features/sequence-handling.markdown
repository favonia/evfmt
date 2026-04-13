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

Policy is only for genuinely ambiguous standalone variation-sequence slots.

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

- `FE0E` on a ZWJ component is replaced with `FE0F` where needed for fully-qualified form — this departs from [UTS #51](https://www.unicode.org/reports/tr51/), which treats `FE0E` as breaking the sequence, but honoring that would require removing ZWJ joiners, violating the "only selectors change" invariant
- selectors required for the fully-qualified form are preserved or inserted
- selectors that are redundant or unsupported under that discipline are removed

ZWJ handling is fixed cleanup, not policy.

### Orphaned or unsanctioned selectors

Selectors that are not part of a sanctioned local selector-bearing context are removed.

## Slot model

The durable slot distinction is:

- standalone variation-sequence slot
- fixed-cleanup sequence slot
- not-a-slot

Each real slot records:

- current variation selector state
- whether extra variation selectors were present
- which of `none`, `FE0E`, and `FE0F` remain reasonable after context-aware reduction

## Violation model

The important split is:

- unsanctioned or structurally broken selector usage
- fixed-cleanup sequence defects
- standalone variation-selector state mismatches that are already non-canonical under the current policy
- genuinely ambiguous standalone slots that need policy

A redundant variation selector belongs to the third class, not the first. It is still sanctioned Unicode structure; it is simply non-canonical under the active formatter policy because the same slot canonically stays bare.

This split matters because only the last class is a real policy choice. The other three classes already have a canonical repair.

Implementation APIs may surface these categories as review findings with valid decisions. That API shape does not change the sequence-family contracts here.

## Relation to policy

| Context                       | Policy applies?                           | Behavior                                             |
| ----------------------------- | ----------------------------------------- | ---------------------------------------------------- |
| Standalone variation sequence | Yes, if multiple reasonable states remain | Governed by the preferred-bare and bare-as-text sets |
| Keycap                        | No                                        | Fixed canonical form: `base FE0F 20E3`               |
| Modifier sequence defect      | No                                        | Remove legacy `FE0F` before the modifier             |
| ZWJ-related sequence          | No                                        | Apply fixed fully-qualified sequence discipline      |
| Not-a-slot                    | No                                        | Remove illegal selector usage                        |
