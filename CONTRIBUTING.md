# Contributing

1. Fork, branch from `main`, open a PR. One green workspace per PR.
2. Before pushing:
   ```sh
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings     # CI gate
   cargo test  --workspace
   ```
   `cargo clippy ... -W clippy::pedantic` is an advisory local pass — clean it up
   when practical, but it does not gate CI.
3. Honor the rules in [`CLAUDE.md`](CLAUDE.md): the paper is the spec; `Rig`
   is a semiring (never an external `Ring`); integer SNF stays custom; every
   change is green; no per-PR CHANGELOG edits; a test is a claim; one canonical
   integration test per published crate — a PR that adds a
   `pub struct|enum|trait|type` under a crate's `src` edits that crate's
   `tests/canonical.rs` header in the same PR.
4. Conventional-commit subjects (`feat:`, `fix:`, `docs:`, `chore:`), ≤ 72 chars,
   imperative mood.

Work is tracked as GitHub issues, not in-repo trackers.
