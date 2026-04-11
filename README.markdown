# ✨️ evfmt: opinionated emoji variation formatter

[![crates.io](https://img.shields.io/crates/v/evfmt?logo=rust)](https://crates.io/crates/evfmt)
[![docs.rs](https://img.shields.io/docsrs/evfmt?logo=docs.rs)](https://docs.rs/evfmt)
[![MSRV](https://img.shields.io/crates/msrv/evfmt?logo=rust)](https://crates.io/crates/evfmt)
[![coverage](https://img.shields.io/codecov/c/github/favonia/evfmt?logo=codecov)](https://app.codecov.io/gh/favonia/evfmt)
[![source code](https://img.shields.io/badge/source%20code-GitHub-24292f?logo=github)](https://github.com/favonia/evfmt)

`evfmt` normalizes text/emoji variation selectors in your files.

It is both a command-line tool and a Rust library.

This project was developed with AI assistance, guided by [detailed design documents](docs/designs/README.markdown) and substantial testing.

## 🔣 What Are Variation Selectors?

Many Unicode characters have dual presentations, text and emoji:

|        | text presentation | emoji presentation |
| ------ | ----------------- | ------------------ |
| U+0023 | #                 | #️                 |
| U+00A9 | ©︎                 | ©️                 |
| U+26A0 | ⚠︎                 | ⚠️                 |
| U+2764 | ❤︎                 | ❤️                 |

Unicode provides invisible variation selectors (`U+FE0E` for text, `U+FE0F` for emoji) to request a specific presentation (though platforms may not always honor the request). Each character can therefore appear in three forms: **bare** (no selector), **text** (`U+FE0E`), or **emoji** (`U+FE0F`). Without explicit selectors, the same file may look different on different platforms. `evfmt` normalizes these selectors for you.

The emoji selector `U+FE0F` also appears in multi-character emoji sequences such as keycaps and [Emoji ZWJ sequences](https://www.unicode.org/reports/tr51/#def_emoji_zwj_sequence) (where multiple emoji are joined into one). `evfmt` normalizes these sequences to their [fully qualified](https://www.unicode.org/reports/tr51/#def_fully_qualified_emoji) forms as well.

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
# Format one file.
evfmt README.md
```

```sh
# Format a group of files.
evfmt docs/*.md
```

```sh
# Format the current project.
evfmt .
```

```sh
# A bare heart (U+2764) becomes the emoji-form heart by default.
# The first command prints the same string as the second command: Love ❤️
printf '%b' 'Love \u2764' | evfmt -
printf '%b' 'Love \u2764\ufe0f'
```

### ✅️ Checking Mode

```sh
# Check without modifying (exits 1 if changes are needed)
evfmt check .
```

```sh
# If a file name is ambiguous with a command, add `--` or use `./`
evfmt -- check
evfmt ./check
```

### 🚪 Exit Codes

| Code | Meaning                               |
| ---- | ------------------------------------- |
| `0`  | Success (or no changes in check mode) |
| `1`  | Changes needed (check mode only)      |
| `2`  | Error (I/O, invalid UTF-8, usage)     |

## 🙈 Ignore Filters

By default, `evfmt` skips files ignored by Git, files matched by `.evfmtignore`, and hidden files or directories. Change the enabled ignore filters only when the project has a specific reason to include or exclude one of those classes.

- `--set-ignore=<filter>[,<filter>...]`
- `--add-ignore=<filter>[,<filter>...]`
- `--remove-ignore=<filter>[,<filter>...]`

Ignore flags take one or more comma-separated filter labels:

- `git`: ignore files matched by Git ignore rules
- `evfmt`: ignore files matched by `.evfmtignore`
- `hidden`: ignore hidden files and directories

Use commas to combine labels: `git,evfmt,hidden`.

Use `all` by itself to select every ignore filter. For example, `--remove-ignore=all` formats everything reachable from the operands, including Git-ignored, `.evfmtignore`-ignored, and hidden files. Use `none` by itself with `--set-ignore` to disable all ignore filters.

Use this when you want to format hidden files while still honoring Git ignore rules and `.evfmtignore`:

```sh
evfmt --remove-ignore=hidden .
```

## 📐 Policy for Dual Presentations

By default, `evfmt` leaves bare ASCII characters alone, but adds explicit selectors to non-ASCII characters with dual presentations so they render more consistently across platforms.

For example, `#` stays bare. A bare copyright sign (U+00A9) gets an explicit emoji selector under the default policy, so it is normalized to `©️` (U+00A9 U+FE0F).

The defaults work well for most projects: bare ASCII characters in text presentation are left alone, while all other characters with dual presentations get an explicit selector, defaulting to emoji.

⚠️ `evfmt` is a formatter, not a presentation editor. If you want to change how the copyright sign looks on your platform—say, switching it from emoji presentation to text presentation—do that in your editor by adding or removing the variation selector (`U+FE0E` or `U+FE0F`). Run `evfmt` only after you are happy with how your document renders.

### 📖 Cookbook

Use these recipes when the default policy is close to what you want, but a small class of symbols needs different handling.

#### Keep Text-Looking Marks in Text Presentation

Use this when copyright, registered, and trademark-style marks should render as text-style symbols, but you still want explicit selectors for portability:

```sh
evfmt \
  --add-bare-as-text=rights-marks \
  README.markdown
```

With that option, bare rights marks normalize to explicit text forms such as `©︎`, `®︎`, and `™︎`. Explicit emoji-form marks such as `©️` stay emoji.

#### Keep Text-Looking Marks Bare

Use this when copyright and trademark-style marks already look like text on your reference platform, and you want their text presentation to stay bare in your files:

```sh
evfmt \
  --add-bare-as-text=rights-marks \
  --add-prefer-bare=rights-marks \
  README.markdown
```

With those options, bare or text-form copyright-style marks normalize to bare copyright-style marks. Explicit emoji-form marks such as `©️` stay emoji.

#### Keep Symbols as Text

Use this when arrows and card suits should stay text-style symbols in a technical document, log, or README:

```sh
evfmt \
  --add-bare-as-text=arrows,card-suits \
  README.markdown
```

With that option, bare arrows and card suits normalize to explicit text forms such as `➡︎` and `♠︎`. Explicit emoji-form symbols such as `➡️` and `♠️` stay emoji.

### ⚙️ Detailed Explanation

The policy is shaped by two choices: how bare characters render on your _reference platform_, and which bare characters are stable enough to keep bare in your files. A reference platform is the environment whose bare-character rendering you are using as your baseline, usually the editor, terminal, or browser where you review the formatted text.

The CLI exposes those choices as two mutable sets:

- `bare-as-text`: Which characters the reference platform shows as text when bare. Many modern platforms show bare non-ASCII characters as emoji, so the default set is `ascii`.
- `prefer-bare`: Among characters that can stay bare without changing their appearance on the reference platform, which ones should stay bare rather than getting an explicit selector. The default set is also `ascii`, so non-ASCII characters always get an explicit selector for maximum cross-platform consistency.

To choose the right policy, first decide whether a character's bare form looks like text or emoji on your reference platform. Put it in `bare-as-text` if the bare form looks like text. Then decide whether the character should stay bare in the files you publish, as long as doing so preserves the intended presentation. Put it in `prefer-bare` if bare spelling is stable enough for your target platforms.

The two choices determine how `evfmt` repairs each ambiguous standalone character:

| If a character is...                | `evfmt` does this                                   |
| ----------------------------------- | --------------------------------------------------- |
| in `bare-as-text` and `prefer-bare` | changes explicit text to bare; leaves others alone  |
| in `bare-as-text` only              | changes bare to explicit text; leaves others alone  |
| in `prefer-bare` only               | changes explicit emoji to bare; leaves others alone |
| in neither set                      | changes bare to explicit emoji; leaves others alone |

With the default sets `bare-as-text = ascii` and `prefer-bare = ascii`, ASCII bare forms stay bare and non-ASCII bare forms get explicit emoji selectors.

#### Policy Flags

Use these flags to update the policy sets. Each flag takes one or more comma-separated charset items:

- To update the `bare-as-text` set:

  <dl>
  <dt><code>--set-bare-as-text=&lt;charset&gt;[,&lt;charset&gt;...]</code></dt>
  <dd>Replaces <code>bare-as-text</code> with the specified charset items.</dd>
  <dt><code>--add-bare-as-text=&lt;charset&gt;[,&lt;charset&gt;...]</code></dt>
  <dd>Adds charset items to <code>bare-as-text</code>.</dd>
  <dt><code>--remove-bare-as-text=&lt;charset&gt;[,&lt;charset&gt;...]</code></dt>
  <dd>Removes charset items from <code>bare-as-text</code>.</dd>
  </dl>

- To update the `prefer-bare` set:

  <dl>
  <dt><code>--set-prefer-bare=&lt;charset&gt;[,&lt;charset&gt;...]</code></dt>
  <dd>Replaces <code>prefer-bare</code> with the specified charset items.</dd>
  <dt><code>--add-prefer-bare=&lt;charset&gt;[,&lt;charset&gt;...]</code></dt>
  <dd>Adds charset items to <code>prefer-bare</code>.</dd>
  <dt><code>--remove-prefer-bare=&lt;charset&gt;[,&lt;charset&gt;...]</code></dt>
  <dd>Removes charset items from <code>prefer-bare</code>.</dd>
  </dl>

Both policy sets start as `ascii`, and flags are processed from left to right. `set-*` replaces the current set, `add-*` unions items into it, and `remove-*` subtracts items from it.

Supported charset items are:

- `ascii`: ASCII characters with text/emoji variation forms, such as `#`, `*`, and digits
- `emoji-defaults`: characters whose bare form defaults to emoji presentation in Unicode
- `rights-marks`: copyright and registered/trademark-style marks
- `arrows`: arrow symbols with text/emoji variation forms
- `card-suits`: card suit symbols with text/emoji variation forms
- `u(HEX)`: one Unicode code point, for example `u(00A9)`
- a single character, for example `#`, `*`, or `©️`; variation selectors are allowed and ignored when choosing the character

Use commas to combine items: `ascii,rights-marks,u(00A9)`. Named sets may change when `evfmt` upgrades Unicode support.

Use `all` by itself to select every character `evfmt` can format. For example, `--remove-prefer-bare=all` makes every format-supported character require an explicit selector. Use `none` by itself with `--set-*` policy flags to clear that policy set. For example, `--set-prefer-bare=none` stops keeping any character bare just because it was in `prefer-bare`; with the default `bare-as-text` set, bare ASCII then normalizes to explicit text form and bare non-ASCII normalizes to explicit emoji form.

## 🧩 Normalization of Emoji ZWJ Sequences

[Emoji ZWJ sequences](https://www.unicode.org/reports/tr51/#def_emoji_zwj_sequence) are sequences of multiple emoji characters joined by the zero-width joiner (ZWJ; `U+200D`). For example, the rainbow flag 🏳️‍🌈 is the white flag 🏳️ and the rainbow 🌈 joined together. These sequences are intended for emoji presentation only, so what should a formatter do when a component carries an explicit text variation selector (`U+FE0E`)? This situation should not arise in practice, but a formatter must handle it. [Unicode Technical Standard #51](https://www.unicode.org/reports/tr51/) says such a selector breaks the entire sequence, and the platform should display the components as separate images. A formatter must therefore either remove the ZWJ joiners to honor the text selector, or remove the text selector to restore the sequence.

`evfmt` chooses to restore the sequence: it normalizes every ZWJ sequence to its [fully qualified](https://www.unicode.org/reports/tr51/#def_fully_qualified_emoji) form, replacing text selectors with emoji selectors where needed. This intentionally departs from a strict reading of the standard, but keeps all changes limited to variation selectors and matches the most likely user intent. `evfmt` also inserts missing emoji selectors on bare components to bring ZWJ sequences to their fully qualified forms.

## ⚖️ License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## 🐛 Issues

Please report bugs, regressions, and feature requests in the [issue tracker](https://github.com/favonia/evfmt/issues).
