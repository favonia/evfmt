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

The scanner must preserve enough recognized structure for item analysis to apply the contracts below in one formatting pass:

- Losslessness: every input byte belongs to exactly one scan item, and concatenating scan item source slices reconstructs the original input.
- Selector-only idempotence: inserting, removing, or replacing `FE0E`/`FE0F` must not reveal newly recognized emoji-related structure on a second pass.
- Selector locality: `FE0E` and `FE0F` attach to the nearest scanner-recognized context that can own them; selectors no such context can own are preserved as unsanctioned selector runs for cleanup.
- Emoji-like permissiveness: scanner grouping is based on potentially emoji-relevant structure, not on RGI status or final semantic validity.
- ZWJ visibility: valid ZWJ sequences and malformed ZWJ-related structures made only of recognized emoji/selector/ZWJ material stay visible to item analysis so selector cleanup can preserve non-selector text.

Concrete scanner state shapes and local edge cases belong in scanner comments and scanner/analysis tests. This note records the cross-module contract those comments implement.

## Core decision boundary

Policy is only for genuinely ambiguous selector contexts that remain after sequence-specific cleanup and collapse to a policy position.

Sequence-specific cleanup rules must resolve before policy.

## Sequence-family contracts

### Variation-sequence contexts

For a base with variation-sequence data, the formatter may need policy to choose among bare, `FE0E`, and `FE0F`.

After context-aware cleanup, the only policy-ambiguous contexts are ordinary variation-sequence contexts and keycap-character contexts.

### Fixed cleanup for singleton-base contexts

When the first modification after a singleton base is an emoji modifier or a tag specification, selector handling is fixed cleanup rather than policy. A singleton base whose first modification is `U+20E3` uses keycap-character policy when the base has variation-sequence data.

Only the first `EmojiModification` after the base determines this base-presentation context. Later modifier, keycap, or tag material is preserved in source order and its own trailing presentation selectors are still cleaned, but it does not move the base context into a different policy domain or fixed-cleanup family.

The base-presentation decision is resolved by this precedence:

| Precedence | Context                                                  | Canonical base presentation |
| ---------- | -------------------------------------------------------- | --------------------------- |
| 1          | The chosen presentation would be unsanctioned            | bare                        |
| 2          | First modification is an emoji modifier after `FE0E`     | text                        |
| 3          | First modification is an emoji modifier sequence         | bare                        |
| 4          | First modification is a tag, emoji-default base          | bare                        |
| 5          | First modification is a tag, other base                  | emoji                       |
| 6          | No fixed-cleanup context remains                         | use policy                  |

Rule 6 is the boundary back to policy. Keycap-character contexts query the keycap-character domain, and ordinary contexts query the ordinary domain.

Every `[0-9#*]` keycap base has standardized text and emoji presentation sequences and is text-default in the pinned Unicode data. Scanner grouping is broader than RGI keycaps, so policy queries outside that standardized base set may still collapse to selector cleanup when the base lacks variation-sequence data.

### Keycap sequences

In true keycap context, the RGI emoji keycap form (UTS #51 §1.4.5) is:

```text
[0-9#*] FE0F 20E3
```

Bare keycap inputs (`[0-9#*] 20E3`) use keycap-character policy. The default policy treats bare keycap-character forms as text, so the default canonical spelling is:

```text
[0-9#*] FE0E 20E3
```

Explicit `FE0E` in the same keycap-character context is preserved. `[0-9#*] FE0E` is a standardized text variation sequence (`StandardizedVariants.txt`), and per the Unicode Standard Chapter 23 a variation selector's effect on the base may propagate to subsequent combining marks. `[0-9#*] FE0E 20E3` therefore has a well-defined compositional meaning: a text-styled digit enclosed by a keycap mark inheriting that appearance.

Explicit `FE0F` in a keycap-character context is preserved under the default policy.

### Modifier sequences

Legacy `FE0F` immediately before an emoji modifier is non-canonical and must be removed. UTS #51 treats this as old defective emoji modifier sequence data where the emoji presentation selector is ignored.

Modifier handling is fixed cleanup, not policy.

### ZWJ-related sequences

ZWJ-related selector handling is component-local after structural recognition.

Under the structural recognition contract above, recognized ZWJ-related structure is preserved before cleanup. The analysis/formatting layer then decides which selectors are valid, repairable, or removable under the formatter invariants.

For cleanup, the relevant ZWJ-like shapes are:

- no emoji component: only presentation selectors attached to ZWJ links are removed
- one emoji component: the component uses the ordinary singleton or flag cleanup path, while surrounding ZWJ links are preserved and link-attached selectors are removed
- multiple emoji components: each component uses the same ordinary/keycap policy and fixed cleanup it would use without the surrounding ZWJ links

In canonical output:

- `FE0E` on a ZWJ component is honored as that component's text-presentation request; per [UTS #51](https://www.unicode.org/reports/tr51/tr51-29.html), this may break the combined emoji image in renderers
- bare component contexts are resolved by the active formatter policy
- selectors attached to ZWJ links are removed
- selectors that are redundant or unsupported under component-local cleanup are removed

ZWJ-link cleanup is fixed; component contexts use the ordinary policy boundary above.

### Orphaned or unsanctioned selectors

Selectors that are not part of a sanctioned local selector-bearing context are removed.

## Selector context model

The durable context distinction is:

- ordinary variation-sequence context
- keycap-character context
- fixed-cleanup sequence context
- not a selector context

Each real selector context records:

- current variation selector state
- whether extra variation selectors were present
- which of `none`, `FE0E`, and `FE0F` remain reasonable after context-aware reduction

## Non-Canonicality Model

The important split is:

- unsanctioned or structurally broken selector usage
- fixed-cleanup sequence defects
- deterministic insertion of a required presentation selector
- variation-selector state mismatches that are already non-canonical under the current policy
- genuinely ambiguous contexts that need policy

A redundant variation selector belongs to the third class, not the first. It is still sanctioned Unicode structure; it is simply non-canonical under the active formatter policy because the same context canonically stays bare.

A missing required selector is separate from a sequence defect. For example, when tag context deterministically requires emoji presentation, inserting `FE0F` is fixed cleanup but is not a UTS #51 defective emoji modifier sequence.

This split matters because only the last class is a real policy choice. The other classes already have a canonical repair.

Implementation APIs may surface these categories as findings with valid decisions. That API shape does not change the sequence-family contracts here.

## Relation to policy

| Context family            | Policy applies?                           | Behavior                                                                  |
| ------------------------- | ----------------------------------------- | ------------------------------------------------------------------------- |
| Ordinary context          | Yes, if multiple reasonable states remain | Governed by the preferred-bare and bare-as-text sets                      |
| Keycap-character context  | Yes, if multiple reasonable states remain | Governed by the preferred-bare and bare-as-text sets in the keycap domain |
| Modifier sequence defect  | No                                        | Remove legacy `FE0F` before the modifier                                  |
| ZWJ link selectors        | No                                        | Remove selectors attached to ZWJ links                                    |
| Not a selector context    | No                                        | Remove illegal selector usage                                             |
