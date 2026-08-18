# Ecosystem digest r2

Round 2 follow-ups: cargo-hack exclusive-feature flags; docs.rs `[package.metadata.docs.rs]` keys; docs.rs rebuild without a crate bump; Trusted Publishing setup (already published first crate).

Stop: **coverage**. No 6–12 month maintainer-burden survey appeared.

## Findings

- claim: cargo-hack `--feature-powerset` can take `--mutually-exclusive-features a,b` (repeatable groups) so conflicting features are never combined. `--exclude-all-features` skips the implicit `--all-features` run that `--each-feature` otherwise adds when multiple features exist. `--depth 1` makes powerset equivalent to each-feature.
  - source: https://github.com/taiki-e/cargo-hack/blob/master/README.md
  - publisher: taiki-e
  - pub_date: live README
  - accessed: 2026-08-18
  - confidence: high
  - class: ops

- claim: docs.rs default is **not** `--all-features`. Customize via `[package.metadata.docs.rs]`: `features = [...]`, `all-features` (default false), `no-default-features`, `default-target`, `targets`, `rustdoc-args`, `cargo-args`. Workspace-level `workspace.metadata.docs.rs` is not supported (docs.rs #2226 / cargo metadata inherit gap).
  - source: https://docs.rs/about/metadata
  - publisher: docs.rs
  - pub_date: undated live
  - accessed: 2026-08-18
  - confidence: high
  - class: ops

- claim: Independent confirmation of the metadata keys from docs.rs source: `Metadata` deserializes `features`, `all_features`, `no_default_features`, targets, rustc/rustdoc/cargo args; default docs.rs build uses default features unless metadata says otherwise.
  - source: https://github.com/rust-lang/docs.rs/blob/e7a97fd4/crates/metadata/lib.rs
  - publisher: rust-lang / docs.rs
  - pub_date: snapshot e7a97fd4 (fetched 2026-08-18)
  - accessed: 2026-08-18
  - confidence: high
  - class: ops

- claim: Crate owners can trigger docs.rs rebuilds from the crates.io version list without publishing a new version (failed builds or new docs.rs features). Announced 2025-07-11.
  - source: https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/
  - publisher: Rust Blog / crates.io team
  - pub_date: 2025-07-11
  - accessed: 2026-08-18
  - confidence: high
  - class: ops

- claim: Trusted Publishing (live docs, accessed 2026-08-18): first crate publish still needs an API token; later CI uses OIDC (`id-token: write` + `rust-lang/crates-io-auth-action`) exchanging for a token that expires after 30 minutes. GitLab is public beta. Both methods can coexist during migration. Requires owner + GitHub/GitLab repo.
  - source: https://crates.io/docs/trusted-publishing
  - publisher: crates.io
  - pub_date: undated live (feature announced 2025-07-11)
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: Cargo Book still: publish is permanent; yank ≠ delete; recommend changelog + git tag; dry-run before publish.
  - source: https://doc.rust-lang.org/cargo/reference/publishing.html
  - publisher: Cargo Book
  - pub_date: undated stable docs
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

## Leads

- Alpha-Omega adoption write-up still optional; not load-bearing for the decision.
- crates.io API token default expiry 90 days (Feb 2025 blog) — operational hygiene, not a next-feature.

## Looked for, not found

- Official Rust doc that Clippy **must** use `-D warnings` (community pattern only).
- Single canonical “RC checklist” from the Rust project.
