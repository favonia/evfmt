# Design Note: Policy

Read when: changing formatter policy predicates, policy defaults, or the mapping from policy sets to canonical selector states.

Defines: the public formatter policy over ambiguous selector slots.

Does not define: CLI flag grammar. That lives in [cli.markdown](cli.markdown). The typed `PolicyKeySet` API lives in [policy-key-set-api.markdown](policy-key-set-api.markdown).

## Policy Keys

Policy applies to selector slots that classification has already reduced to multiple reasonable states.

A policy key is a variation-sequence base plus one domain:

- ordinary, for non-keycap selector slots
- keycap-character

The domain is chosen by classification. Policy queries `PolicyKeySet` membership for that key.

## Predicates

Policy has two `PolicyKeySet` predicates:

- `prefer_bare`: policy keys for which bare spelling is canonical when bare can preserve the selected presentation
- `bare_as_text`: policy keys for which bare spelling is treated as text presentation when bare must be interpreted

## Decision Table

|                    | Bare as text                     | Bare not as text                  |
| ------------------ | -------------------------------- | --------------------------------- |
| Prefer bare        | Change text to bare; keep others | Change emoji to bare; keep others |
| Do not prefer bare | Change bare to text; keep others | Change bare to emoji; keep others |

Policy chooses among the reasonable states produced by classification. It does not apply to slots with exactly one reasonable state.

## Defaults

The default policy is:

```sh
--set-prefer-bare=ascii,emoji-defaults
--set-bare-as-text=text-defaults,keycap:variation-bases
```

These defaults must satisfy the exact-RGI constraints in [formatting.markdown](../core/formatting.markdown): default formatting preserves recognized exact RGI emoji input, and default policy chooses the RGI spelling when its chosen canonical form has emoji presentation and that spelling is reasonable.

`text-defaults` selects text-default bases only in non-keycap selector slots. Keycap selector slots are a separate policy-key domain, so their default text-side behavior is expressed explicitly with `keycap:variation-bases`.

This means:

- ASCII ambiguous bare forms stay bare.
- Emoji-default ambiguous bare forms stay bare.
- Text-default non-ASCII ambiguous bare forms in non-keycap selector slots default to text presentation.
- Bare keycap-character forms default to text presentation.
