# Design Note: Verification Strategy

Read when: adding or restructuring tests, or evaluating what a test actually proves.

Defines: the evidence model for correctness verification and the independence-of-evidence principle.

The guiding rule is independence of evidence: each test layer should prove a different claim, and no two layers should share hidden assumptions.

## Verification tiers

Not all evidence belongs in the same execution tier.

Core correctness evidence must be runnable locally from pinned repository inputs, without depending on network access or mutable upstream state.

Fast verification that depends only on those pinned local inputs belongs in the routine verification gate. Slower runs, external monitoring, and upgrade-specific evidence belong in deeper verification tiers, but only after the routine gate already covers the core correctness claims needed for ordinary development.

## Evidence layers

### 1. Spec-level semantic evidence

A data-driven decision table where each row makes the semantic axes explicit:

- slot kind
- sanctioned selector set
- no selector / `FE0E` / `FE0F`
- preferred-bare matches slot true/false
- bare-as-text matches slot true/false

### 2. Unicode-data conformance evidence

An independent test parses the pinned Unicode source files committed to the repository and compares them against the generated runtime tables. This verifies:

- the variation-sequence base set
- text and emoji variation-sequence membership
- Unicode default presentation side

### 3. Sequence-data conformance evidence

Independent tests parse `emoji-sequences.txt`, `emoji-zwj-sequences.txt`, `emoji-test.txt`, and related pinned repository inputs to verify the structural assumptions the formatter depends on:

- keycap bases are exactly `#`, `*`, `0`-`9`
- standalone keycap cleanup and multi-component ZWJ keycap cleanup remain distinct
- modifier-sequence cleanup remains compatible with pinned Unicode data
- ZWJ-related rules remain compatible with fully-qualified generation discipline
- non-emoji or otherwise unsupported ZWJ-component selectors are removed
- variation-selector bases are single code points where the data model requires that property
- any `emoji-test.txt` usage remains a qualification and family cross-check, not the sole source of formatter truth

### 4. Exhaustive behavioral evidence

Every variation-sequence entry is exercised under all relevant policy combinations and selector inputs. The expected output is computed independently from the formatter implementation. This evidence should remain local and reproducible. If the full exhaustive suite becomes too costly for the routine gate, that gate must still retain enough local evidence to protect the same load-bearing correctness claims, while the full exhaustive run can move to a deeper verification tier.

### 5. Property-based string evidence

Randomized tests verify:

- idempotence
- no illegal selectors in output
- no unresolved disallowed bare forms in output
- only selectors change

Use a two-tier budget when needed: a quick randomized smoke run for every PR/push, and deeper or longer-running campaigns on a schedule.

### 6. Scanner and slot invariants

- losslessness: `reconstruct(scan(input)) == input`
- idempotent recognition: selector-only repairs must not reveal newly recognized emoji-related structure that needs a second formatting pass
- cluster coherence for recognized emoji-related structure, including the `evfmt` broadening needed to keep valid flags and keycaps inside ZWJ-related scan items
- recognized leading or malformed ZWJ-related clusters remain visible to findings analysis instead of disappearing into passthrough
- scanner and formatter agreement on singleton inputs
- keycap slot invariant: standalone keycaps follow keycap sequence rules, while keycap components inside multi-component ZWJ sequences follow ZWJ forced-emoji cleanup
- modifier-defect invariant: modifier defect leaves exactly one reasonable state, `none`
- standalone keycap-base invariant: as standalone variation-sequence bases, `#`, `*`, and digits may retain three reasonable states

### 7. Derived invariants for Unicode upgrades

These guard the product assumptions:

- modifier context must not re-enter policy ambiguity
- ZWJ terminal handling must not re-enter policy ambiguity
- keycap cleanup must not re-enter policy ambiguity; standalone keycaps and ZWJ-component keycaps keep distinct fixed-cleanup behavior
- after fixed rules, ambiguous policy slots must still collapse to base-indexed policy keys

### 8. CLI contract evidence

Integration tests cover:

- format rewrite success
- already-canonical no-op success
- `evfmt check` exit codes
- stdin and stdout via `-`
- ordered `set/add/remove` CLI behavior for policy and ignore filters
- invalid UTF-8 exits `2`
- partial failure exits `2` without rollback

## Unicode upgrade gate

Unicode upgrades are not validated only by data-file diffs. Repository-local copies of the relevant Unicode inputs are the primary evidence base; upstream change detection exists to reveal new versions or errata, not to substitute for local correctness checks. The full upgrade gate is:

- machine diff of parsed data and generated tables
- machine diff of prose anchors that support product assumptions
- derived invariants
- human review when prose anchors move or assumptions become less obvious

The minimum upgrade input set is:

- `emoji-data.txt`
- `emoji-variation-sequences.txt`
- `emoji-sequences.txt`
- `emoji-zwj-sequences.txt`
- `emoji-test.txt`
- the upstream prose sources that justify product assumptions and sequence-handling rules

The prose-anchor diff should explicitly watch the normative or explanatory passages that support:

- omitted/default-presentation framing
- the limits of `possible_emoji`
- modifier handling around legacy `FE0F`
- ZWJ generation discipline and `FE0E` breakage
- the absence of sanctioned `FE0E`/`FE0F` on non-emoji ZWJ components
- `emoji-variation-sequences.txt` as the exact sanctioned EVS list
- keycap interpretation for `#`, `*`, and digits before `U+20E3`

The intended decision model is:

- green: prose anchors unchanged and invariants pass
- yellow: invariants pass but prose anchors moved, requiring human review
- red: invariants fail
