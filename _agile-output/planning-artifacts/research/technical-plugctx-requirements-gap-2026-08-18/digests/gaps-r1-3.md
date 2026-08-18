# Digest: 缺口与风险 — r1-3

**Decision:** 分类未实现 / 部分实现 / 刻意偏离 / 文档不一致。
**Accessed:** 2026-08-18

## Claims

- {claim: "设计功能面（内核+全部列出的 Cargo feature）已有实现与验收测试；缺口主要在发布、托管 CI、用户指南、场景示例，而非缺一整块运行时模块。", source: "feature-matrix.md + lib.rs + acceptance_story_*", publisher: "plugin-system", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "gap"}
- {claim: "「热插拔」在引言是目标；native 路径明确逻辑卸载≠dlclose，无法作为真正卸载映射后再加载新 .so 的热替换。WASM 路径有实例 close。", source: "1. 引言.md; 4. 扩展模块设计.md §4.3", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "gap"}
- {claim: "独立 crate plugctx-async / plugctx-dynamic 未创建；能力并入 plugctx features。属架构形态偏离，非能力缺失。", source: "2. 总体架构.md §2.2.2; Cargo.toml members", publisher: "plugin-system", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "architecture-deviation"}
- {claim: "CI 脚本存在但 GitHub Actions 工作流文件不存在；clippy 与 cargo test --all-features 未按 §8.9 原文落地（feature-matrix 说明 thread-safe 与部分默认同步测试互斥）。", source: "8. 测试策略.md §8.9; scripts/ci-test.sh; feature-matrix.md", publisher: "plugin-system", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "nfr-gap"}
- {claim: "路线图阶段5：用户指南、CLI/Web/游戏示例、crates.io 正式发布未完成。", source: "9. 实施路线图.md §9.2; crates.io 404", publisher: "plugin-system + crates.io", pub_date: "2026-08-18", accessed: "2026-08-18", confidence: "high", class: "roadmap"}
- {claim: "大纲列举 anymap 为核心依赖，实现未使用；正文已改为可选。tracing 大纲/§7.8 写必须，实现为非默认 feature（feature-matrix 刻意偏离 #4）。", source: "设计方案大纲（v2）.md §7.1; feature-matrix.md; Cargo.toml", publisher: "plugin-system", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "doc-deviation"}
- {claim: "Error 伪代码含 TypeId 载荷与 BuildFailed(Box<dyn Error>)；实现为单元变体（api-freeze 已记录）。", source: "6. API 设计概览.md §6.2.5; docs/api-freeze.md", publisher: "plugin-system", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "api-deviation"}
- {claim: "bench 设计表含 isolate 10万次、1000 异步插件 start_async、emit_parallel 100 监听器；已交付 core_paths 覆盖 get/emit/start，未覆盖该全表。", source: "8. 测试策略.md §8.5; docs/testing.md; crates/plugctx/benches/core_paths.rs", publisher: "plugin-system", pub_date: "n/a", accessed: "2026-08-18", confidence: "medium", class: "test-gap"}

## Leads
- 无需要第二轮网页扇出的新实体。

## Not found
- 未发现未实现的核心同步 API（plugin/provide/get/on/emit/effect/start/dispose/isolate/interceptor）。
