# Design Documents

This directory holds durable design notes for future developers, including AI agents.

Use this file as a retrieval map. Each design note has "Read when:" and "Defines:" headers so you can decide whether to read it without opening the file.

`core/project-principles.markdown` is the final authority for design decisions. Other notes may refine detail but may not override it.

## Always Read

- [`core/project-principles.markdown`](core/project-principles.markdown) — project-wide priorities for design tradeoffs
- [`core/formatting-model.markdown`](core/formatting-model.markdown) — rule-engine layering, policy boundaries, canonicalization

## Read When Needed

- [`guides/workspace-layout.markdown`](guides/workspace-layout.markdown) — crate structure, module placement, build pipeline
- [`guides/verification-strategy.markdown`](guides/verification-strategy.markdown) — durable verification tiers, evidence layers, what a test proves
- [`guides/explanatory-writing.markdown`](guides/explanatory-writing.markdown) — explanatory-document examples, beginner-facing explanations, and local detail placement
- [`guides/source-text-stability.markdown`](guides/source-text-stability.markdown) — writing checked-in text clearly while keeping source bytes evfmt-stable

### Feature models

- [`features/formatter-policy.markdown`](features/formatter-policy.markdown) — policy defaults, warning semantics, exit codes
- [`features/variation-set-api.markdown`](features/variation-set-api.markdown) — typed policy variation-set API, named sets, and set operations
- [`features/sequence-handling.markdown`](features/sequence-handling.markdown) — durable sequence-family contracts and policy boundaries

## Rationale Archive

Rationale files are non-normative records of evidence, tradeoffs, rejected alternatives, and revisit triggers. Read them when auditing, challenging, or replacing a design decision; do not treat them as behavior contracts.

- [`../rationale/README.markdown`](../rationale/README.markdown) — rationale archive index
- [`../rationale/sequence-handling.markdown`](../rationale/sequence-handling.markdown) — sequence-handling policy boundary and tradeoff rationale

## Directory Scope

Use `docs/designs/` only for durable design information that is broader than one local edit.

- `core/` for project-wide principles and architecture
- `features/` for durable feature contracts, invariants, and scope boundaries
- `guides/` for shared editing or verification rules reused across unrelated features
- choose the narrowest durable home for a rule: keep durable cross-component constraints here, and keep local implementation detail, repository operations, and tool wiring in code comments, tests, help text, or the files that implement them
- update an existing note before adding a new one
- add a new note only when no existing note can own the information cleanly and the information is durable, cross-file, and likely to matter again
- keep temporary rollout notes, branch-local rationale, review notes, and one-file heuristics out of `docs/designs/`
