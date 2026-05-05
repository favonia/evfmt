# Rationale Archive

This directory holds non-normative rationale for durable design decisions.

Use these files when auditing, challenging, or replacing a design decision. They are for lower-frequency audit material: evidence, assumptions, tradeoffs, rejected alternatives, weak points, and revisit triggers. They do not define current behavior; normative contracts and frequently used operating guidance live in `docs/designs/`, code, tests, and public API documentation.

Rationale is upstream of durable design changes, not superior to current design contracts. When changing a durable design decision, preserve, revise, or retire the relevant rationale instead of bypassing it.

Rationale may include guidance when it supports a design decision, but it must not be the only home for a binding rule or guidance that editors, implementers, or test authors need routinely.

## Shared Guidance

- [`authoring.markdown`](authoring.markdown) — shared authoring, manual review, and prompt protocols for rationale files

## Entries

- [`formatting-model.markdown`](formatting-model.markdown) — rationale for the core selector-only formatting boundary, policy boundary, bare display assumptions, and policy-position shape
- [`sequence-handling.markdown`](sequence-handling.markdown) — rationale for sequence-family policy boundaries, keycap preservation, modifier cleanup, ZWJ cleanup, and related rejected alternatives
- [`source-text-stability.markdown`](source-text-stability.markdown) — rationale for checked-in text source stability and reader/source separation
