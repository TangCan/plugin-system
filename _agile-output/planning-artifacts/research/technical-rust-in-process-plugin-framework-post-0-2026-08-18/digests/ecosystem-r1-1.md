# Ecosystem digest r1

## Findings

- claim: Trusted publishing is OIDC-based short-lived tokens (~30 min) for CI; it does not replace the need for an API token on the first publish of a crate. Prerequisites: crate already on crates.io, you are an owner, repo on GitHub or GitLab (GitLab public beta).
  - source: https://crates.io/docs/trusted-publishing
  - publisher: crates.io
  - pub_date: undated live docs
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: Official announcement: first release must be published manually; trusted publishing is for later releases.
  - source: https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/
  - publisher: Rust Blog (crates.io team)
  - pub_date: 2025-07-11
  - accessed: 2026-08-18
  - confidence: high
  - class: ecosystem

- claim: Rust Project forge: trusted publishing is the recommended publish path for rust-lang crates (no long-lived GHA secret).
  - source: https://forge.rust-lang.org/infra/docs/trusted-publishing.html
  - publisher: Rust Forge
  - pub_date: undated live
  - accessed: 2026-08-18
  - confidence: high
  - class: ops

- claim: Cargo Book: publish is permanent; version never overwritten; yank ≠ delete; recommends dry-run, filled Cargo.toml metadata, changelog + git tag; first publish via API token + cargo login.
  - source: https://doc.rust-lang.org/cargo/reference/publishing.html
  - publisher: Cargo Book
  - pub_date: undated stable docs
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: cargo yank removes a version from the index only; data remains downloadable; new resolvers avoid yanked versions unless lockfile already pins them. For secrets, revoke immediately—yank does not stop existing lockfiles/downloads.
  - source: https://doc.rust-lang.org/stable/cargo/commands/cargo-yank.html
  - publisher: Cargo Book
  - pub_date: undated
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: Whole-crate deletion exists under narrow policy (<72h old OR single owner ∧ <1000 downloads per month of life ∧ no reverse deps); else contact team. Feb 2025 blog screenshot said 500/month; live policies say 1000.
  - source: https://crates.io/policies
  - publisher: crates.io
  - pub_date: undated live
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: README on crates.io is tied to the published .crate for that version; maintainers declined “update README without bumping version” because it would break artifact immutability.
  - source: https://github.com/rust-lang/crates.io/issues/1750
  - publisher: crates.io issue discussion
  - pub_date: thread from ~2019; stance reaffirmed in comments
  - accessed: 2026-08-18
  - confidence: medium
  - class: ops

- claim: docs.rs build customization is via [package.metadata.docs.rs]; crates.io owners can trigger docs.rs rebuilds from the version list without a new crate release.
  - source: https://docs.rs/about/builds
  - publisher: docs.rs
  - pub_date: undated
  - accessed: 2026-08-18
  - confidence: medium
  - class: ops

- claim: CI pattern in maintained tooling: OS matrix including Windows, fail-fast false, MSRV jobs, clippy with -D warnings, and cargo-hack for feature combinations (--mutually-exclusive-features when features conflict).
  - source: https://github.com/taiki-e/cargo-hack
  - publisher: taiki-e
  - pub_date: live README
  - accessed: 2026-08-18
  - confidence: high
  - class: ops

- claim: cargo-hack documents --feature-powerset and --mutually-exclusive-features when features conflict (so blind --all-features is insufficient).
  - source: https://github.com/taiki-e/cargo-hack
  - publisher: taiki-e
  - pub_date: live README
  - accessed: 2026-08-18
  - confidence: high
  - class: ops

## Leads

- Alpha-Omega trusted-publishing adoption write-up.
- crates.io Feb 2025: default API token expiry 90 days.
- Re-fetch docs.rs/about/metadata for exact features keys.

## Looked for, not found

- Quantitative 6–12 month maintainer burden surveys.
- Canonical community RC checklist as a single official Rust doc.
- Primary-doc mandate that Clippy must use -D warnings.
