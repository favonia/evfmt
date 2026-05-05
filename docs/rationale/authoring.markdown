# Rationale Authoring

Read when: creating or editing rationale files in this directory.

Defines: shared authoring conventions for non-normative rationale records.

Does not define: formatter behavior, public API behavior, or design contracts. Normative contracts live in `docs/designs/`, code, tests, and public API documentation.

## Rationale Scope

Rationale files record why a durable design decision currently seems defensible. They may include evidence, assumptions, tradeoffs, rejected alternatives, and revisit reasons. They are useful when auditing, challenging, or replacing a design decision, but they are not behavior contracts.

Rationale prose should distinguish facts, project-principle inferences, product assumptions, tests, implementation notes, evidence gaps, and revisit triggers when the distinction improves precision, accuracy, clarity, or understandability.

These categories are vocabulary, not a template. Do not add tags, fixed sections, or boilerplate just to show that every category was considered. Prefer readable prose that makes the argument easy to inspect.

## Manual Review

Argumentative rationale units must have one of these exact markers immediately after the relevant subsection heading:

- `Manually reviewed: no.`
- `Manually reviewed: yes.`

`Manually reviewed: yes.` means a human maintainer has reviewed the argument as written and accepts both its meaning and its presence in the rationale record. It does not mean that every external fact has been independently reverified unless the argument says so.

`Manually reviewed: no.` means the argument still needs human review before it should be treated as accepted project rationale.

AI edits must reset the marker to `Manually reviewed: no.` whenever the argument's meaning may have changed. When in doubt, reset it. Formatting-only edits and literal copy or move operations may preserve `Manually reviewed: yes.` when the argument meaning is clearly unchanged.

The marker scope should be clear in ordinary Markdown reading. Do not make one marked unit carry unrelated claims just to avoid another marker; split the prose when separate claims need separate review status.

## Prompt Protocol

Human authors may use `(PROMPT: ...)` as temporary AI-facing drafting context while working on a rationale file.

`(PROMPT: ...)` is not rationale content, not an unresolved objection marker, and not a final-document annotation. Before committing or finalizing a rationale document, every `(PROMPT: ...)` must be removed, either by integrating the intended correction into ordinary prose or by deciding no prose change is needed.

Do not introduce alternative temporary markers such as `Favonia:`, `Reviewer note:`, `AI TODO`, or `AI instruction:` in rationale files.
