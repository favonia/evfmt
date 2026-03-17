# ✨️ evfmt: opinionated emoji variation formatter

`evfmt` normalizes text/emoji variation selectors in your files.

It is both a command-line tool and a Rust library.

This project was developed with AI assistance, guided by [detailed design documents](docs/designs/README.markdown) and substantial testing.

## 🔣 What Are Variation Selectors

Many Unicode characters have dual presentations, text and emoji:

|        | text presentation | emoji presentation |
| ------ | ----------------- | ------------------ |
| U+0023 | #                 | #️                 |
| U+00A9 | ©︎                 | ©️                 |
| U+26A0 | ⚠︎                 | ⚠️                 |
| U+2764 | ❤︎                 | ❤️                 |

Unicode provides invisible variation selectors (`U+FE0E` for text, `U+FE0F` for emoji) to request a specific presentation (though platforms may not always honor the request). Each character can therefore appear in three forms: **bare** (no selector), **text** (`U+FE0E`), or **emoji** (`U+FE0F`). Without explicit selectors, the same file may look different on different platforms. `evfmt` normalizes these selectors for you.

The emoji selector `U+FE0F` also appears in multi-character emoji sequences such as keycaps and [ZWJ sequences](https://www.unicode.org/reports/tr51/#def_emoji_zwj_sequence) (where multiple emoji are joined into one). `evfmt` normalizes these sequences to their [fully qualified](https://www.unicode.org/reports/tr51/#def_fully_qualified_emoji) forms as well.

## ✨️ What It Does

Different platforms can render the same character differently when variation selectors are missing or ambiguous. `evfmt` produces a canonical source spelling that reduces this cross-platform inconsistency:

- Chooses a deterministic form—bare, text, or emoji—for each character with dual presentations
- Removes stray selectors in unsupported positions
- Fixes multi-character emoji sequences that are not [fully qualified](https://www.unicode.org/reports/tr51/#def_fully_qualified_emoji)
- Respects `.gitignore` and `.evfmtignore`

**Hard invariants:** `evfmt` is idempotent, deterministic, and only modifies variation selectors—no other content is touched.

## 📦️ Installation

Install the CLI from [crates.io](https://crates.io/crates/evfmt):

```sh
cargo install evfmt --locked
```

Add the library to another crate with `cargo add`:

```sh
cargo add evfmt
```

**Minimum supported Rust version (MSRV):** Rust 1.88

If you are working from a local checkout, you can also install it with:

```sh
cargo install --path evfmt
```

## 🚀 Quick Start

### 🛠️ Fixing Mode

```sh
# Format files in place
evfmt README.md docs/*.md

# A bare heart becomes the emoji-form heart by default.
# Both commands print: Love ❤️
printf '%b' 'Love \u2764' | evfmt -
printf '%b' 'Love \u2764\ufe0f'
```

### ✅️ Checking Mode

```sh
# Check without modifying (exits 1 if changes are needed)
evfmt check README.md

# If a file name is ambiguous with a command, add `--` or use `./`
evfmt -- check
evfmt ./check
```

## 🧭 What the Default Policy Does

By default, `evfmt` leaves bare ASCII characters alone, but adds explicit selectors to non-ASCII characters with dual presentations so they render more consistently across platforms.

For example, `#` stays bare. The copyright sign (`©︎`, U+00A9) gets an explicit emoji selector under the default policy, so a bare copyright sign is normalized to `©️`.

## 📐 Resolution Policy

The defaults work well for most projects: bare ASCII characters in text presentation are left alone, while all other characters with dual presentations get an explicit selector—defaulting to emoji.

⚠️ `evfmt` is a formatter, not a presentation editor. If you want to change how the copyright sign looks on your platform—say, switching it from emoji presentation to text presentation—do that in your editor by adding or removing the variant selector (`U+FE0E` or `U+FE0F`). Run `evfmt` only after you are happy with how your document renders.

### ⚙️ Customizing the Policy

The policy is shaped by two ideas: how bare characters render on your _reference platform_, and which bare characters are stable across the _target platforms_ where you want consistent results. Two options control the policy:

- `--treat-bare-as-text-for`: Which characters the reference platform shows as text when bare. Many modern platforms show bare non-ASCII characters as emoji, so the default is `ascii`.
- `--prefer-bare-for`: Among characters that can stay bare without changing their appearance on the reference platform, which ones should stay bare rather than getting an explicit selector. The default is `ascii`, so non-ASCII characters always get an explicit selector for maximum cross-platform consistency.

These two options together completely determine what the tool does when it encounters each form:

|                     | Treating bare as text            | Not treating bare as text         |
| ------------------- | -------------------------------- | --------------------------------- |
| Preferring bare     | Change text to bare; keep others | Change emoji to bare; keep others |
| Not preferring bare | Change bare to text; keep others | Change bare to emoji; keep others |

With the default values `--treat-bare-as-text-for=ascii` and `--prefer-bare-for=ascii`, we can derive the following actions:

|                                      | Treating bare as text (`ascii`) | Not treating bare as text (except `ascii`) |
| ------------------------------------ | ------------------------------- | ------------------------------------------ |
| Preferring bare (`ascii`)            | Change text to bare for ASCII   | Change emoji to bare for none              |
| Not preferring bare (except `ascii`) | Change bare to text for none    | Change bare to emoji for non-ASCII         |

The expression language for these two options supports combinators (`union`, `subtract`, `except`), named sets (`ascii`, `emoji-defaults`, `arrows`, `card-suits`, ...), single characters (`u(00A9)`, `'#'`), and string literals (`"#*"`) as unions of their characters. In quoted literals, selectors do not matter, so adding or removing them does not change the expression. Run `evfmt --help-expression` for the full reference.

The more detailed policy decision table and derivation model live in [docs/designs/features/formatter-policy.markdown](docs/designs/features/formatter-policy.markdown).

## 🚪 Exit Codes

| Code | Meaning                               |
| ---- | ------------------------------------- |
| `0`  | Success (or no changes in check mode) |
| `1`  | Changes needed (check mode only)      |
| `2`  | Error (I/O, invalid UTF-8, usage)     |

## ⚖️ License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## 🐛 Issues

Please report bugs, regressions, and feature requests in the [issue tracker](https://github.com/favonia/evfmt/issues).
