# Design Note: Formatter Policy

Read when: changing formatter CLI behavior, policy defaults, warning semantics, or exit-code meaning.

Defines: the public policy surface, CLI modes, and exit codes.

## Policy predicates

`evfmt` exposes two policy predicates. Both use the [expression language](expression-language.markdown).

### `--prefer-bare-for=<expr>`

Selects bases whose bare form is preferred when both bare and explicit forms remain reasonable.

### `--treat-bare-as-text-for=<expr>`

Selects bases whose bare form is interpreted as text-like rather than emoji-like when policy must decide what bare means.

## Policy decision model

For an ambiguous standalone slot, the two predicates determine the canonical result:

|                     | Treating bare as text            | Not treating bare as text         |
| ------------------- | -------------------------------- | --------------------------------- |
| Preferring bare     | Change text to bare; keep others | Change emoji to bare; keep others |
| Not preferring bare | Change bare to text; keep others | Change bare to emoji; keep others |

You can also derive the predicates from the actions you want:

- `--treat-bare-as-text-for`: the union of "change text to bare" and "change bare to text"
- `--prefer-bare-for`: the union of "change text to bare" and "change emoji to bare"

## Recommended defaults

```sh
--prefer-bare-for='ascii'
--treat-bare-as-text-for='ascii'
```

This means:

- ASCII ambiguous bare forms stay bare
- non-ASCII ambiguous bare forms default to emoji presentation

With the default values `--treat-bare-as-text-for=ascii` and `--prefer-bare-for=ascii`, the resulting actions are:

|                                      | Treating bare as text (`ascii`) | Not treating bare as text (except `ascii`) |
| ------------------------------------ | ------------------------------- | ------------------------------------------ |
| Preferring bare (`ascii`)            | Change text to bare for ASCII   | Change emoji to bare for none              |
| Not preferring bare (except `ascii`) | Change bare to text for none    | Change bare to emoji for non-ASCII         |

## Formatting modes

### Format in place

```sh
evfmt README.md
evfmt docs/*.md
```

Default behavior rewrites files in place via atomic writes.

### Check mode

```sh
evfmt check README.md
```

No files are modified. Exit nonzero if any file would change.

### Stdin and stdout

`-` as a file operand means read from stdin and write to stdout. At most one `-` operand is allowed.

## Exit codes

- `0`: success, and in check mode no file would change
- `1`: `evfmt check` (or `--check`) found at least one file that would change
- `2`: usage error, decoding failure, I/O failure, or mixed success/failure across multiple file operands
