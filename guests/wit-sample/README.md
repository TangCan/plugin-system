# wit-sample-guest（FR50）

最小 **WIT world** + **`wasm32-wasip2`** 样例客人，供 `plugctx` 的
`dynamic-wasm-component` 宿主路径照抄接入。

## 制品

| 文件 | 说明 |
| --- | --- |
| `wit/world.wit` | 最小 world：`export add: func(a: s32, b: s32) -> s32` |
| `src/lib.rs` | wit-bindgen **0.60** 客人实现 |
| 检入 `.wasm` | `../../crates/plugctx/testdata/wit_sample_add.wasm`（CI 用，免装工具链） |

本 crate **不是** workspace member（manifest 含空 `[workspace]`），避免宿主
`cargo test --workspace` 被迫交叉编译。

## 构建（真实 wasip2）

```bash
rustup target add wasm32-wasip2
cd plugin-system
./scripts/build-wit-sample-guest.sh
```

等价手动命令：

```bash
cd plugin-system/guests/wit-sample
cargo build --target wasm32-wasip2 --release
cp target/wasm32-wasip2/release/wit_sample_guest.wasm \
  ../../crates/plugctx/testdata/wit_sample_add.wasm
```

## 宿主验收

```bash
cd plugin-system
cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_4
```

版本矩阵：[`docs/component-model-versions.md`](../../docs/component-model-versions.md)。
