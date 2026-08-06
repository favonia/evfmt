# Design Note: PolicyKeySet API

Read when: changing the typed `PolicyKeySet` API, adding named sets, or modifying set-combinator semantics.

Defines: the typed `PolicyKeySet` model exported through `evfmt::policy_key_set` and used by `evfmt::Policy`. The model describes finite sets of sanctioned formatter policy keys.

Does not define: the CLI list grammar. The CLI uses ordered `set/add/remove` flags with comma-separated list items; see [cli.markdown](cli.markdown) for that surface.

## Public Surface

The public typed surface is built from:

- `PolicyKeySet::all()`
- `PolicyKeySet::none()`
- `PolicyKeySet::singleton(c)`
- `PolicyKeySet::singleton_keycap(c)`
- `set.contains(c)`
- `set.contains_keycap(c)`
- `policy_key_set::ASCII`
- `policy_key_set::TEXT_DEFAULTS`
- `policy_key_set::EMOJI_DEFAULTS`
- `policy_key_set::VARIATION_BASES`
- `policy_key_set::KEYCAP_RGI`
- `policy_key_set::KEYCAP_TEXT_DEFAULTS`
- `policy_key_set::KEYCAP_EMOJI_DEFAULTS`
- `policy_key_set::KEYCAP_VARIATION_BASES`
- `policy_key_set::is_variation_sequence_character(c)`
- `!set`
- `set | other`
- `set & other`
- `set ^ other`
- `set - other`

## Policy Keys

Every `PolicyKeySet` contains policy keys. A policy key is one variation-sequence base plus one domain:

- ordinary, queried with `contains(c)`, for non-keycap selector slots
- keycap-character, queried with `contains_keycap(c)`, where the base is followed by `U+20E3 COMBINING ENCLOSING KEYCAP`

Both domains are indexed by the same pinned `emoji-variation-sequences.txt` base-character table. Characters outside that table never form policy keys.

The internal bitset type is private. Public code should treat `PolicyKeySet` as an opaque value with constructors, queries, and set operators.

## Atoms

| Constructor                              | Meaning                                                                          |
| ---------------------------------------- | -------------------------------------------------------------------------------- |
| `PolicyKeySet::all()`                    | Every non-keycap and keycap-character policy key                                 |
| `PolicyKeySet::none()`                   | No policy key                                                                    |
| `PolicyKeySet::singleton(c)`             | One non-keycap policy key, or empty if `c` is outside the policy universe        |
| `PolicyKeySet::singleton_keycap(c)`      | One keycap-character policy key, or empty if `c` is outside the policy universe  |
| `policy_key_set::ASCII`                  | ASCII variation-sequence bases (`#`, `*`, and `0`-`9`) as non-keycap policy keys |
| `policy_key_set::TEXT_DEFAULTS`          | Text-default variation-sequence bases as non-keycap policy keys                  |
| `policy_key_set::EMOJI_DEFAULTS`         | Emoji-default variation-sequence bases as non-keycap policy keys                 |
| `policy_key_set::VARIATION_BASES`        | Every non-keycap policy key for a variation-sequence base                        |
| `policy_key_set::KEYCAP_RGI`             | RGI emoji keycap bases (`#`, `*`, `0`-`9`) as keycap-character policy keys       |
| `policy_key_set::KEYCAP_TEXT_DEFAULTS`   | Text-default variation-sequence bases as keycap-character policy keys            |
| `policy_key_set::KEYCAP_EMOJI_DEFAULTS`  | Emoji-default variation-sequence bases as keycap-character policy keys           |
| `policy_key_set::KEYCAP_VARIATION_BASES` | Every keycap-character policy key for a variation-sequence base                  |

Unprefixed named sets affect non-keycap policy keys only. Keycap-specific membership is expressed explicitly with `KEYCAP_RGI`, `KEYCAP_TEXT_DEFAULTS`, `KEYCAP_EMOJI_DEFAULTS`, `KEYCAP_VARIATION_BASES`, or `PolicyKeySet::singleton_keycap(c)`.

`PolicyKeySet::all()` is exactly:

```rust
policy_key_set::KEYCAP_VARIATION_BASES | policy_key_set::VARIATION_BASES
```

## Combinators

| Constructor    | Meaning                                           |
| -------------- | ------------------------------------------------- |
| `!set`         | Policy keys in the universe that are not in `set` |
| `set \| other` | Policy keys matched by either set                 |
| `set & other`  | Policy keys matched by both sets                  |
| `set ^ other`  | Policy keys matched by exactly one set            |
| `set - other`  | Policy keys in `set` but not in `other`           |

Operators apply componentwise to non-keycap and keycap-character domains. The assignment operators `|=`, `&=`, `^=`, and `-=` have the corresponding in-place meanings.

## Display

`Display` renders fully empty and fully full sets as `none` and `all`. Other sets render members in variation-table order, separated by commas.

Examples:

- `PolicyKeySet::singleton('#')` renders as `u(0023)`
- `PolicyKeySet::singleton_keycap('#')` renders as `keycap:u(0023)`

## Queries

| Query                                                | Meaning                                                       |
| ---------------------------------------------------- | ------------------------------------------------------------- |
| `policy_key_set::is_variation_sequence_character(c)` | Whether `c` is inside the eligible base-character table       |
| `set.contains(c)`                                    | Whether the non-keycap policy key for `c` is in the set       |
| `set.contains_keycap(c)`                             | Whether the keycap-character policy key for `c` is in the set |

## Examples

```rust
use evfmt::{PolicyKeySet, policy_key_set};

let prefer_bare = policy_key_set::ASCII | PolicyKeySet::singleton('\u{00A9}');
let treat_bare_as_text = policy_key_set::TEXT_DEFAULTS
    | policy_key_set::KEYCAP_VARIATION_BASES;
```
