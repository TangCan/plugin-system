---
status: done
---

# Stories 3.1–3.4

- 3.1 `examples/cli-hotplug.rs`（`native` feature）
- 3.2 `examples/web-service.rs`（`tiny_http` 0.12，仅示例外包）
- 3.3 `examples/game-loop.rs`
- 3.4 `docs/guide.md`；README 在 C ABI 脚手架前链入

## Code review

approve。tiny_http 未进入 plugctx。CLI 走物理卸载。Web 默认 Context 非 Send，自检把 client 放进线程。
