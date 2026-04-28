# Design Note: Formatting Model

Read when: changing rule-engine layering, policy boundaries, or canonicalization rules.

Defines: the core rule-engine model — what `evfmt` decides mechanically from Unicode data, what it decides as product policy, and how the two layers interact.

## Overview

`evfmt` is an opinionated formatter and linter for Unicode text/emoji variation selectors, specifically `U+FE0E` and `U+FE0F`.

Its job is to produce the most stable, least surprising source spelling under:

- a pinned Unicode version
- a sequence-aware parser
- a small set of explicit product assumptions
- a formatter policy over genuinely ambiguous selector contexts

`evfmt` is a formatter first, not a general Unicode emoji validator.

This document specifies the final formatting model. The sections below define principles, layers, and invariants from which the expected output follows. They are not an implementation plan: implementations may use different internal state machines, scanners, or repair passes as long as the observable output satisfies this model. Code comments may describe implementation details, but those details do not define this specification.

## Scope

`evfmt` operates on Unicode text containing:

- standalone text/emoji variation sequences
- keycap contexts
- modifier contexts
- ZWJ-related contexts
- malformed or unsanctioned selector usage that can be repaired or removed

The formatter guarantees source-level canonicalization under its own policy to improve consistency across current and future platforms. It does not guarantee identical rendering on every platform.

## Non-goals

`evfmt` is not:

- a general Unicode normalizer
- a renderer simulator
- a semantic Markdown or programming-language parser

## Model Layers

### Validator and parser layer

This layer is data-driven and sequence-aware. It identifies:

- what family a sequence belongs to
- whether a selector pair is sanctioned
- which selector-bearing context, if any, owns the selector state

This layer is about Unicode-defined structure, not formatter preference.

The layer must be permissive enough to preserve malformed selector and ZWJ-related structure for later diagnosis. Recognition does not imply validity.

### Context and reasonableness layer

This layer converts parsed structure into local selector contexts. For each context it computes which of the three selector states are reasonable outputs:

- `none`
- `FE0E`
- `FE0F`

This is the key reduction step. Fixed-cleanup cases such as modifier defects, required deterministic selector insertion, ZWJ-link selector cleanup, and unsanctioned selectors must resolve before policy runs. Ordinary and keycap-character contexts can remain ambiguous and enter policy through their matching policy positions.

### Policy layer

This layer applies only when more than one reasonable state remains. The intended public policy is base-indexed with an ordinary/keycap domain qualifier and uses two `VariationSet` membership predicates:

- `prefer_bare(position)`
- `bare_as_text(position)`

If a context has zero or one reasonable states, policy does not apply.

Policy positions are divided into ordinary non-keycap positions and keycap-character positions. Both are indexed by variation-sequence base character, but policy membership is queried in the domain that matches the context.

The public option surface for these predicates lives in [formatter-policy.markdown](../features/formatter-policy.markdown).

## Product assumptions

Unicode standards and pinned Unicode data define Unicode facts for this project; they do not by themselves establish how real renderers, keyboards, editors, terminals, fonts, or users behave. Questions about user expectation or implementation behavior need platform/user evidence. Check real implementations when feasible; otherwise state that evidence is missing.

### Omitted-state policy

`evfmt` does not claim that omitted presentation is literally identical to `FE0E` or `FE0F`. Instead it adopts a weaker product assumption:

- for formatter purposes, omitted presentation is treated as either text-like or emoji-like
- if omitted rendering is stable enough to keep, it becomes a reasonable bare output
- if omitted rendering is too unstable, the formatter must emit an explicit selector instead

### Domain-qualified base-indexability policy

After context classification and reasonableness filtering, genuinely ambiguous contexts are expected to collapse to a policy position: a base character plus an ordinary/keycap domain. If a future Unicode version breaks this property, the design must move to richer policy keys.

## Core terminology

### Variation-sequence base

A base code point with sanctioned variation-sequence data in the pinned Unicode data set.

### Selector context

A local selector-bearing context after classification. A selector context is not just a base character; it includes the surrounding sequence structure needed to decide which selector states are sanctioned, reasonable, redundant, or defective.

### Policy position

The key used by formatter policy after a genuinely ambiguous selector context has survived fixed cleanup. A policy position is a variation-sequence base character plus an ordinary/keycap domain. `VariationSet` values contain policy positions, not arbitrary selector contexts.

### Reasonable state

A selector state that `evfmt` accepts as a valid formatter output in a given selector context.

### Canonical state

The single state that `evfmt` will emit after fixed cleanup and policy resolution.

## Canonicalization Model

### Structural recognition

Use sequence-aware recognition to classify selector-bearing contexts and nearby emoji-related structure. Scanner boundaries are an implementation concern, but the recognized structure must preserve the distinctions required by [sequence-handling.markdown](../features/sequence-handling.markdown).

### Reasonable states

For each selector context, compute which of `none`, `FE0E`, and `FE0F` are reasonable formatter outputs.

### Fixed rules

The following cases do not enter policy:

- modifier cleanup removes legacy defective `FE0F` before a modifier, while preserving sanctioned `FE0E` as text presentation on the base
- keycap-character handling uses policy when the base has variation-sequence data
- ZWJ links are preserved, selectors attached to ZWJ links are removed, and each component is resolved as if the surrounding ZWJ links were absent
- unsanctioned or orphaned selectors are removed

### Policy resolution

When multiple reasonable states remain and the context collapses to a policy position, policy resolves it using the preferred-bare set and the bare-as-text set.

### Canonical replacements

Return the text produced by applying the fixed-rule repairs and policy resolutions above. The returned text is the canonical result under this model.

The concrete context families and per-family rules live in [sequence-handling.markdown](../features/sequence-handling.markdown).

## Hard invariants

### Idempotence

For any fixed Unicode version and fixed option values:

```text
format(format(x)) = format(x)
```

### Determinism

For the same input, same Unicode version, and same option values, output is identical.

### Only selectors change

Formatting only inserts, removes, or replaces `FE0E` and `FE0F`.

### Policy only sees ambiguous contexts

Modifier defects, ZWJ-link selector cleanup, and unsanctioned selector cleanup must be resolved before policy. Keycap-character contexts that still have multiple reasonable selector states use keycap-character policy positions.

The evidence model for these invariants lives in [verification-strategy.markdown](../guides/verification-strategy.markdown).
