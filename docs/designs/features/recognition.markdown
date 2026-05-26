# Design Note: Recognition

Read when: changing scanner invariants, recognition boundaries, or what structure must remain visible to classification.

Defines: the high-level scanner contract for recognizing structure before selector-slot classification.

Does not define: selector-slot classification rules. Those live in [classification.markdown](classification.markdown). Concrete scanner state shapes and local edge cases belong in scanner comments and scanner tests.

## Contract

Recognition partitions input into source-preserving structural units for the formatting pipeline. The representation may expose more structure than current classification rules need, but it must satisfy these invariants:

- Losslessness: every input byte belongs to exactly one scan item, and concatenating scan item source slices reconstructs the original input.
- Selector coverage: every `FE0E` and `FE0F` is exposed to classification.
- Selector-only idempotence: inserting, removing, or replacing `FE0E`/`FE0F` must not reveal newly recognized emoji-related structure on a second pass.
- Emoji-like permissiveness: grouping is based on valid emoji skeletons, meaning the non-`FE0E`/`FE0F` structure shared with valid emoji sequences, not on RGI status or byte-exact emoji qualification.
- ZWJ visibility: valid ZWJ sequences and malformed ZWJ-related structures made only of recognized emoji, selector, and ZWJ material stay visible to classification.

Recognition does not decide canonical selector states. It preserves enough local structure for classification while leaving non-selector text available for unchanged output.
