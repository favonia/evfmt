# Design Note: Documentation Source Stability

Read when: editing any repository text that may contain dual-presentation Unicode characters, including `README.markdown`, design notes, code comments, tests, operator messages, and other checked-in prose.

Defines: how repository text should balance clear rendered meaning with `evfmt`-stable source bytes, and how that choice follows from [Project Principles](../core/project-principles.markdown).

Does not define: feature semantics, README-specific writing rules, local message wording, or a general writing style guide.

This note applies the project principles to repository text:

- `Correctness`: checked-in source should remain stable under the formatter instead of relying on platform-dependent rendering or accidental byte choices.
- `Usability`: readers should see the explanation that best helps them act correctly, even when the source spelling uses explicit selectors or escaped code points.
- `Maintainability Improvements`: editors should have a predictable rule for when to prefer raw glyphs, explicit selectors, or escaped code points.

## Core Rule

Write for the meaning that humans should read, but encode the source so it stays stable under `evfmt`.

These are separate concerns:

- rendered text should explain the intended meaning clearly
- raw source bytes should avoid unstable dual-presentation spellings
- the best rendered explanation does not need to mirror the literal source spelling exactly

## Scope

This rule applies to checked-in repository text, not only the README:

- documentation such as `README.markdown` and design notes
- source comments and doc comments
- tests, especially string literals used as documentation-by-example
- user-facing operator messages and diagnostics

The exact wording still belongs to the local document or message context. This note only defines the source-stability constraint and the resulting tradeoff.

## Editing Guidance

Start with the explanation that is clearest for the reader in that local context. Then make the source stable.

- Prefer natural prose first.
- If a dual-presentation character would be unstable in source, add an explicit selector.
- If the literal glyph would still be unclear or too fragile in source, use an explicit code point such as `U+00A9` or `\u{00A9}` where that notation fits the local context better.
- When the distinction matters to the meaning, name the code point explicitly instead of relying on readers to infer it from glyph shape.

## Placement And Layering

Do not force every local contract surface to carry the full technical rationale.

- At the point where the reader acts, prefer the shortest wording that preserves the local contract.
- Put deeper technical precision in nearby prose or in the relevant design note when that precision does not change the immediate local contract.
- Keep tables and short reference lists focused on the contract they define.
- If a technical detail explains *why* the contract exists but not *what* the local contract is, describe it outside the table or list.

This is usually the right split:

- local contract surface: user-visible rule
- nearby prose or design note: technical explanation and rationale

## Examples

- Write about bare `U+00A9` even if the source uses an explicit selector on the rendered glyph or uses `U+00A9` notation.
- Prefer `'\u{00A9}'` over a raw `©︎` literal in a test when that makes the source more stable or easier to audit.
- Say "selectors do not matter here" in a README when the exact code points do not change the reader's decision.

## Non-Goals

This note does not require maximum escaping everywhere.

- Use escaped code points when they improve source stability or precision.
- Keep raw glyphs when they are already stable enough and materially clearer.
- Let the local document decide how much technical detail to expose, subject to the layering rule above.
