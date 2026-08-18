# ADR：ContextData 按职责拆分（service / event / effect / lifecycle）

- **状态**：Deferred（推迟至下一 major / 专项重构故事）
- **日期**：2026-08-17
- **关联**：Epic 1 回顾 action item（Architect）；`crates/plugctx/src/context.rs` 内 `ContextData`

## 背景

Epic 1–6 交付后，运行时状态几乎全部集中在 `Context` / `ContextData`：服务 DI、事件槽、
effect、插件 SlotMap / `PluginScope`、isolate 子树、拦截器，以及扩展路径（async 事件、
stages、dynamic-*）的条件字段。文件体量持续增长，后续改动的冲突面与认知负荷偏高。

回顾建议评估按 `service` / `event` / `effect` / `lifecycle` 拆分 `ContextData`，避免单文件
跨 epic 继续膨胀。

## 选项

1. **现在拆分**：抽 `ContextServices` / `ContextEvents` / `ContextEffects` / `ContextLifecycle`
   等子结构（或对应模块），`ContextData` 仅组合。
2. **推迟到下一 major**：维持现状，用模块边界与 rustdoc 约束，待有明确热改区域或 API 冻结
   窗口再拆。
3. **仅文件物理拆分、类型不拆**：`context/mod.rs` + `context/data.rs` 等，类型仍一体。

## 决策

**采用选项 2：defer until next major（或独立 refactor 故事，且不阻塞 0.2 扩展切片）。**

## 理由

- **行为正确性优先**：当前债务是可维护性，不是功能阻塞；Epic 回顾未将拆分列为发布门禁。
- **横切借用**：`dispose` / `dispose_plugin` / `start` 对多字段有交错 `borrow_mut` 与顺序契约；
  过早拆类型易引入临时 API 表面或 RefCell 死锁风险，收益主要是目录美学。
- **feature 门控字段**：`async_events`、stages 事件路径等与核心字段交织；拆分需同步设计
  cfg 边界，适合单独规划而非夹带在 retro 修修补补中。
- **下一 major 更合适**：若要动公开模块路径或内部不变量文档，放在 semver 窗口更清晰。

## 后果

- **短期**：继续在 `context.rs` 上演进；新增逻辑优先落在已有薄模块（`service` / `event` /
  `effect` / `plugin`）的类型与句柄层，避免再堆业务过程到无关文件。
- **触发重开条件**（任一即可排 refactor 故事）：
  - `context.rs` 再显著膨胀且多故事并行冲突频繁；
  - 需要独立测试/基准某一子系统而不拖动全 Context；
  - 准备 1.0 API 冻结前的内部结构整理。
- **非目标**：本 ADR 不要求立刻改代码；关闭 Architect 跟踪项即可。
