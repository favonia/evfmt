# Design Note: Project Principles

Read when: making design tradeoffs or choosing between competing implementations.

Defines: the project-wide priorities that govern all design and implementation choices. Other design notes may add domain models, contracts, and invariants, but they may not override this ordering.

The ordering is absolute: level N always wins over level N+1. Nuance lives in how each level is defined, not in how flexibly they are applied.

## 0. Maintainability Gate

Can the maintainer maintain this? This is a binary yes/no constraint. If a change pushes the project beyond the maintainer's capacity to understand and maintain, it is rejected before any other principle is considered.

## 1. Do No Harm

No memory unsafety. No uncontrolled termination (panics, segfaults, aborts). No security holes. Code should be auditing-friendly. Graceful error handling (structured errors and clean exit) is not crashing; it is correct behavior under principle 2.

## 2. Correctness

The program must produce correct output. This includes determinism, reproducibility, and thorough testing. Same input must always produce same output regardless of environment.

When evaluating product semantics, judge the design against intended user-facing behavior, not against the current implementation. Current code, tests, architecture, and data structures are not evidence that a semantic behavior is correct.

## 3. Usability

Easy adoption, clear error messages, sensible defaults. A correct but unusable tool fails its purpose. Note: noticeable performance degradation is a usability problem and is caught here, not at level 5.

## 4. Maintainability Improvements

Reasonable (not minor) structural improvements to the codebase. The "reasonable" qualifier exists to prevent trivial maintainability gains from blocking meaningful performance improvements at level 5. This level governs qualitative improvements among options that all pass the gate at level 0.

## 5. Performance

Speed and memory usage optimizations. This level only governs choices where all options are already usable (level 3). If a performance difference is noticeable to users, it is a usability concern at level 3.

## 6. Aesthetics

Personal taste as a tiebreaker, applied only when a choice cannot be justified by any of the principles above.
