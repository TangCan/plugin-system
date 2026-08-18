# Landscape digest r2

Round 2 follow-ups: WASI 0.3 pin churn, Fidius as a peer framework, hot-lib-reloader as a native-reload tool.

Stop: **coverage** — plan questions on maturity/churn and peer approaches are answered. Remaining gaps (download rankings, CM vs Extism vs native numbers) are absence-of-evidence, not new leads.

## Findings

- claim: WASI 0.3.0 was ratified 2026-06-11; async (`future`, `stream`, `async func`) moves into the Component Model canonical ABI and `wasi:io` pollables/streams are replaced. WASI.dev says Wasmtime 45 runs the latest RC and Wasmtime 46 will ship WASI 0.3.0 with Component Model Async enabled by default. Hosts must pin the same WIT version across Wasmtime / wit-bindgen / jco; mismatches surface as confusing type errors at instantiation. Target pin named in current toolchain docs is still `0.3.0` vs lingering `0.3.0-rc-2026-03-15`.
  - source: https://wasi.dev/releases/wasi-p3
  - publisher: WASI.dev
  - pub_date: 2026-06-11 (release); page current as of fetch
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: Bytecode Alliance announcement agrees: spec is stable 0.3.0 with continued 0.3.x patches; Wasmtime 45 = RC runtime, 46 = 0.3.0 + async default.
  - source: https://bytecodealliance.org/articles/WASI-0.3
  - publisher: Bytecode Alliance
  - pub_date: 2026-06-11
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: Component Model Rust guest tutorial still shows `wasi:cli/run@0.3.0-rc-2026-03-15`. As of WASI 0.3.0 (2026-06-11), wit-bindgen 0.58 and Wasmtime 45 still ship that RC WIT; pinning guests to published `0.3.0` produces runtime export-name errors until those tools refresh. Use the RC pin until then. Tracking: wit-bindgen #1554.
  - source: https://component-model.bytecodealliance.org/language-support/creating-runnable-components/rust.html
  - publisher: Bytecode Alliance (Component Model book)
  - pub_date: undated live (content dated against 2026-06-11 release)
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: Fidius is a current in-process Rust plugin product: `#[fidius::interface]` / `#[fidius::plugin]` generate a stable C ABI cdylib with hash-checked entry points; `fidius-host` loads a typed proxy. Same macros can emit a wasmtime WebAssembly component (`fidius-guest`, `wasm32-wasip2`) with deny-all sandbox and a package.toml capability allow-list (`http` requires host egress policy). Native plugins keep host authority; WASM is the sandbox tier.
  - source: https://github.com/colliery-io/fidius
  - publisher: colliery-io / Fidius
  - pub_date: live README (commit snapshot fetched 2026-08-18)
  - accessed: 2026-08-18
  - confidence: high
  - class: landscape

- claim: hot-lib-reloader is explicitly a **development** live-reload tool on top of libloading (watch dylib, unload, load again). Documented limits: no signature changes, type changes need care, no generics, global state in reloadable code is unsafe. Not a production plugin ABI or a substitute for a host-owned dispose→drop Library→load cycle.
  - source: https://docs.rs/hot-lib-reloader/latest/hot_lib_reloader/index.html
  - publisher: docs.rs / rksm
  - pub_date: live crate docs
  - accessed: 2026-08-18
  - confidence: high
  - class: landscape

## Leads

- wit-bindgen #1554 for the 0.3.0 WIT refresh (versions claim; monthly).
- No further landscape entities required for the decision.

## Looked for, not found

- Independent download/ranking of Fidius vs Extism vs generic plugin crates.
- Production post-mortem of Fidius WASM deny-all in a named shipped product.
