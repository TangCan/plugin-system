# Architecture digest r2

Round 2 follow-ups: libloading unload API; hot-lib-reloader reload vs production dispose; Windows file lock after FreeLibrary.

Stop: **coverage** on the unload pattern. **Novelty exhaustion** on a fresh official libloading Windows file-lock post-mortem (none; OS contract + Drop docs suffice).

## Findings

- claim: libloading 0.9 `Library::close(self)` unloads; it may be a no-op depending on open flags / platform. Call it only to observe unload errors. Otherwise `Drop` closes the library and **ignores** unload errors. If `close` errors, underlying data structures may leak. `Library::new` safety: init routines run on load and termination routines may run on unload.
  - source: https://docs.rs/libloading/latest/libloading/struct.Library.html
  - publisher: docs.rs (libloading 0.9.0)
  - pub_date: live crate docs
  - accessed: 2026-08-18
  - confidence: high
  - class: pattern

- claim: On Windows, libloading `Drop` calls `FreeLibrary` and ignores the BOOL; `close` maps a zero return to `Error::FreeLibrary` then `mem::forget`s so Drop does not retry.
  - source: https://github.com/nagisa/rust_libloading/blob/master/src/os/windows/mod.rs
  - publisher: nagisa / rust_libloading
  - pub_date: live source (master as of fetch)
  - accessed: 2026-08-18
  - confidence: high
  - class: pattern

- claim: Microsoft contract: `FreeLibrary` decrements the per-process module refcount; the module unloads when the count reaches zero. A successful `FreeLibrary` does not by itself prove the file is unlocked or unmapped if other loads/pins remain.
  - source: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary
  - publisher: Microsoft Learn
  - pub_date: undated API contract
  - accessed: 2026-08-18
  - confidence: high
  - class: pattern

- claim: Named production-oriented pattern remains: logical dispose, then drop (or `close`) the `Library` so the OS can `dlclose`/`FreeLibrary`, then load a new instance later. There is no soundness case in UCG or libloading docs for a combined `reload()` that keeps outstanding refs. hot-lib-reloader's `LibReloader::update` is a **dev-time** watch/unload/load helper with the same OS constraints, plus documented limits (no signature change, care with types/globals).
  - source: https://docs.rs/hot-lib-reloader/latest/hot_lib_reloader/struct.LibReloader.html
  - publisher: docs.rs / rksm
  - pub_date: live crate docs
  - accessed: 2026-08-18
  - confidence: high
  - class: pattern

- claim: abi_stable still documents “plugin system (without support for unloading)” — a stable-layout Rust-to-Rust dylib path that is a **different** product bet from physical unload.
  - source: https://docs.rs/abi_stable/latest/abi_stable/
  - publisher: docs.rs (abi_stable 0.11.3)
  - pub_date: live crate docs
  - accessed: 2026-08-18
  - confidence: high
  - class: pattern

## Leads

None that change the decision. Windows file-lock anecdotes (SSL statics, GET_MODULE_HANDLE_EX_FLAG_PIN) remain secondary; primary contract is Microsoft + libloading.

## Looked for, not found

- Official libloading maintainer post-mortem dedicated to Windows DLL file locks after Drop.
- Dual independent 2025–2026 Wasmtime pooling vs Extism reset benchmarks.
