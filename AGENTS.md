<!-- bmad:context -->
<!-- Verified 2026-08-18 against 453cb6e. Managed by bmad-project-context; edits inside this block are replaced on refresh. Keep anything you want preserved outside the markers. -->

## plugin-system

Rust workspace: in-process plugin framework `plugctx` plus optional C ABI / WASM adapters. Planning and current native-unload decisions live in `_agile-output/planning-artifacts/` (PRD, architecture, epics). API contract deviations are in `docs/api-freeze.md`. Sprint tracking is `_agile-output/implementation-artifacts/`.

## Where things are

- Coding behavior (Karpathy): `.cursor/rules/karpathy-guidelines.mdc`
- Native hot-plug / physical unload: `_agile-output/planning-artifacts/architecture.md` (AD-1–AD-3)
- Public API freeze: `docs/api-freeze.md`
- Feature flags and CI matrix: `docs/feature-matrix.md`, `docs/testing.md`
- Publish surface: `docs/publishing.md` — only `plugctx` and `plugctx-derive` are publishable
- WIT guest (not a workspace member): `guests/wit-sample/`

## Running and verifying

- Full regression is `./scripts/ci-test.sh` from the repo root. Do not use `cargo test --all-features` as the sole gate: `thread-safe` conflicts with tests gated `#![cfg(not(feature = "thread-safe"))]`.
- Before `cargo run -p plugin-host`, run `cargo build -p hello_plugin -p echo_plugin`; the host crate does not build those cdylibs. Host unit tests auto-build if the `.so` is missing.

## Conventions that differ from defaults

- Native `dynamic-native` unload must Drop `libloading::Library` after logical unregister (architecture AD-1). WASM unload stays instance `close`/`free` (FR26).
- Do not add `reload()`; hot-plug is load → use → dispose → load. New public Error variants or core signatures must update `docs/api-freeze.md`.
- Keep `plugctx` `default = []`; do not pull `extism` / `wasmtime` / `libloading` into the default graph. Example apps stay `publish = false` and must not add those runtimes to `plugctx` defaults.
- Core `Error` variants are unit-like; `get` / `get_trait` return `Option`, not `Error::ServiceNotFound`.
- User-facing docs in 中文; code identifiers in English.

<!-- /bmad:context -->
