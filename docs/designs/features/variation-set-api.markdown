# Design Note: VariationSet API

Read when: changing the typed `evfmt::variation_set` API, adding named sets, or modifying set-combinator semantics.

Defines: the typed variation-set model used by `evfmt::variation_set` and `evfmt::Policy`. The model describes finite sets of sanctioned formatter variation positions.

Does not define: the CLI list grammar. The CLI uses ordered `set/add/remove` flags with comma-separated list items; see [formatter-policy.markdown](formatter-policy.markdown) for that surface.

## Public Surface

The public typed surface is built from:

- `VariationSet::all()`
- `VariationSet::none()`
- `VariationSet::singleton(c)`
- `VariationSet::singleton_keycap(c)`
- `set.contains(c)`
- `set.contains_keycap(c)`
- `variation_set::ASCII`
- `variation_set::TEXT_DEFAULTS`
- `variation_set::EMOJI_DEFAULTS`
- `variation_set::RIGHTS_MARKS`
- `variation_set::ARROWS`
- `variation_set::CARD_SUITS`
- `variation_set::KEYCAP_CHARS`
- `variation_set::NON_KEYCAP_CHARS`
- `variation_set::KEYCAP_EMOJIS`
- `variation_set::is_variation_sequence_character(c)`
- `!set`
- `set | other`
- `set & other`
- `set ^ other`
- `set - other`

## Domains

Every `VariationSet` has two domains, both indexed by the same pinned `emoji-variation-sequences.txt` base-character table:

- ordinary non-keycap positions, queried with `contains(c)`
- keycap-character positions, queried with `contains_keycap(c)`, where the base is followed by `U+20E3 COMBINING ENCLOSING KEYCAP`

Characters outside the variation-sequence base table are never members of either domain.

The internal bitset type is private. Public code should treat `VariationSet` as an opaque value with constructors, queries, and set operators.

## Atoms

| Constructor                         | Meaning                                                                            |
| ----------------------------------- | ---------------------------------------------------------------------------------- |
| `VariationSet::all()`               | Every ordinary and keycap-character position                                       |
| `VariationSet::none()`              | No position                                                                        |
| `VariationSet::singleton(c)`        | One ordinary position, or empty if `c` is outside the policy universe              |
| `VariationSet::singleton_keycap(c)` | One keycap-character position, or empty if `c` is outside the policy universe      |
| `variation_set::ASCII`              | ASCII variation-sequence bases in ordinary positions                               |
| `variation_set::TEXT_DEFAULTS`      | Text-default variation-sequence bases in ordinary positions                        |
| `variation_set::EMOJI_DEFAULTS`     | Emoji-default variation-sequence bases in ordinary positions                       |
| `variation_set::RIGHTS_MARKS`       | The rights marks currently listed in Unicode's `emoji-variation-sequences.txt`     |
| `variation_set::ARROWS`             | The arrow characters currently listed in Unicode's `emoji-variation-sequences.txt` |
| `variation_set::CARD_SUITS`         | The card suits currently listed in Unicode's `emoji-variation-sequences.txt`       |
| `variation_set::KEYCAP_CHARS`       | Every keycap-character position for a variation-sequence base                      |
| `variation_set::NON_KEYCAP_CHARS`   | Every ordinary non-keycap variation-sequence base position                         |
| `variation_set::KEYCAP_EMOJIS`      | RGI emoji keycap bases (`#`, `*`, `0`-`9`) in keycap-character positions           |

Semantic named sets such as `ASCII`, `RIGHTS_MARKS`, `ARROWS`, and `CARD_SUITS` affect ordinary positions only. Keycap-specific membership is expressed explicitly with `KEYCAP_CHARS`, `KEYCAP_EMOJIS`, or `VariationSet::singleton_keycap(c)`.

`VariationSet::all()` is exactly:

```rust
variation_set::KEYCAP_CHARS | variation_set::NON_KEYCAP_CHARS
```

## Combinators

| Constructor    | Meaning                                         |
| -------------- | ----------------------------------------------- |
| `!set`         | Positions in the universe that are not in `set` |
| `set \| other` | Positions matched by either set                 |
| `set & other`  | Positions matched by both sets                  |
| `set ^ other`  | Positions matched by exactly one set            |
| `set - other`  | Positions in `set` but not in `other`           |

Operators apply componentwise to ordinary and keycap-character domains. The assignment operators `|=`, `&=`, `^=`, and `-=` have the corresponding in-place meanings.

## Display

`Display` renders fully empty and fully full sets as `none` and `all`. Other sets render members in variation-table order, separated by commas.

Examples:

- `VariationSet::singleton('#')` renders as `u(0023)`

## Queries

| Query                                               | Meaning                                                 |
| --------------------------------------------------- | ------------------------------------------------------- |
| `variation_set::is_variation_sequence_character(c)` | Whether `c` is inside the eligible base-character table |
| `set.contains(c)`                                   | Whether `c` is in the ordinary domain                   |
| `set.contains_keycap(c)`                            | Whether `c` is in the keycap-character domain           |

## Examples

```rust
use evfmt::variation_set;

let prefer_bare = variation_set::ASCII | variation_set::RIGHTS_MARKS;
let treat_bare_as_text = variation_set::ASCII
    | variation_set::RIGHTS_MARKS
    | variation_set::KEYCAP_CHARS;
```
