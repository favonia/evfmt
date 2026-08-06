# Design Note: Formatting

Read when: changing the abstract formatting model, policy boundary, or cross-feature canonicalization invariants.

Defines: selector slots, reasonable states, fixed cleanup, policy resolution, and hard invariants.

Does not define: how recognized structure classifies a slot into reasonable states. Those rules live in [classification.markdown](../features/classification.markdown). Policy predicates and defaults live in [policy.markdown](../features/policy.markdown).

## Overview

`evfmt` formats Unicode text by canonicalizing text/emoji variation selectors, specifically `U+FE0E` and `U+FE0F`.

The abstract formatting model is:

1. recognize structure
2. expose selector slots
3. assign each slot a nonempty set of reasonable states
4. choose one canonical state for each slot
5. emit the source text with only `FE0E` and `FE0F` changed

This is a specification of observable behavior, not an implementation plan. Implementations may use different scanners, parsers, or repair passes as long as their output satisfies this model and the feature contracts in the design notes.

## Selector Slots

A selector slot is exactly one of:

- the span before the first non-selector scalar
- the span between two adjacent non-selector scalars
- the span after the last non-selector scalar

A slot state is one of:

- `none`: bare form
- `FE0E`: text form
- `FE0F`: emoji form

Each selector slot has at least one reasonable state.

## Canonicalization

### Fixed cleanup

If a slot has exactly one reasonable state, that state is canonical.

### Policy resolution

If a slot has more than one reasonable state and policy applies, policy chooses the canonical state.

Policy does not create additional reasonable states. It chooses among the states already accepted by classification.

### Default exact-RGI preservation

Default formatting preserves recognized exact RGI emoji sequences byte for byte.

This constrains both classification and default policy. The selector state induced by an exact RGI spelling must be reasonable for every selector slot in that context, and default policy must not choose a text-presentation result for that exact RGI input.

### Default RGI preference

If the canonical form chosen by the default policy has emoji presentation and an RGI spelling is one of the reasonable states, the chosen canonical form must be the RGI spelling.

This preference does not choose emoji presentation over text presentation. Explicit non-default policy may choose another reasonable selector state.

## Hard Invariants

### Idempotence

For any fixed Unicode version and fixed option values:

```text
format(format(x)) = format(x)
```

### Determinism

For the same input, same Unicode version, and same option values, output is identical.

### Only selectors change

Formatting only inserts, removes, or replaces `FE0E` and `FE0F`.

The evidence model for these invariants lives in [verification-strategy.markdown](../guides/verification-strategy.markdown).
