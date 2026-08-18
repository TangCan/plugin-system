//! Acceptance tests for story 7.3 — 池概念文档与 feature-matrix 更新（ATDD / FR46）。
//!
//! Red phase: fails until README / feature-matrix / requirements 写清逻辑 InstancePool
//! ≠ Wasmtime 资源 pooling，且不再声称「当前无 InstancePool」。

use std::path::PathBuf;

fn plugin_system_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(|p| p.parent()) // plugin-system/
        .expect("plugin-system root")
        .to_path_buf()
}

fn read_required(rel: &str) -> String {
    let path = plugin_system_root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "missing required doc artifact {rel} at {}: {e}",
            path.display()
        )
    })
}

/// 禁止再出现的「无池」否定句（FR46）。
fn assert_no_outdated_no_pool_claims(label: &str, text: &str) {
    let forbidden = [
        "当前无 `InstancePool` 实现",
        "当前无 InstancePool 实现",
        "**无** `InstancePool` 实现",
        "无 `InstancePool` 实现",
        "MVP 无池",
        "仅为 NFR10 意图说明，**无**",
    ];
    for needle in forbidden {
        assert!(
            !text.contains(needle),
            "{label} 不得再声称「{needle}」（FR46）"
        );
    }
}

/// AC#1: feature-matrix 写清两层概念与 API/feature 入口。
#[test]
fn feature_matrix_documents_two_layer_pool_concepts() {
    let text = read_required("docs/feature-matrix.md");
    assert!(
        text.contains("WasmInstancePool"),
        "feature-matrix 须提及 WasmInstancePool（逻辑池 API）"
    );
    assert!(
        text.contains("dynamic-wasm"),
        "feature-matrix 须标明 dynamic-wasm feature 入口"
    );
    assert!(
        text.contains("PoolingAllocationConfig")
            || (text.contains("Wasmtime") && text.contains("资源")),
        "feature-matrix 须区分 Wasmtime 运行时资源 pooling（PoolingAllocationConfig 或等价表述）"
    );
    assert!(
        text.contains("逻辑") || text.contains("应用层"),
        "feature-matrix 须说明逻辑/应用层 InstancePool"
    );
    assert!(
        text.contains("acceptance_story_7_1") || text.contains("7_1") || text.contains("7.1"),
        "feature-matrix 须指向池验收入口"
    );
    assert_no_outdated_no_pool_claims("docs/feature-matrix.md", &text);
}

/// AC#1: README 写清两层概念与入口。
#[test]
fn readme_documents_logical_pool_vs_wasmtime_pooling() {
    let text = read_required("README.md");
    assert!(
        text.contains("WasmInstancePool"),
        "README 须提及 WasmInstancePool"
    );
    assert!(
        text.contains("PoolingAllocationConfig")
            || (text.contains("Wasmtime")
                && (text.contains("资源") || text.contains("pooling") || text.contains("Pooling"))),
        "README 须对照 Wasmtime 运行时资源 pooling"
    );
    assert!(
        text.contains("dynamic-wasm"),
        "README 须标明 dynamic-wasm 入口"
    );
    assert!(
        !text.contains("见 Story 7.3") && !text.contains("全面文档清理见 Story 7.3"),
        "README 不得再把概念说明推迟到 Story 7.3"
    );
    assert_no_outdated_no_pool_claims("README.md", &text);
}

/// AC#2: requirements 扩展设计文档不得再声称无池。
#[test]
fn requirements_extension_doc_no_longer_claims_no_pool() {
    let text = read_required("docs/requirements/4. 扩展模块设计.md");
    assert_no_outdated_no_pool_claims("docs/requirements/4. 扩展模块设计.md", &text);
    assert!(
        text.contains("WasmInstancePool") || text.contains("InstancePool"),
        "扩展设计文档须指向已交付逻辑 InstancePool / WasmInstancePool"
    );
    assert!(
        text.contains("dynamic-wasm"),
        "扩展设计文档须标明 dynamic-wasm 可选 feature"
    );
}

/// 护栏：testing.md 登记 Story 7.3 门禁。
#[test]
fn testing_md_maps_story_7_3() {
    let text = read_required("docs/testing.md");
    assert!(
        text.contains("7.3") || text.contains("acceptance_story_7_3") || text.contains("FR46"),
        "testing.md 须映射 Story 7.3 / FR46 / acceptance_story_7_3"
    );
}

/// Automate：模块 rustdoc 保持两层概念，防止源码文档回退到「无池 / 见 Story」。
#[test]
fn dynamic_wasm_rustdoc_keeps_two_layer_pool_concepts() {
    let text = read_required("crates/plugctx/src/dynamic_wasm.rs");
    assert!(
        text.contains("WasmInstancePool"),
        "dynamic_wasm.rs 须文档化 WasmInstancePool"
    );
    assert!(
        text.contains("PoolingAllocationConfig"),
        "dynamic_wasm.rs 须对照 PoolingAllocationConfig"
    );
    assert!(
        text.contains("两层概念") || text.contains("逻辑"),
        "dynamic_wasm.rs 须保留逻辑池概念说明"
    );
    assert!(
        !text.contains("MVP 无池") && !text.contains("见 Story 7.3"),
        "dynamic_wasm.rs 不得再含过时「MVP 无池」或 Story 7.3 占位指针"
    );
}
