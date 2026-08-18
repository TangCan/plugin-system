# Landscape digest r1

## Findings

- claim: Wasmtime is documented as the reference implementation of the WebAssembly Component Model; hosts can run WASI CLI/HTTP worlds, and WASI 0.3 (`async func`, `stream`, `future`) is available in Wasmtime 43+ via `-Sp3` and `-W component-model-async=y`. Custom-export invocation exists as of Wasmtime 33.0.0. Toolchains must pin the same WASI 0.3 WIT version or instantiation fails with confusing type errors.
  - source: https://component-model.bytecodealliance.org/running-components/wasmtime.html
  - publisher: Bytecode Alliance (Component Model docs)
  - pub_date: undated page (content references Wasmtime 43 / WASI 0.3 and a `0.3.0-rc-2026-03-15` WIT pin)
  - accessed: 2026-08-18
  - confidence: high
  - class: landscape

- claim: A 2025-08 practitioner guide presents WebAssembly Components + WIT + Wasmtime as a newly practical alternative to native `dlopen`/shared-library plugins for security (sandbox), interface definition (WIT worlds), and binary compatibility (language-agnostic component ABI); it builds a Rust host calling C and JavaScript guest plugins and states Wasmtime then had the best Component Model support among common runtimes.
  - source: https://tartanllama.xyz/posts/wasm-plugins
  - publisher: Sy Brand (personal/technical blog)
  - pub_date: 2025-08-05
  - accessed: 2026-08-18
  - confidence: high
  - class: landscape

- claim: Extism positions itself as a cross-language WASM plug-in ecosystem (Host SDKs + PDKs): guests are `.wasm` modules, hosts load/call them in-process via an SDK, with sandboxing and host-defined capabilities rather than native shared-library loading as the primary model.
  - source: https://extism.org/docs/concepts/plug-in-system/
  - publisher: Extism / Dylibso
  - pub_date: undated docs page
  - accessed: 2026-08-18
  - confidence: high
  - class: landscape

- claim: Extism maintainers still treat Component Model support as out of scope for the near term: they prioritize a portable, multi-runtime host story (Wasmtime is only one backend among others) and a bytes-oriented Extism ABI; they intend optional WASI Preview 2 where feasible *without* adopting the Component Model “right now.” Thread remains open and was updated 2025-09-21.
  - source: https://github.com/extism/extism/issues/666
  - publisher: Extism project (GitHub)
  - pub_date: 2024-01-26 (opened); maintainer stance restated 2024-07-25; last activity 2025-09-21
  - accessed: 2026-08-18
  - confidence: high
  - class: landscape

- claim: crates.io Trusted Publishing shipped (RFC 3691): CI can publish without long-lived registry tokens by exchanging platform OIDC identity for a short-lived publish token (docs: expires after 30 minutes). Initial crate publish still requires a manual API-token publish; owners then configure a trusted publisher (repo/workflow) on crates.io.
  - source: https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/
  - publisher: Rust Blog / crates.io team
  - pub_date: 2025-07-11
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: Official Trusted Publishing docs confirm GitHub Actions as supported and GitLab CI/CD as public beta; setup uses `id-token: write` + `rust-lang/crates-io-auth-action` (GitHub) or OIDC JWT exchange to `https://crates.io/api/v1/trusted_publishing/tokens` (GitLab). Both publishing methods can coexist during migration from static API tokens.
  - source: https://crates.io/docs/trusted-publishing
  - publisher: crates.io
  - pub_date: undated docs (content aligns with 2025-07 announcement; docs also mention GitLab beta)
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

## Leads

- Fidius / fidius-host; hot-lib-reloader + libloading; Zed extensions (WIT/CM); wit-bindgen / WASI 0.3 pin churn.
- Native in-process DI plugin crate leaders; Extism WASIp2 vs #666; dlclose practicality (UCG #526).

## Looked for, not found

- crates.io download leaders for plugin/DI crates.
- ≤3-month AI-adjacent primary posts on this topic.
- Performance numbers Component Model vs Extism vs native.
