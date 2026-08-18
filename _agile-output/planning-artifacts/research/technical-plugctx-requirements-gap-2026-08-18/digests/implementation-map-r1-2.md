# Digest: 实现对照 — r1-2

**Decision:** 用代码、Cargo features、验收测试核对需求能力是否落地。
**Accessed:** 2026-08-18

## Claims

- {claim: "plugctx 公开模块覆盖 context/plugin/service/event/effect/interceptor/error，以及 feature 门控 async/dynamic-*/stages。", source: "crates/plugctx/src/lib.rs", publisher: "plugin-system source", pub_date: "2026-08-18 (HEAD 453cb6e)", accessed: "2026-08-18", confidence: "high", class: "implementation"}
- {claim: "Cargo features：async、parallel、thread-safe、dynamic-native、dynamic-wasm、dynamic-wasm-component、tracing、stages；default=[]。", source: "crates/plugctx/Cargo.toml [features]", publisher: "plugin-system source", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "versions/compatibility"}
- {claim: "plugin_async / start_async / on_async / emit_parallel / get_trait / isolate 均在 context.rs 实现。", source: "crates/plugctx/src/context.rs", publisher: "plugin-system source", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "implementation"}
- {claim: "Feature 矩阵宣称大纲 §2.4 项（async/parallel/dynamic-native/thread-safe/derive）已交付，并增补 wasm/component/tracing/stages。", source: "docs/feature-matrix.md", publisher: "plugin-system docs", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "implementation"}
- {claim: "CHANGELOG 将 Epic 1–2 标为 0.1.0 核心、扩展 feature+derive 标为 0.2.0 能力清单；workspace version 仍为 0.1.0。", source: "CHANGELOG.md; Cargo.toml workspace.package.version", publisher: "plugin-system docs", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "versions/compatibility"}
- {claim: "api-freeze 记录 Error 单元变体、get 返回 Option 而非 ServiceNotFound、isolate 返回 Result 等刻意偏差。", source: "docs/api-freeze.md", publisher: "plugin-system docs", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "api-deviation"}
- {claim: "plugctx Cargo.toml 无 repository 字段；注释指向 publishing.md 等价项。", source: "crates/plugctx/Cargo.toml L10", publisher: "plugin-system source", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "release"}
- {claim: "仓库根不存在 .github/workflows；ci-test.sh 含 fmt/test/bench --no-run/doc/扩展矩阵，不含 clippy。", source: "scripts/ci-test.sh; glob .github", publisher: "plugin-system source", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "nfr"}
- {claim: "示例仅有 combo/async-start/stages-lifecycle 与 derive/wasm/component；无 CLI/Web/游戏应用示例。", source: "README.md; examples/README.md", publisher: "plugin-system docs", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "roadmap"}
- {claim: "Context 无 Drop impl；§6.6 写明忘记 dispose 时 cleanup 可能不跑。", source: "crates/plugctx/src/context.rs; docs/requirements/6. API 设计概览.md §6.6", publisher: "plugin-system", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "implementation"}
- {claim: "ContextData 按职责拆分 ADR 状态为 Deferred。", source: "docs/adr-contextdata-split.md", publisher: "plugin-system docs", pub_date: "2026-08-17", accessed: "2026-08-18", confidence: "high", class: "deferred"}
- {claim: "crates.io GET /api/v1/crates/plugctx 与 plugctx-derive 均为 404。", source: "https://crates.io/api/v1/crates/plugctx", publisher: "crates.io", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "release"}

## Leads
- 独立 crate plugctx-async/dynamic 是否被 feature 完全替代。
- 热插拔语义。

## Not found
- 工作区 members 无 plugctx-async、plugctx-dynamic。
- docs/ 下无用户指南文件。
