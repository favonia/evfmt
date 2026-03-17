# Design Note: Formatting Model

Read when: changing rule-engine layering, policy boundaries, or canonicalization rules.

Defines: the core rule-engine model — what `evfmt` decides mechanically from Unicode data, what it decides as product policy, and how the two layers interact.

## Overview

`evfmt` is an opinionated formatter and linter for Unicode text/emoji variation selectors, specifically `U+FE0E` and `U+FE0F`.

Its job is to produce the most stable, least surprising source spelling under:

- a pinned Unicode version
- a sequence-aware parser
- a small set of explicit product assumptions
- a formatter policy over genuinely ambiguous slots

`evfmt` is a formatter first, not a general Unicode emoji validator.

## Scope

`evfmt` operates on Unicode text containing:

- standalone text/emoji variation sequences
- keycap contexts
- modifier contexts
- ZWJ-related contexts
- malformed or unsanctioned selector usage that can be repaired or removed

The formatter guarantees source-level canonicalization under its own policy. It does not guarantee identical rendering on every platform.

## Non-goals

`evfmt` is not:

- a general Unicode normalizer
- a renderer simulator
- a semantic Markdown or programming-language parser

## Three layers

### Validator and parser layer

This layer is data-driven and sequence-aware. It answers:

- what family a sequence belongs to
- whether a selector pair is sanctioned
- whether the slot is standalone, keycap-related, modifier-related, ZWJ-related, or not a slot

This layer is about Unicode-defined structure, not formatter preference.

### Slot and reasonableness layer

This layer converts parsed structure into local slots. For each slot it computes which of the three selector states are reasonable outputs:

- `none`
- `FE0E`
- `FE0F`

This is the key reduction step. Fixed-cleanup cases such as keycap, modifier-defect, and ZWJ discipline should collapse to one reasonable state before policy runs.

### Policy layer

This layer applies only when more than one reasonable state remains. The intended public policy is base-indexed and uses two predicates:

- `prefer_bare(base)`
- `treat_bare_as_text(base)`

If a slot has zero or one reasonable states, policy does not apply.

The public option surface for these predicates lives in [formatter-policy.markdown](../features/formatter-policy.markdown).

## Product assumptions

### Omitted-state policy

`evfmt` does not claim that omitted presentation is literally identical to `FE0E` or `FE0F`. Instead it adopts a weaker product assumption:

- for formatter purposes, omitted presentation is treated as either text-like or emoji-like
- if omitted rendering is stable enough to keep, it becomes a reasonable bare output
- if omitted rendering is too unstable, the formatter must emit an explicit selector instead

### Base-indexability policy

After slot classification and reasonableness filtering, genuinely ambiguous slots are expected to be indexable by base character alone. If a future Unicode version breaks this property, the design must move to richer policy keys.

## Core terminology

### Variation-sequence base

A base code point with sanctioned variation-sequence data in the pinned Unicode data set.

### Slot

A local selector-bearing context after classification. A slot is not just a base character; it includes surrounding sequence structure.

### Reasonable state

A selector state that `evfmt` accepts as a valid formatter output in a given slot.

### Canonical state

The single state that `evfmt` will emit after fixed cleanup and policy resolution.

## Canonicalization flow

### Step 1: Parse and classify

Use a sequence-aware scanner and slot classifier.

### Step 2: Compute reasonable states

For each slot, compute which of `none`, `FE0E`, and `FE0F` are reasonable formatter outputs.

### Step 3: Apply fixed rules

The following cases do not enter policy:

- keycap context canonicalizes to `[0-9#*] FE0F 20E3`
- modifier defect canonicalizes by removing legacy `FE0F` before a modifier
- ZWJ generation follows fully-qualified discipline; unsupported `FE0E`/`FE0F` are not left on ZWJ components, and required `FE0F` is inserted where needed
- unsanctioned or orphaned selectors are removed

### Step 4: Apply policy to ambiguous standalone slots

When multiple reasonable states remain, policy resolves them using `--prefer-bare-for` and `--treat-bare-as-text-for`.

### Step 5: Iterate until stable

Removing selectors can expose new structure, so formatting may require multiple passes.

The concrete slot families and per-family rules live in [sequence-handling.markdown](../features/sequence-handling.markdown).

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

### Policy only sees ambiguous slots

Keycap, modifier-defect, and ZWJ cleanup must be resolved before policy.

The evidence model for these invariants lives in [verification-strategy.markdown](../guides/verification-strategy.markdown).
