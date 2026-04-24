# Design Note: Formatter Policy

Read when: changing formatter CLI behavior, policy defaults, warning semantics, or exit-code meaning.

Defines: the public policy surface, CLI modes, and exit codes.

## Policy predicates

`evfmt` resolves policy ambiguity through two policy predicates:

- the preferred-bare set
- the bare-as-text set

The CLI manages those predicates through ordered set operations, while the library can still construct them directly with the typed [VariationSet API](variation-set-api.markdown).

### Preferred-bare set

Selects variation positions whose bare form is preferred when both bare and explicit forms remain reasonable.

CLI flags:

- `--set-prefer-bare=<set[,set]...>`
- `--add-prefer-bare=<set[,set]...>`
- `--remove-prefer-bare=<set[,set]...>`

### Bare-as-text set

Selects variation positions whose bare form is interpreted as text-like rather than emoji-like when policy must decide what bare means.

CLI flags:

- `--set-bare-as-text=<set[,set]...>`
- `--add-bare-as-text=<set[,set]...>`
- `--remove-bare-as-text=<set[,set]...>`

## Ordered CLI model

The CLI applies repeated set-operation flags strictly left to right within each domain.

- `set-*` replaces the current set
- `add-*` unions new items into the current set
- `remove-*` subtracts items from the current set

Policy set flags take comma-separated lists of:

- named presets such as `ascii`, `text-defaults`, `emoji-defaults`, `rights-marks`, `arrows`, `keycap-chars`, `non-keycap-chars`, or `keycap-emojis`
- ordinary `u(HEX)` code-point items
- single-character literals, optionally followed by a variation selector, such as `#`, `*`, `©︎` (U+00A9 U+FE0E), or `©️` (U+00A9 U+FE0F)

`all` selects every character `evfmt` can format and works with any policy set-operation flag. `none` clears a policy set and works only with `--set-*` policy flags. Unknown preset-like items are errors and should offer nearby suggestions when practical.

Ignore filtering uses the same ordered model with these labels:

- `git`
- `evfmt`
- `hidden`

The ignore flags are:

- `--set-ignore=<filter[,filter]...>`
- `--add-ignore=<filter[,filter]...>`
- `--remove-ignore=<filter[,filter]...>`

`all` selects every ignore filter and works with any ignore set-operation flag. `none` disables all ignore filters and works only with `--set-ignore`.

## Policy decision model

For an ambiguous policy slot, the two predicates determine the canonical result:

|                     | Treating bare as text            | Not treating bare as text         |
| ------------------- | -------------------------------- | --------------------------------- |
| Preferring bare     | Change text to bare; keep others | Change emoji to bare; keep others |
| Not preferring bare | Change bare to text; keep others | Change bare to emoji; keep others |

You can also derive the predicates from the actions you want:

- bare-as-text: the union of "change text to bare" and "change bare to text"
- preferred-bare: the union of "change text to bare" and "change emoji to bare"

## Recommended defaults

```sh
--set-prefer-bare=ascii,emoji-defaults
--set-bare-as-text=ascii,keycap-chars
--set-ignore=git,evfmt,hidden
```

This means:

- ASCII ambiguous bare forms stay bare
- emoji-default ambiguous bare forms stay bare
- text-default non-ASCII ambiguous bare forms default to emoji presentation
- bare keycap-character forms default to text presentation

With the default sets `bare-as-text = ascii,keycap-chars` and `preferred-bare = ascii,emoji-defaults`, the resulting actions are:

|                                                     | Treating bare as text (`ascii,keycap-chars`)       | Not treating bare as text (except `ascii,keycap-chars`)   |
| --------------------------------------------------- | -------------------------------------------------- | --------------------------------------------------------- |
| Preferring bare (`ascii,emoji-defaults`)            | Change text to bare for ASCII                      | Change emoji to bare for emoji-default positions          |
| Not preferring bare (except `ascii,emoji-defaults`) | Change bare to text for keycap-character positions | Change bare to emoji for text-default non-ASCII positions |

## Formatting modes

### Format in place

```sh
evfmt format README.md
evfmt format docs/*.md
```

The `format` subcommand rewrites files in place via atomic writes.

### Check mode

```sh
evfmt check README.md
```

No files are modified. Exit nonzero if any file would change.

### Stdin and stdout

With no file operands, `format` reads stdin and writes formatted text to stdout; `check` reads stdin and reports whether changes would be needed.

`-` as a file operand means read from stdin and, in format mode, write formatted text to stdout at that operand position. A path such as `./-` refers to a file literally named `-`.

Repeated `-` operands are allowed and read the same stdin stream from its current position. With piped input, the first `-` normally consumes the stream and later `-` operands see EOF.

Use `--` only to end option parsing before file operands that look like options, such as `evfmt format -- --set-ignore`. Subcommand names are not file-name ambiguities once `format` or `check` has been selected; for example, `evfmt format check` formats a file named `check`.

## Exit codes

- `0`: success, and in check mode no file would change
- `1`: `evfmt check` found at least one file that would change
- `2`: usage error, decoding failure, I/O failure, or mixed success/failure across multiple file operands
