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

## Structural Recognition Contract

The scanner must preserve enough recognized structure for findings analysis to apply the contracts below in one formatting pass:

- Losslessness: every input byte belongs to exactly one scan item, and concatenating scan item source slices reconstructs the original input.
- Selector-only idempotence: inserting, removing, or replacing `FE0E`/`FE0F` must not reveal newly recognized emoji-related structure on a second pass.
- Selector locality: `FE0E` and `FE0F` attach to the nearest scanner-recognized context that can own them; selectors no such context can own are preserved as unsanctioned selector runs for cleanup.
- Emoji-like permissiveness: scanner grouping is based on potentially emoji-relevant structure, not on RGI status or final semantic validity.
- ZWJ visibility: valid ZWJ sequences and malformed ZWJ-related structures made only of recognized emoji/selector/ZWJ material stay visible to findings analysis so selector cleanup can preserve non-selector text.

Concrete scanner state shapes and local edge cases belong in scanner comments and scanner/findings tests. This note records the cross-module contract those comments implement.

## Core decision boundary

Policy is only for genuinely ambiguous standalone variation positions.

Sequence-specific cleanup rules must resolve before policy.

## Sequence-family contracts

### Standalone variation-sequence contexts

For a standalone base with variation-sequence data, the formatter may need policy to choose among bare, `FE0E`, and `FE0F`.

Only standalone variation-sequence slots and standalone keycap-character slots may remain policy-ambiguous after context-aware cleanup.

### Fixed cleanup for singleton-base slots

When the first modification after a singleton base is an emoji modifier or a tag specification, or when the base appears as a component of a multi-component ZWJ sequence, selector handling is fixed cleanup rather than policy. A standalone singleton base whose first modification is `U+20E3` uses keycap-character policy when the base has variation-sequence data.

Only the first `EmojiModification` after the base determines this base-presentation context. Later modifier, keycap, or tag material is preserved in source order and its own trailing presentation selectors are still cleaned, but it does not move the base slot into a different policy domain or fixed-cleanup family.

The base-presentation decision is resolved by this precedence:

| Precedence | Context                                                                         | Canonical base presentation |
| ---------- | ------------------------------------------------------------------------------- | --------------------------- |
| 1          | The chosen presentation would be unsanctioned                                   | bare                        |
| 2          | First modification is an emoji modifier                                         | bare                        |
| 3          | First modification is a tag, or multi-component ZWJ context, emoji-default base | bare                        |
| 4          | First modification is a tag, or multi-component ZWJ context, other base         | emoji                       |
| 5          | No fixed-cleanup context remains                                                | use policy                  |

Rule 5 is the boundary back to policy. Keycap-character slots query the keycap-character domain, and ordinary standalone slots query the ordinary domain. This is not a policy hook for emoji modifiers, tag modifiers, or multi-component ZWJ components.

Every `[0-9#*]` keycap base has standardized text and emoji presentation sequences and is text-default in the pinned Unicode data. Scanner grouping is broader than RGI keycaps, so policy queries outside that standardized base set may still collapse to selector cleanup when the base lacks variation-sequence data.

### Keycap sequences

In true keycap context, the RGI emoji keycap form (UTS #51 §1.4.5) is:

```text
[0-9#*] FE0F 20E3
```

Standalone bare keycap inputs (`[0-9#*] 20E3`) use keycap-character policy. The default policy treats bare keycap-character forms as text, so the default canonical spelling is:

```text
[0-9#*] FE0E 20E3
```

Explicit `FE0E` in the same standalone slot is preserved rather than overwritten. `[0-9#*] FE0E` is a standardized text variation sequence (`StandardizedVariants.txt`), and per the Unicode Standard Chapter 23 a variation selector's effect on the base may propagate to subsequent combining marks — so `[0-9#*] FE0E 20E3` has a well-defined compositional meaning (a text-styled digit enclosed by a keycap mark inheriting that appearance). It is not an RGI emoji sequence, but it is not unsanctioned either; promoting `FE0E` to `FE0F` here would destructively convert a text-style form into an emoji one.

Explicit `FE0F` in a standalone keycap-character slot is preserved under the default policy. Keycap components inside multi-component ZWJ sequences are still fixed cleanup and normalize to emoji presentation.

### Modifier sequences

Legacy `FE0F` immediately before an emoji modifier is non-canonical and must be removed.

Modifier handling is fixed cleanup, not policy.

### ZWJ-related sequences

ZWJ-related selector handling follows fully-qualified generation discipline after structural recognition.

Under the structural recognition contract above, recognized ZWJ-related structure is preserved before cleanup. The findings/formatting layer then decides which selectors are valid, repairable, or removable under the formatter invariants.

For cleanup, the relevant ZWJ-like shapes are:

- no emoji component: only presentation selectors attached to ZWJ links are removed
- one emoji component: the component uses the ordinary singleton or flag cleanup path, while surrounding ZWJ links are preserved and link-attached selectors are removed
- multiple emoji components: each component is in true ZWJ component context and standalone ordinary/keycap policy never applies

In canonical output:

- `FE0E` on a ZWJ component is replaced with `FE0F` where needed for fully-qualified form — this departs from [UTS #51](https://www.unicode.org/reports/tr51/tr51-29.html), which treats `FE0E` as breaking the sequence, but honoring that would require removing ZWJ joiners, violating the "only selectors change" invariant
- selectors required for the fully-qualified form are preserved or inserted
- selectors that are redundant or unsupported under that discipline are removed

ZWJ handling is fixed cleanup, not policy.

### Orphaned or unsanctioned selectors

Selectors that are not part of a sanctioned local selector-bearing context are removed.

## Slot model

The durable slot distinction is:

- standalone variation-sequence slot
- standalone keycap-character slot
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

Implementation APIs may surface these categories as findings with valid decisions. That API shape does not change the sequence-family contracts here.

## Relation to policy

| Context                           | Policy applies?                           | Behavior                                                                  |
| --------------------------------- | ----------------------------------------- | ------------------------------------------------------------------------- |
| Standalone variation sequence     | Yes, if multiple reasonable states remain | Governed by the preferred-bare and bare-as-text sets                      |
| Standalone keycap-character slot  | Yes, if multiple reasonable states remain | Governed by the preferred-bare and bare-as-text sets in the keycap domain |
| Keycap inside multi-component ZWJ | No                                        | Apply fixed fully-qualified sequence discipline                           |
| Modifier sequence defect          | No                                        | Remove legacy `FE0F` before the modifier                                  |
| ZWJ-related sequence              | No                                        | Apply fixed fully-qualified sequence discipline                           |
| Not-a-slot                        | No                                        | Remove illegal selector usage                                             |
