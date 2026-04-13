# Design Note: Charset API

Read when: changing the typed `evfmt::charset` API, adding named sets, or modifying set-combinator semantics.

Defines: the typed charset model used by `evfmt::charset` and `evfmt::Policy`. The model describes finite sets of dual-presentation characters.

Does not define: the CLI list grammar. The CLI uses ordered `set/add/remove` flags with comma-separated list items; see [formatter-policy.markdown](formatter-policy.markdown) for that surface.

## Public Surface

The public typed surface is built from:

- `CharSet::all()`
- `CharSet::none()`
- `charset::ASCII`
- `charset::EMOJI_DEFAULTS`
- `charset::RIGHTS_MARKS`
- `charset::ARROWS`
- `charset::CARD_SUITS`
- `charset::is_variation_sequence_character(c)`
- `CharSet::singleton(c)`
- `!charset`
- `charset | other`
- `charset & other`
- `charset ^ other`
- `charset - other`

## Atoms

| Constructor               | Meaning                                                                            |
| ------------------------- | ---------------------------------------------------------------------------------- |
| `CharSet::all()`          | Every eligible dual-presentation character                                         |
| `CharSet::none()`         | No character                                                                       |
| `charset::ASCII`          | ASCII characters (U+0000-U+007F) that are in the eligible charset universe         |
| `charset::EMOJI_DEFAULTS` | Variation-sequence code points whose Unicode default side is emoji                 |
| `charset::RIGHTS_MARKS`   | The rights marks currently listed in Unicode's `emoji-variation-sequences.txt`     |
| `charset::ARROWS`         | The arrow characters currently listed in Unicode's `emoji-variation-sequences.txt` |
| `charset::CARD_SUITS`     | The card suits currently listed in Unicode's `emoji-variation-sequences.txt`       |
| `CharSet::singleton(c)`   | A single eligible code point, or empty if `c` is outside the policy universe       |

## Combinators

| Constructor        | Meaning                                    |
| ------------------ | ------------------------------------------ |
| `!charset`         | Eligible characters not in `charset`       |
| `charset \| other` | Characters matched by either set           |
| `charset & other`  | Characters matched by both sets            |
| `charset ^ other`  | Characters matched by exactly one set      |
| `charset - other`  | Characters in `charset` but not in `other` |

The assignment operators `|=`, `&=`, `^=`, and `-=` have the corresponding
in-place meanings.

## Queries

| Query                                         | Meaning                                             |
| --------------------------------------------- | --------------------------------------------------- |
| `charset::is_variation_sequence_character(c)` | Whether `c` is inside the eligible charset universe |

## Named sets

### `ascii`

Matches characters in U+0000-U+007F.

### `emoji-defaults`

Matches variation-sequence code points whose Unicode default side is emoji. This is narrower than the full Unicode `Emoji_Presentation` property because it excludes emoji-only code points that do not participate in `evfmt` policy.

### `rights-marks`

Matches the rights marks currently listed in Unicode's `emoji-variation-sequences.txt`: ©️ (`U+00A9`), ®️ (`U+00AE`), ™️ (`U+2122`).

This is a project-defined set tied to the repository's pinned Unicode version, not a permanently frozen member list. It may change when `evfmt` upgrades Unicode support.

### `arrows`

Matches the arrow characters currently listed in Unicode's `emoji-variation-sequences.txt`: ↔️ (`U+2194`), ↕️ (`U+2195`), ↖️ (`U+2196`), ↗️ (`U+2197`), ↘️ (`U+2198`), ↙️ (`U+2199`), ↩️ (`U+21A9`), ↪️ (`U+21AA`), ➡️ (`U+27A1`), ⤴️ (`U+2934`), ⤵️ (`U+2935`), ⬅️ (`U+2B05`), ⬆️ (`U+2B06`), ⬇️ (`U+2B07`).

This is a project-defined set tied to the repository's pinned Unicode version, not a permanently frozen member list. It may change when `evfmt` upgrades Unicode support.

### `card-suits`

Matches the card suits currently listed in Unicode's `emoji-variation-sequences.txt`: ♠️ (`U+2660`), ♣️ (`U+2663`), ♥️ (`U+2665`), ♦️ (`U+2666`).

This is a project-defined set tied to the repository's pinned Unicode version, not a permanently frozen member list. It may change when `evfmt` upgrades Unicode support.

## Examples

```rust
use evfmt::charset;

let prefer_bare = charset::ASCII | charset::RIGHTS_MARKS;
let treat_bare_as_text = charset::ASCII | charset::RIGHTS_MARKS;
```
