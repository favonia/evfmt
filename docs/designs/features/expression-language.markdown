# Design Note: Expression Language

Read when: changing the set DSL parser, adding named sets, or modifying policy expression syntax.

Defines: the DSL contract for policy expressions used by `--prefer-bare-for` and `--treat-bare-as-text-for`. The language describes character sets, not boolean predicates.

## Grammar

```text
expr          = "all" | "none" | named_set | u_expr | quoted
              | union_expr | subtract_expr
top_expr      = expr | except_expr
named_set     = "ascii" | "emoji-defaults" | "rights-marks" | "arrows" | "card-suits"
u_expr        = "u(" hex4_6 ")"
quoted        = "'" char "'" | '"' char+ '"'
union_expr    = "union(" expr ("," expr)* ")"
subtract_expr = "subtract(" expr "," expr ("," expr)* ")"
except_expr   = "except(" expr ")"
```

## Atoms

| Form             | Meaning                                                                  |
| ---------------- | ------------------------------------------------------------------------ |
| `all`            | Every character                                                          |
| `none`           | No character                                                             |
| `ascii`          | ASCII characters (U+0000-U+007F)                                         |
| `emoji-defaults` | Variation-sequence code points whose Unicode default side is emoji       |
| `rights-marks`   | ©️, ®️, ™️                                                               |
| `arrows`         | Arrow characters                                                         |
| `card-suits`     | ♠️, ♣️, ♥️, ♦️                                                           |
| `u(XXXX)`        | A single code point                                                      |
| `'c'`            | A single character; selectors inside the quotes do not matter            |
| `"abc"`          | Union of contained characters; selectors inside the quotes do not matter |

## Combinators

| Form                            | Meaning                                          |
| ------------------------------- | ------------------------------------------------ |
| `union(e1, e2, ...)`            | Characters matched by any sub-expression         |
| `subtract(base, ex1, ex2, ...)` | Characters in `base` but not in any excluded set |
| `except(e)`                     | Top-level only; sugar for `subtract(all, e)`     |

## Named sets

### `ascii`

Matches characters in U+0000-U+007F.

### `emoji-defaults`

Matches variation-sequence code points whose Unicode default side is emoji. This is narrower than the full Unicode `Emoji_Presentation` property because it excludes emoji-only code points that do not participate in `evfmt` policy.

### `rights-marks`

Matches ©️, ®️, ™️.

### `arrows`

Matches the project-defined arrow set.

### `card-suits`

Matches ♠️, ♣️, ♥️, ♦️.

## Examples

```sh
--prefer-bare-for='ascii'
--treat-bare-as-text-for='ascii'
--treat-bare-as-text-for='all'
--prefer-bare-for='union(u(0023), u(002A))'
--treat-bare-as-text-for='union(ascii, rights-marks)'
--prefer-bare-for='subtract(ascii, "#*")'
--treat-bare-as-text-for='none'
```
