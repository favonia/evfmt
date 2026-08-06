# Design Note: CLI

Read when: changing command-line modes, option grammar, ignore filtering, stdin/stdout behavior, or exit codes.

Defines: the command-line behavior of `evfmt`.

Does not define: policy semantics. Those live in [policy.markdown](policy.markdown).

## Ordered Set Operations

Set-operation flags apply left to right within each option domain:

- `set-*` replaces the current set
- `add-*` unions items into the current set
- `remove-*` subtracts items from the current set

## Policy Flags

Policy flags configure the predicates defined in [policy.markdown](policy.markdown):

- `--set-prefer-bare=<set[,set]...>`
- `--add-prefer-bare=<set[,set]...>`
- `--remove-prefer-bare=<set[,set]...>`
- `--set-bare-as-text=<set[,set]...>`
- `--add-bare-as-text=<set[,set]...>`
- `--remove-bare-as-text=<set[,set]...>`

Policy set items are comma-separated:

- named sets such as `ascii`, `text-defaults`, `emoji-defaults`, `variation-bases`, `keycap:rgi`, `keycap:text-defaults`, `keycap:emoji-defaults`, or `keycap:variation-bases`
- non-keycap `u(HEX)` code-point items
- keycap-character `keycap:u(HEX)` code-point items
- single-character literals, optionally followed by a variation selector, such as `#`, `*`, `©︎`, or `©️`
- keycap-character literals prefixed with `keycap:`, such as `keycap:#`

`all` selects every policy key. `none` clears a policy set and is valid only with `--set-*` policy flags.

Unknown preset-like items are errors and should offer nearby suggestions when practical.

## Ignore Flags

Ignore filtering uses the same ordered set-operation model.

The ignore labels are:

- `git`
- `evfmt`
- `hidden`

The ignore flags are:

- `--set-ignore=<filter[,filter]...>`
- `--add-ignore=<filter[,filter]...>`
- `--remove-ignore=<filter[,filter]...>`

The default ignore set is `git,evfmt,hidden`.

`all` selects every ignore filter. `none` disables all ignore filters and is valid only with `--set-ignore`.

## Modes

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

With no file operands, `format` reads stdin and writes formatted text to stdout. With no file operands, `check` reads stdin and reports whether changes would be needed.

`-` as a file operand means read from stdin and, in format mode, write formatted text to stdout at that operand position. A path such as `./-` refers to a file literally named `-`.

Repeated `-` operands are allowed and read the same stdin stream from its current position. With piped input, the first `-` normally consumes the stream and later `-` operands see EOF.

Use `--` only to end option parsing before file operands that look like options, such as `evfmt format -- --set-ignore`. Subcommand names are not file-name ambiguities once `format` or `check` has been selected; for example, `evfmt format check` formats a file named `check`.

## Exit Codes

- `0`: success, and in check mode no file would change
- `1`: `evfmt check` found at least one file that would change
- `2`: usage error, decoding failure, I/O failure, or mixed success/failure across multiple file operands
