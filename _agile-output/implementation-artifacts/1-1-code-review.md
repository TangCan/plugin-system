# Code Review — Story 1.1 Native dispose 物理卸载

Date: 2026-08-18
Outcome: **approve**（无阻塞问题）

## Findings

无 HIGH / MEDIUM。

### 已核对

- `ManuallyDrop<Library>` 已移除；`destroy_instance` 在持 `Mutex` 下 `vtable.destroy` 再 `take` Drop `Library`。
- `NativeInvoker::call` 持同一把锁，dispose 后返回可 `match` 的 `Error::NativeCall`，无 panic。
- 未新增 `reload()`；`default = []` 未改；`libloading` 仍仅 `dynamic-native`。
- 旧 `library_mapping_retained` 已删；`acceptance_story_4_2` 改为物理卸载 + stale invoker + Context dispose。
- WASM 路径未改。

### 备注（不阻塞）

- README / requirements 仍写 FR25，属 Story 1.3。
- 同路径覆盖 `.so` 再加载属 Story 1.2。
