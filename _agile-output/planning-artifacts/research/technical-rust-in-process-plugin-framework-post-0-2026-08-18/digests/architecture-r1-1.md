# Architecture digest r1

## Findings

- claim: For dlclose to be sound you must clear all external refs to dylib data/code, all outbound function pointers, all cross-DSO symbol links, and all threads owned by the dylib; TLS with non-trivial destructors is separately called out as blocking sound unload (macOS often refuses unload; Linux may allow unload but still leave no sound path).
  - source: https://github.com/rust-lang/unsafe-code-guidelines/issues/526
  - publisher: rust-lang (Unsafe Code Guidelines)
  - pub_date: 2024-08-13
  - accessed: 2026-08-18
  - confidence: high
  - class: failure

- claim: Named production-oriented pattern for native plugins that need physical unload without a dedicated reload() API: logical dispose then drop the loader handle so the OS unloads (dlclose/FreeLibrary), then load a new instance later; UCG treats unload as opsem UB if any outstanding use remains.
  - source: https://github.com/rust-lang/unsafe-code-guidelines/issues/526
  - publisher: rust-lang
  - pub_date: 2024-08
  - accessed: 2026-08-18
  - confidence: medium
  - class: pattern

- claim: Historical TLS+dlclose crash on macOS when TLS dtors outlive the mapping; later dyld marks TLV dylibs never-unload (success from dlclose can mean no real unmap).
  - source: https://github.com/rust-lang/rust/issues/28794
  - publisher: rust-lang
  - pub_date: 2015–2018 comments
  - accessed: 2026-08-18
  - confidence: medium
  - class: failure

- claim: Windows FreeLibrary only decrements a per-process module refcount; unload (and thus file replace) happens only when count hits zero—or process exit; extra LoadLibrary / pinned modules / deps keep the DLL mapped and often file-locked despite a successful FreeLibrary return.
  - source: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary
  - publisher: Microsoft Learn
  - pub_date: undated API contract
  - accessed: 2026-08-18
  - confidence: high
  - class: pattern

- claim: Diagnostic pattern when unload “succeeds” but module stays: unmatched load/refcount (e.g. side-effect loads); Application Verifier / !avrf -dlls stack history is the documented investigation path.
  - source: https://devblogs.microsoft.com/oldnewthing/20170915-00/?p=97035
  - publisher: Microsoft (Raymond Chen / Old New Thing)
  - pub_date: 2017-09-15
  - accessed: 2026-08-18
  - confidence: medium
  - class: failure

- claim: abi_stable named use case: Rust-to-Rust dynamic libs with load-time layout checks, different rustc versions OK; explicitly “Creating a plugin system (without support for unloading)”.
  - source: https://docs.rs/abi_stable/latest/abi_stable/
  - publisher: docs.rs (crate abi_stable 0.11.3)
  - pub_date: crate docs current as of fetch
  - accessed: 2026-08-18
  - confidence: high
  - class: pattern

- claim: Extism maintainers (thread still open; last activity 2025-09-21): no Component Model support now; optional WASIp2 interest separate; CM seen as wasmtime-narrow, conflicting with multi-runtime Extism hosts; plugins use shared bytes-in/bytes-out signatures.
  - source: https://github.com/extism/extism/issues/666
  - publisher: Extism project
  - pub_date: 2024-01-26 opened, updated 2025-09-21
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: Extism plugins and Wasmtime Component/bindgen guests are different ABIs/contracts—hosts choosing one do not get the other.
  - source: https://github.com/extism/extism/issues/666
  - publisher: Extism
  - pub_date: 2024–2025
  - accessed: 2026-08-18
  - confidence: high
  - class: versions/compat

- claim: Extism Plugin::reset invalidates allocated guest memory/state; C API exposes extism_plugin_free / extism_plugin_reset—instance teardown/reuse without native dlclose.
  - source: https://docs.rs/extism/latest/extism/struct.Plugin.html
  - publisher: Extism / docs.rs
  - pub_date: current
  - accessed: 2026-08-18
  - confidence: high
  - class: pattern

- claim: Wasmtime pooling pre-allocates memories/tables/instances; drop Store returns slots to pool; PoolingAllocationConfig includes component-instance knobs; not default—tuned mainly for Unix.
  - source: https://docs.wasmtime.dev/api/wasmtime/struct.PoolingAllocationConfig.html
  - publisher: Bytecode Alliance
  - pub_date: current
  - accessed: 2026-08-18
  - confidence: high
  - class: pattern

## Leads

- libloading Library Drop/dlclose crate docs.
- Zed extension system as production CM plugin case.
- Extism JS Plugin.close vs Rust free.

## Looked for, not found

- Fresh official libloading post-mortem on Windows DLL file locks.
- Dual independent 2025–2026 pooling benchmarks.
- Extism docs claiming CM coexistence (explicit non-support instead).
