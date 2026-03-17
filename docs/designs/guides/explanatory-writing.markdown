# Design Note: Explanatory Document Writing

Read when: editing `README.markdown` or any similar user-facing explanatory document such as a tutorial, guide chapter, or first-use guide.

Defines: the local writing rules that are not obvious from the project principles, especially for examples, first-use material, and placement of detail in explanatory documents.

Does not define: feature semantics, repository-wide source-stability rules outside explanatory documents, or exact wording for diagnostics and code comments.

This note applies the project principles to explanatory documents with the same reader/task shape, not just to one file. Use [Project Principles](../core/project-principles.markdown) for general tradeoffs, [Documentation Source Stability](source-text-stability.markdown) for checked-in source spelling, and this note only for local corrective rules that help keep similar explanatory documents aligned with those principles.

## Reader-Facing Explanations

- In quick-start sections, including tutorials and other first-use flows, show what an example string means and what result to expect.
- Mention internal mechanisms only when they change what the reader must choose, configure, or verify.
- Historical or Unicode-background detail is worth adding only when it helps explain current formatter behavior, policy choices, or likely user confusion.

## Point-Of-Use Clarity

- At the point where the reader acts, repeat short required facts when omitting them would likely cause a wrong expectation.
- If an example uses escaped code points for source stability, explain nearby what string the reader should understand from that example.
- If two commands are meant to produce the same result, say so directly instead of forcing the reader to infer it from shell mechanics.

## Layering And Placement

- Keep the example and its nearby prose focused on the minimum information needed for the next reader action.
- Put byte-level proofs, edge cases, and deeper normalization details in later technical sections or design notes when they do not change the immediate beginner-facing takeaway.
- Keep a short warning or qualification near the example only when omitting it would create a wrong setup or misleading expectation.
- Durable feature semantics still belong in `docs/designs/features/`; explanatory documents may summarize them but should not become their defining source.

## Stable Terms

- Use `bare`, `text`, and `emoji` consistently when those distinctions affect the reader's understanding.
- When the exact code point matters to the example, name it explicitly instead of relying only on glyph shape.

## Scope Boundary

It does not define:

- durable feature semantics, which belong in `docs/designs/features/`
- non-explanatory wording such as diagnostics or code comments
- source spelling rules outside explanatory documents, which belong in [Documentation Source Stability](source-text-stability.markdown)
