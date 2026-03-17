# Contributing to `evfmt`

First of all, thank you for your contribution.

## Security Reports

If you are reporting a security vulnerability, stop here. Do not use public issues or pull requests. See [Security Policy](https://github.com/favonia/evfmt/security/policy) and follow the steps there.

## Raise an Issue

If you are raising an issue, include a small reproducer when practical. Exact input text, command lines, and output are usually the fastest way to diagnose a formatter or parser problem.

## Make a Pull Request

If you have code ready, please make a pull request. Before you do:

1. Check the license.

   Roughly speaking, you agree to license your contribution under either MIT or Apache 2.0, and you assert that you have the right to do so. See [LICENSE-MIT](../LICENSE-MIT) and [LICENSE-APACHE](../LICENSE-APACHE) for the precise terms.

2. Test your code.

   Add or update tests for new features and bug fixes when practical. You can run the full test suite locally with `cargo test --workspace`.

3. Follow the coding style.

   We use `rustfmt`, `clippy`, `actionlint`, `yamlfmt`, and `mdformat` in CI. You can wait for GitHub Actions or run the relevant tools locally before opening the pull request.

4. Put documentation in the right place.

   Use [README.markdown](../README.markdown) for user-facing documentation. If you edit `docs/designs/`, start with [docs/designs/README.markdown](designs/README.markdown).

5. Open the pull request.

   Keep the summary focused on behavior and include test evidence when relevant. We loosely follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/), and the maintainer may normalize the pull-request title.

## Who’s in Charge

[favonia](mailto:favonia+github@gmail.com) is currently the sole maintainer and makes all final decisions.
