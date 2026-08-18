# Digest: 需求清单与范围边界 — r1-1

**Decision:** 抽出原始需求的必须能力、可选扩展与明确非目标。
**Accessed:** 2026-08-18

## Claims

- {claim: "核心目标覆盖：一切皆插件、TypeId DI、isolate、start/dispose+effect、轻量可扩展、async/parallel、动态库/WASM、过程宏与拦截器。", source: "docs/requirements/1. 引言.md §1.2", publisher: "plugin-system requirements", pub_date: "n/a (repo doc)", accessed: "2026-08-18", confidence: "high", class: "capability-inventory"}
- {claim: "引言将动态加载表述为「动态库 / WASM 插件，实现热插拔」。", source: "docs/requirements/1. 引言.md §1.2", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "capability-inventory"}
- {claim: "大纲核心组件：Context、Plugin、DI（含 trait 对象）、事件（含生命周期）、Effect、作用域精确卸载、isolate、start/dispose、拦截器。", source: "docs/requirements/设计方案大纲（v2）.md §3", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "capability-inventory"}
- {claim: "大纲扩展：async、parallel emit、dynamic loading、derive macros、trait 服务、thread-safe、custom stages。", source: "docs/requirements/设计方案大纲（v2）.md §4", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "capability-inventory"}
- {claim: "§2.2.2 曾画独立 crate plugctx-async / plugctx-dynamic；同时允许 Cargo feature。", source: "docs/requirements/2. 总体架构.md §2.2.2", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "architecture"}
- {claim: "emit_parallel 并发上限 buffer_unordered 明确「本版不做」。", source: "docs/requirements/4. 扩展模块设计.md §4.2.3", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "non-goal"}
- {claim: "native 卸载默认不以 dlclose 为正确性前提（逻辑卸载）。", source: "docs/requirements/4. 扩展模块设计.md §4.3.1", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "non-goal"}
- {claim: "阶段调度器（严格顺序）初期用事件即可，非必须。", source: "docs/requirements/4. 扩展模块设计.md §4.7.4", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "medium", class: "deferred"}
- {claim: "路线图阶段5要求：完整 API 文档、用户指南、CLI/Web/游戏示例、RC 社区反馈、0.2.0 正式发布。", source: "docs/requirements/9. 实施路线图.md §9.2 阶段5", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "roadmap"}
- {claim: "测试 CI 要求 fmt、clippy、test --all-features、test --no-default-features、bench --no-run。", source: "docs/requirements/8. 测试策略.md §8.9", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "high", class: "nfr"}
- {claim: "大纲 §7.1 列举核心依赖含 anymap、downcast-rs、tracing；正文 §7.8 称 tracing 必须、anymap 可选。", source: "设计方案大纲（v2）.md §7.1; 7. 技术选型与依赖.md §7.8", publisher: "plugin-system requirements", pub_date: "n/a", accessed: "2026-08-18", confidence: "medium", class: "dependency"}

## Leads
- 对照 Cargo features 与公开 API 是否覆盖大纲 §3–§4。
- 「热插拔」与「逻辑卸载 ≠ dlclose」是否冲突。
- 路线图阶段5与发布切片是否已关账。

## Not found
- 需求文档没有单独的「v1 明确砍掉」清单；非目标散落在各章「本版不做」与 NFR。
