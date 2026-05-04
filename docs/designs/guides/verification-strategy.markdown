# Design Note: Verification Strategy

Read when: adding or restructuring tests, or evaluating what a test actually proves.

Defines: the evidence model for correctness verification and the independence-of-evidence principle.

## Core rule

Routine correctness checks must be local, reproducible, and based on pinned repository inputs. They must not depend on network access, live upstream data, external monitoring, or other mutable state.

This rule does not mean every correctness check must be fast. It means ordinary development must have a local evidence base strong enough to protect the formatter's core claims before slower or upgrade-specific evidence is considered.

## Routine gate

The routine gate should cover the load-bearing claims of the formatter with independent local evidence:

- spec-level semantic evidence for policy resolution, selector states, and fixed-cleanup boundaries
- conformance evidence that parses pinned Unicode data and compares it with generated runtime tables
- sequence-data evidence that checks the structural assumptions used by scanner recognition and sequence-family cleanup
- behavioral evidence over representative or exhaustive selector contexts, with expected output computed independently from formatter production logic
- property-based string evidence for hard invariants such as idempotence, no remaining findings, and selector-only rewrites
- scanner evidence for losslessness and one-pass recognition of selector-repairable structure
- CLI integration evidence for the public batch-formatting contract

This guide names evidence families, not a test inventory. Formatter-level invariants belong in [formatting-model.markdown](../core/formatting-model.markdown); sequence-family contracts and scanner-recognition boundaries belong in [sequence-handling.markdown](../features/sequence-handling.markdown); pinned Unicode inputs and generated-data wiring belong in [workspace-layout.markdown](workspace-layout.markdown).

## Deeper evidence

Slower exhaustive runs, larger property campaigns, external monitoring, and expensive cross-checks may live outside the routine gate. They are useful for increasing confidence, finding missed cases, and preparing upgrades.

They cannot be the only evidence for core correctness. If an exhaustive or randomized check becomes too expensive for routine use, the routine gate must retain smaller local evidence that protects the same load-bearing claim, while the larger run moves to a deeper tier.

## Unicode upgrade gate

Unicode upgrade checks are upgrade evidence, not a substitute for local routine checks. Repository-local copies of Unicode data remain the primary correctness inputs for routine verification; upstream checks exist to reveal new versions, errata, and changed assumptions during an upgrade.

The upgrade gate should include:

- machine diffs of parsed data and generated tables
- machine diffs of upstream prose anchors that support product assumptions
- derived invariants that protect formatter and sequence-family assumptions
- human review when prose anchors move or product assumptions become unclear

Pinned data files, generated tables, and broader sequence inputs are described in [workspace-layout.markdown](workspace-layout.markdown). The upgrade decision model is:

- green: prose anchors unchanged and invariants pass
- yellow: invariants pass but prose anchors moved, requiring human review
- red: invariants fail

## Independence rule

Each test should state or imply its oracle: what claim it proves, where the expected result comes from, and which assumptions it intentionally shares or avoids.

Independent evidence is strongest when the test's expected output is derived from a different representation than the production path. Examples include parsing pinned Unicode files instead of trusting generated tables, using a decision table instead of calling formatter helpers to compute expected output, or checking scanner reconstruction directly from raw item slices.

Avoid test layers that silently share the same hidden assumption. When shared assumptions are necessary, keep them narrow and explicit near the owning test helper or design note.
