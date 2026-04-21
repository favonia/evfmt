# Design Note: Formatter Policy

Read when: changing formatter CLI behavior, policy defaults, warning semantics, or exit-code meaning.

Defines: the public policy surface, CLI modes, and exit codes.

## Policy predicates

`evfmt` resolves standalone ambiguity through two policy predicates:

- the preferred-bare set
- the bare-as-text set

The CLI manages those predicates through ordered set operations, while the library can still construct them directly with the typed [charset API](charset-api.markdown).

### Preferred-bare set

Selects bases whose bare form is preferred when both bare and explicit forms remain reasonable.

CLI flags:

- `--set-prefer-bare=<charset[,charset]...>`
- `--add-prefer-bare=<charset[,charset]...>`
- `--remove-prefer-bare=<charset[,charset]...>`

### Bare-as-text set

Selects bases whose bare form is interpreted as text-like rather than emoji-like when policy must decide what bare means.

CLI flags:

- `--set-bare-as-text=<charset[,charset]...>`
- `--add-bare-as-text=<charset[,charset]...>`
- `--remove-bare-as-text=<charset[,charset]...>`

## Ordered CLI model

The CLI applies repeated set-operation flags strictly left to right within each domain.

- `set-*` replaces the current set
- `add-*` unions new items into the current set
- `remove-*` subtracts items from the current set

Character-set flags take comma-separated lists of:

- named presets such as `ascii`, `rights-marks`, or `arrows`
- `u(HEX)` code-point items
- single-character charset literals, optionally followed by a variation selector, such as `#`, `*`, `©︎` (U+00A9 U+FE0E), or `©️` (U+00A9 U+FE0F)

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

For an ambiguous standalone slot, the two predicates determine the canonical result:

|                     | Treating bare as text            | Not treating bare as text         |
| ------------------- | -------------------------------- | --------------------------------- |
| Preferring bare     | Change text to bare; keep others | Change emoji to bare; keep others |
| Not preferring bare | Change bare to text; keep others | Change bare to emoji; keep others |

You can also derive the predicates from the actions you want:

- bare-as-text: the union of "change text to bare" and "change bare to text"
- preferred-bare: the union of "change text to bare" and "change emoji to bare"

## Recommended defaults

```sh
--set-prefer-bare=ascii
--set-bare-as-text=ascii
--set-ignore=git,evfmt,hidden
```

This means:

- ASCII ambiguous bare forms stay bare
- non-ASCII ambiguous bare forms default to emoji presentation

With the default sets `bare-as-text = ascii` and `preferred-bare = ascii`, the resulting actions are:

|                                      | Treating bare as text (`ascii`) | Not treating bare as text (except `ascii`) |
| ------------------------------------ | ------------------------------- | ------------------------------------------ |
| Preferring bare (`ascii`)            | Change text to bare for ASCII   | Change emoji to bare for none              |
| Not preferring bare (except `ascii`) | Change bare to text for none    | Change bare to emoji for non-ASCII         |

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

`-` as a file operand means read from stdin and write to stdout. At most one `-` operand is allowed.

Use `--` only to end option parsing before file operands that look like options, such as `evfmt format -- --set-ignore`. Subcommand names are not file-name ambiguities once `format` or `check` has been selected; for example, `evfmt format check` formats a file named `check`.

## Exit codes

- `0`: success, and in check mode no file would change
- `1`: `evfmt check` found at least one file that would change
- `2`: usage error, decoding failure, I/O failure, or mixed success/failure across multiple file operands
