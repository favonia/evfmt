# evfmt

`evfmt` normalizes text/emoji variation selectors in your files.

`evfmt` is ready for normal formatter use. The Rust library APIs and some advanced CLI use are still experimental. `evfmt` follows [Cargo's SemVer compatibility conventions](https://doc.rust-lang.org/cargo/reference/semver.html).

It is an opinionated formatter for Unicode variation selectors:

- chooses a deterministic bare, text, or emoji form for characters with variation sequences
- removes stray selectors in unsupported positions
- normalizes multi-character emoji sequences to fully qualified forms
- respects `.gitignore` and `.evfmtignore`

Install from [crates.io](https://crates.io/crates/evfmt) with:

```sh
cargo install evfmt --locked
```

Add the library to another crate with `cargo add`:

```sh
cargo add evfmt
```

Minimum supported Rust version (MSRV): Rust 1.88.

From a local checkout, you can also run:

```sh
cargo install --path evfmt
```

Quick start:

```sh
evfmt format README.markdown docs/*.markdown
evfmt check README.markdown
printf '%b' 'Love \u2764' | evfmt format
```

Use `-` as an explicit stdin operand when mixing stdin with files; `./-` refers to a file literally named `-`.

For full documentation, see the [repository README](https://github.com/favonia/evfmt).
