# ✨ evfmt: emoji-variation selector formatter

[![crates.io](https://img.shields.io/crates/v/evfmt?logo=rust)](https://crates.io/crates/evfmt)
[![docs.rs](https://img.shields.io/docsrs/evfmt?logo=docs.rs)](https://docs.rs/evfmt)
[![MSRV](https://img.shields.io/crates/msrv/evfmt?logo=rust)](https://crates.io/crates/evfmt)
[![coverage](https://img.shields.io/codecov/c/github/favonia/evfmt?logo=codecov)](https://app.codecov.io/gh/favonia/evfmt)
[![source code](https://img.shields.io/badge/source%20code-GitHub-24292f?logo=github)](https://github.com/favonia/evfmt)

`evfmt` is an opinionated formatter for Unicode presentation selectors (`U+FE0E` and `U+FE0F`).

The name stands for “emoji variation formatter”. It is a command-line formatter and a Rust library.

This project was developed with AI assistance, guided by [detailed design documents](docs/designs/README.markdown) and substantial testing.

## 🧭 Stability

`evfmt` is ready for normal formatter use. Core commands such as `evfmt format` and `evfmt check`, documented exit codes, and the formatter's hard invariants are intended to remain stable.

The Rust library APIs and some advanced CLI use are still experimental. `evfmt` follows [Cargo's SemVer compatibility conventions](https://doc.rust-lang.org/cargo/reference/semver.html).

## 🔣 What Are Presentation Selectors?

Many Unicode characters have dual presentations, text and emoji:

|        | text presentation | emoji presentation |
| ------ | ----------------- | ------------------ |
| U+0023 | #                 | #️                 |
| U+00A9 | ©︎                 | ©️                 |
| U+26A0 | ⚠︎                 | ⚠️                 |
| U+2764 | ❤︎                 | ❤️                 |

Unicode provides invisible presentation selectors (`U+FE0E` for text, `U+FE0F` for emoji) to request a specific presentation (though platforms may not always honor the request). These two characters (`U+FE0E` and `U+FE0F`) are _variation selectors_. [Unicode Technical Standard #51](https://www.unicode.org/reports/tr51/tr51-29.html) also calls them _presentation selectors_ in the emoji context, and this document follows that convention. (The _ev_ in _evfmt_ stands for _emoji variation_, after the [emoji variation sequences](https://www.unicode.org/reports/tr51/tr51-29.html#def_emoji_variation_sequence) that these selectors produce.)

Each character can therefore appear in three forms: **bare** (no selector), **text** (`U+FE0E`), or **emoji** (`U+FE0F`). Without explicit selectors, certain dual-presentation characters may look different on different platforms. On the other hand, selectors are considered redundant or even defective in other contexts. `evfmt` normalizes these selectors for you.

The emoji selector `U+FE0F` also appears in multi-character emoji sequences such as keycaps and [Emoji ZWJ sequences](https://www.unicode.org/reports/tr51/tr51-29.html#def_emoji_zwj_sequence) (where multiple emoji are joined into one). `evfmt` normalizes selector usage in these sequences as well.

## ✨ What It Does

Different platforms can render the same character differently when presentation selectors are missing or ambiguous. `evfmt` produces a canonical source spelling that reduces this cross-platform inconsistency:

- Chooses a deterministic form—bare, text, or emoji—for each character with dual presentations
- Preserves all emoji sequences that are [recommended for general interchange (RGI)](https://www.unicode.org/reports/tr51/tr51-29.html#def_rgi_set)
- Removes stray selectors in unsupported positions
- Respects `.gitignore` and `.evfmtignore`

**Hard invariants:** `evfmt` is idempotent, deterministic, and only modifies presentation selectors—no other content is touched.

## 📦 Installation

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

### 🛠️ Formatting Mode

```sh
# Format one file.
evfmt format README.markdown
```

```sh
# Format a group of files.
evfmt format docs/*.md
```

```sh
# Format files under the current directory recursively.
evfmt format .
```

```sh
# A bare heart (U+2764) becomes the emoji-form heart by default.
# The first command prints the same string as the second command: Love ❤️
printf '%b' 'Love \u2764' | evfmt format
printf '%b' 'Love \u2764\ufe0f'
```

Use `-` as an explicit stdin operand when mixing stdin with files. A path such as `./-` still means a file literally named `-`. Repeating `-` is allowed and reads the same stdin stream again from its current position; with piped input, the first `-` normally consumes the stream.

```sh
evfmt format a.txt - b.txt
evfmt format ./-
printf '%b' 'Love \u2764' | evfmt check -
```

### ✅ Checking Mode

```sh
# Check without modifying (exits 1 if changes are needed)
evfmt check .
```

With no file operands, `evfmt check` checks stdin.

```sh
# If a file name looks like an option, add `--` before file operands.
evfmt format -- --set-ignore
evfmt check -- --set-ignore
```

### 🚪 Exit Codes

| Code | Meaning                               |
| ---- | ------------------------------------- |
| `0`  | Success (or no changes in check mode) |
| `1`  | Changes needed (check mode only)      |
| `2`  | Error (I/O, invalid UTF-8, usage)     |

## 📝 Notes on Specific Emoji Sequences

### 🧩 Emoji ZWJ Sequences

[Emoji ZWJ sequences](https://www.unicode.org/reports/tr51/tr51-29.html#def_emoji_zwj_sequence) are sequences of multiple emoji characters joined by the zero-width joiner (ZWJ; `U+200D`). For example, the rainbow flag 🏳️‍🌈 joins the white flag 🏳️ and the rainbow 🌈. `evfmt` normalizes each component in a ZWJ sequence as if that component appeared without the surrounding ZWJ links.

### 🔢 Normalization of Keycap Sequences

Keycap sequences combine a base character (`0`–`9`, `#`, or `*`) with the combining enclosing keycap (`U+20E3`) to produce keycap buttons like 1️⃣ and #️⃣. The base character can appear bare, with a text selector (`U+FE0E`), or with an emoji selector (`U+FE0F`) before the keycap mark. Historically, bare keycap sequences were used for both text and emoji presentations, which made them ambiguous. `evfmt` normalizes bare keycap sequences to explicit text forms by default. Explicit text and emoji selectors in keycap forms are preserved.

## 🧪 Advanced Configuration

The options below are for projects that need to tune traversal or presentation policy beyond the default formatter behavior.

### 🙈 Ignore Filters

By default, `evfmt` enables all ignore filters: it skips files ignored by Git, files matched by `.evfmtignore`, and hidden files or directories. Change the enabled ignore filters only when you have a specific reason to include or exclude one of those classes.

| Option                                   | Effect              |
| ---------------------------------------- | ------------------- |
| `--set-ignore=<filter>[,<filter>...]`    | Replace the set     |
| `--add-ignore=<filter>[,<filter>...]`    | Add to the set      |
| `--remove-ignore=<filter>[,<filter>...]` | Remove from the set |

Ignore flags take one or more comma-separated filter labels:

| Label    | Meaning                           |
| -------- | --------------------------------- |
| `git`    | Files matched by Git ignore rules |
| `evfmt`  | Files matched by `.evfmtignore`   |
| `hidden` | Hidden files and directories      |

Use commas to combine labels: `git,evfmt,hidden`.

Use `all` by itself to select every ignore filter; this is the default. For example, `--remove-ignore=all` formats everything reachable from the operands, including Git-ignored, `.evfmtignore`-ignored, and hidden files. Using `none` by itself with `--set-ignore` also disables all ignore filters.

Use this when you want to format hidden files while still honoring Git ignore rules and `.evfmtignore`:

```sh
evfmt format --remove-ignore=hidden .
```

<a id="singleton-character"></a>

### 📐 Presentation Policy Cookbook

By default, `evfmt` leaves bare ASCII characters and Unicode emoji-default characters alone, while text-default non-ASCII characters with dual presentations get an explicit emoji selector. For example, `#` and bare sparkles (U+2728) stay bare, while a bare copyright sign (U+00A9) normalizes to `©️` (U+00A9 U+FE0F).

⚠️ `evfmt` is a formatter, not a presentation editor. If you want to change how the copyright sign looks on your platform—say, switching it from emoji presentation to text presentation—do that in your editor by adding or removing the presentation selector (`U+FE0E` or `U+FE0F`). Run `evfmt` only after you are happy with how your document renders.

Use these recipes when the default policy is close to what you want, but a small class of symbols needs different handling.

#### Keep Selected Symbols in Text Presentation

Use this when rights marks, arrows, and card suits already render as text-style symbols, and you want explicit text selectors for portability:

```sh
evfmt format \
  --add-bare-as-text=rights-marks,arrows,card-suits \
  README.markdown
```

With that option, bare rights marks, arrows, and card suits normalize to explicit text forms such as `©︎`, `®︎`, `™︎`, `➡︎`, and `♠︎`. Explicit emoji-form symbols such as `©️`, `➡️`, and `♠️` stay emoji.

#### Keep Text-Looking Marks Bare

Use this when copyright and trademark-style marks already look like text on your reference platform, and you want their text presentation to stay bare in your files:

```sh
evfmt format \
  --add-bare-as-text=rights-marks \
  --add-prefer-bare=rights-marks \
  README.markdown
```

With those options, bare or text-form copyright-style marks normalize to bare copyright-style marks. Explicit emoji-form marks such as `©️` stay emoji.

#### Use Emoji-Style Keycaps

Use this when existing text contains bare keycap sequences such as `1` + `U+20E3` and `#` + `U+20E3`, and you want them treated as emoji keycaps rather than text-style keycaps. [Some older emoji mappings](https://www.unicode.org/L2/L2011/11414-emoji-var-seq.pdf) used these bare keycap sequences as emoji.

```sh
evfmt format \
  --remove-bare-as-text=keycap-emojis \
  README.markdown
```

With that option, bare keycap sequences normalize to explicit emoji forms such as `1️⃣` and `#️⃣`. Explicit text-form keycaps such as `1︎⃣` stay text.

### ⚙️ How Presentation Policy Works

The policy is shaped by two choices: how bare characters render on your _reference platform_, and which bare characters are stable enough to keep bare in your files. A reference platform is the environment whose bare-character rendering you are using as your baseline, usually the editor, terminal, or browser where you review the formatted text.

The CLI exposes those choices as two mutable sets:

- `bare-as-text`: Which variation positions the reference platform shows as text when bare. Many modern platforms show bare non-ASCII, non-keycap characters as emoji, so the default set is `ascii,keycap-chars`.
- `prefer-bare`: Among characters that can stay bare without changing their appearance on the reference platform, which ones should stay bare rather than getting an explicit selector. The default set is `ascii,emoji-defaults`, so characters with default emoji presentation in Unicode stay bare, while text-default non-ASCII characters still get explicit selectors.

To choose the right policy, first decide whether a character's bare form looks like text or emoji on your reference platform. Put it in `bare-as-text` if the bare form looks like text. Then decide whether the character should stay bare in the files you publish, as long as doing so preserves the intended presentation. Put it in `prefer-bare` if bare spelling is stable enough for your target platforms.

The two choices determine how `evfmt` repairs each ambiguous standalone variation position:

| If a character is...                | `evfmt` does this                                   |
| ----------------------------------- | --------------------------------------------------- |
| in `bare-as-text` and `prefer-bare` | changes explicit text to bare; leaves others alone  |
| in `bare-as-text` only              | changes bare to explicit text; leaves others alone  |
| in `prefer-bare` only               | changes explicit emoji to bare; leaves others alone |
| in neither set                      | changes bare to explicit emoji; leaves others alone |

With the default sets `bare-as-text = ascii,keycap-chars` and `prefer-bare = ascii,emoji-defaults`, ASCII bare forms and emoji-default bare forms stay bare, text-default non-ASCII bare forms get explicit emoji selectors, and bare keycap-character forms get explicit text selectors.

#### Policy Flags

Use these flags to update the policy sets. Each flag takes one or more comma-separated variation sets:

To update the `bare-as-text` set:

| Option                                   | Effect              |
| ---------------------------------------- | ------------------- |
| `--set-bare-as-text=<set>[,<set>...]`    | Replace the set     |
| `--add-bare-as-text=<set>[,<set>...]`    | Add to the set      |
| `--remove-bare-as-text=<set>[,<set>...]` | Remove from the set |

To update the `prefer-bare` set:

| Option                                  | Effect              |
| --------------------------------------- | ------------------- |
| `--set-prefer-bare=<set>[,<set>...]`    | Replace the set     |
| `--add-prefer-bare=<set>[,<set>...]`    | Add to the set      |
| `--remove-prefer-bare=<set>[,<set>...]` | Remove from the set |

The policy sets start as `prefer-bare = ascii,emoji-defaults` and `bare-as-text = ascii,keycap-chars`, and flags are processed from left to right. `set-*` replaces the current set, `add-*` unions items into it, and `remove-*` subtracts items from it.

Each `<set>` in the list can be either one of the named sets below, one code point with `u(HEX)`, such as `u(00A9)`, or one character, such as `#`, `*`, or `©️`. Presentation selectors are ignored when matching a single character. Except for sets whose names start with `keycap-`, named sets apply only to ordinary non-keycap positions.

| Set                | Meaning                                                                                |
| ------------------ | -------------------------------------------------------------------------------------- |
| `ascii`            | Dual-presentation characters in the ASCII range, such as `#`, `*`, and digits          |
| `text-defaults`    | Dual-presentation characters whose bare form defaults to text presentation in Unicode  |
| `emoji-defaults`   | Dual-presentation characters whose bare form defaults to emoji presentation in Unicode |
| `rights-marks`     | Copyright and registered/trademark-style marks with dual presentations                 |
| `arrows`           | Arrow symbols with dual presentations                                                  |
| `card-suits`       | Card suit symbols with dual presentations                                              |
| `keycap-chars`     | Dual-presentation characters in keycap-character positions                             |
| `non-keycap-chars` | Dual-presentation characters in ordinary non-keycap positions                          |
| `keycap-emojis`    | Emoji keycap bases (`#`, `*`, `0`–`9`) in keycap-character positions                   |

The meaning of named sets may change as Unicode adds or revises dual-presentation characters.

Use `all` by itself to select every variation position `evfmt` can format. For example, `--remove-prefer-bare=all` makes every format-supported position require an explicit selector. Use `none` by itself with `--set-*` policy flags to clear that policy set. For example, `--set-prefer-bare=none` stops keeping any character bare just because it was in `prefer-bare`; with the default `bare-as-text` set, bare ASCII and bare keycap-character forms then normalize to explicit text form and ordinary bare non-ASCII normalizes to explicit emoji form.

## ⚖️ License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## 🐛 Issues

Please report bugs, regressions, and feature requests in the [issue tracker](https://github.com/favonia/evfmt/issues).
