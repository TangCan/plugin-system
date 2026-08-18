//! Acceptance tests for post-0.1.1 story 2.1 — native 卸载限度（ATDD / FR4 / NFR3 / NFR4）。

use std::path::PathBuf;

fn plugin_system_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("plugin-system root")
        .to_path_buf()
}

fn read_required(rel: &str) -> String {
    let path = plugin_system_root().join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing required artifact {rel} at {}: {e}", path.display()))
}

fn asserts_unload_limits(name: &str, text: &str) {
    assert!(
        (text.contains("FreeLibrary") || text.contains("dlclose"))
            && (text.contains("覆盖") || text.contains("解锁")),
        "{name} 须写明 FreeLibrary/dlclose 成功 ≠ 文件一定可覆盖（FR4）"
    );
    assert!(
        text.contains("macOS") && (text.contains("TLS") || text.contains("tls")),
        "{name} 须写明 macOS 上非平凡 TLS 可能永不卸载（FR4）"
    );
    assert!(
        (text.contains("残留") || text.contains("引用"))
            && (text.contains("函数指针") || text.contains("线程")),
        "{name} 须写明 sound 卸载要求无残留引用 / 出站函数指针 / 库内线程（FR4）"
    );
}

/// AC#1: 指南与 §4.3 写清三点限度且不互相矛盾。
#[test]
fn guide_and_requirements_document_native_unload_limits() {
    let guide = read_required("docs/guide.md");
    let req = read_required("docs/requirements/4. 扩展模块设计.md");
    asserts_unload_limits("docs/guide.md", &guide);
    asserts_unload_limits("docs/requirements/4. 扩展模块设计.md", &req);

    for (name, text) in [("guide", guide.as_str()), ("§4.3", req.as_str())] {
        assert!(
            text.contains("load") && text.contains("dispose"),
            "{name} 须保持 load → dispose → load（FR4）"
        );
        assert!(
            text.contains("无 `reload()`")
                || text.contains("不提供 `reload()`")
                || text.contains("无 reload()"),
            "{name} 须写明无 reload()（NFR3）"
        );
        assert!(
            !text.contains("fn reload") && !text.contains("Context::reload"),
            "{name} 不得把 reload 写成 API（NFR3）"
        );
    }
    assert!(
        !guide.to_ascii_lowercase().contains("hot-lib-reloader")
            || guide.contains("开发")
            || guide.contains("不是生产"),
        "若提及 hot-lib-reloader，不得写成生产公开 API（NFR3）"
    );
}

/// AC#2: WASM 卸载仍是实例 close/free，不是 native dlclose。
#[test]
fn wasm_unload_stays_instance_close() {
    let guide = read_required("docs/guide.md");
    assert!(
        guide.contains("close") && (guide.contains("free") || guide.contains("FR26")),
        "指南 WASM 卸载须为 close/free（NFR4）"
    );
    let req = read_required("docs/requirements/4. 扩展模块设计.md");
    assert!(
        req.contains("close") && req.contains("free"),
        "§4.3 WASM 卸载须为 close/free（NFR4）"
    );
}
