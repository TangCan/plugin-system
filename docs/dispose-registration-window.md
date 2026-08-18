# 销毁窗口：`provide` / `on` / `effect` 何时允许

对齐 `Context::dispose` 实现与 Epic 1 回顾（cleanup 重入有意允许）。

## 时间线

```text
dispose() 调用
  → disposing=true（此时 is_disposed() == false）
  → [stages] emit(PreDisposeEvent)
  → emit(DisposeEvent)          ← 监听器仍可见旧 events
  → take 旧 services/events/effects/plugins…
  → disposed=true
  → 逆序执行 effect cleanup     ← cleanup 闭包内的写入落到「销毁后」表
  → 级联子上下文
```

| 阶段 | `is_disposed()` | `provide` / `on` / `on_async` | `effect(setup)` | `plugin` / `start` / `isolate` |
|------|-----------------|-------------------------------|-----------------|--------------------------------|
| 正常运行 | false | 允许 | setup 立即执行并登记 cleanup | 允许（契约内） |
| `DisposeEvent` / `PreDisposeEvent` 监听器内 | false（仍 `disposing`） | **允许**（写到尚未 take 的表；重入 `dispose` 幂等） | **允许**：setup 立即执行，cleanup **会**被纳入本轮 take（见 1.8 验收） | `plugin`/`start` 仍按既有检查；`isolate` 允许 |
| effect **cleanup** 执行期间 | **true** | **静默可写**：写入已清空后的新表，**不会**进入下一轮生命周期业务语义 | setup **仍立即执行**，但 **不登记** cleanup（已无销毁周期） | `plugin`/`start`/`isolate` → [`AlreadyDisposed`](../crates/plugctx/src/error.rs) |
| dispose 完全结束后 | true | 同上（静默写入无生命周期保证） | 同上 | 同上 |

## 调用方应遵守的契约

1. **安装类写操作**（`plugin` / `start` / `isolate` / `PluginHandle::dispose` 在 Context 已毁时）在
   `is_disposed()` 后返回明确错误 [`AlreadyDisposed`](../crates/plugctx/src/error.rs)。
2. **DisposeEvent 监听器内**可以 `provide` / `on` / `effect`——用于观测与临时登记；勿假设这些写入在
   dispose 完成后仍对「下一个」应用生命周期可见。
3. **cleanup 内**允许 `provide`/`on`（历史行为；用于测试与防御性重入），但框架**不保证**其参与
   后续 `start`/`emit` 业务；新代码应避免依赖该窗口。
4. 完全 `disposed` 后的 `effect(setup)`：setup 会跑，cleanup **丢弃**（无二次 dispose 可挂接）。

## 相关测试

- `acceptance_story_1_8`：DisposeEvent 内 `provide` / 重入 `on`/`emit`；DisposeEvent 内登记的 effect 仍执行
- `acceptance_story_1_6`：cleanup 逆序与 cancel
- `acceptance_story_1_7`：父已 dispose 后 `isolate` → `AlreadyDisposed`
