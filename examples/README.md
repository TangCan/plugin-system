# plugctx-examples

工作区级可运行演示（`publish = false`，不上架 crates.io）。

## 跑法

在 `plugin-system/` 下：

| 示例 | 命令 |
|------|------|
| derive | `cargo run -p plugctx-examples --example derive-plugin` |
| Component / WIT | `cargo run -p plugctx-examples --example component-add --features component` |
| Extism echo | `cargo run -p plugctx-examples --example wasm-echo --features wasm` |

默认同步内核演示仍在 **`crates/plugctx/examples/`**（`combo` / `async-start` / `stages-lifecycle`），见根 README「可运行示例」。

## 设计要点

- 重运行时（Extism / wasmtime）仅经本包 feature 开启，避免污染 `plugctx` 默认可发布图。
- 与验收测试互补：这里是给人看的短程序；ATDD 仍在 `crates/plugctx/tests/`。
