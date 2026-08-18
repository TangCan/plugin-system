//! Acceptance tests for post-0.1.1 story 1.4 — docs.rs metadata 守门（ATDD / FR7）。

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

fn section_after_header<'a>(manifest: &'a str, header: &str) -> &'a str {
    let start = manifest
        .find(header)
        .unwrap_or_else(|| panic!("Cargo.toml 须含 {header}"));
    let after = &manifest[start + header.len()..];
    let after = after.strip_prefix('\r').unwrap_or(after);
    let after = after.strip_prefix('\n').unwrap_or(after);
    let end = after
        .find("\n[")
        .or_else(|| after.find("\r\n["))
        .unwrap_or(after.len());
    &after[..end]
}

fn normalize_toml_key_eq(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

/// AC#1: plugctx 轻量子集仍在，且不得 all-features=true。
#[test]
fn plugctx_docs_rs_subset_remains() {
    let manifest = read_required("crates/plugctx/Cargo.toml");
    let docs_rs = section_after_header(&manifest, "[package.metadata.docs.rs]");
    let normalized = normalize_toml_key_eq(docs_rs);
    assert!(
        !normalized.contains("all-features=true"),
        "plugctx docs.rs 不得 all-features=true（FR7）\n{docs_rs}"
    );
    for light in ["async", "parallel", "thread-safe", "tracing", "stages"] {
        assert!(
            docs_rs.contains(light),
            "plugctx docs.rs 子集须含 {light}（FR7）\n{docs_rs}"
        );
    }
    for heavy in ["dynamic-native", "dynamic-wasm", "dynamic-wasm-component"] {
        assert!(
            !docs_rs.contains(heavy),
            "plugctx docs.rs 不得含 {heavy}（FR7）\n{docs_rs}"
        );
    }
}

/// AC#2: derive 有 docs.rs 表且不得 all-features=true；publishing 写明两 crate 约定。
#[test]
fn derive_docs_rs_has_no_all_features() {
    let manifest = read_required("crates/plugctx-derive/Cargo.toml");
    assert!(
        manifest.contains("[package.metadata.docs.rs]"),
        "plugctx-derive 须有 [package.metadata.docs.rs]（FR7）"
    );
    let docs_rs = section_after_header(&manifest, "[package.metadata.docs.rs]");
    let normalized = normalize_toml_key_eq(docs_rs);
    assert!(
        !normalized.contains("all-features=true"),
        "plugctx-derive docs.rs 不得 all-features=true（FR7）\n{docs_rs}"
    );

    let publishing = read_required("docs/publishing.md");
    assert!(
        publishing.contains("plugctx-derive") && publishing.contains("docs.rs"),
        "publishing.md 须说明两公开 crate 的 docs.rs 约定（FR7）"
    );
    assert!(
        !publishing.contains("all-features = true")
            || publishing.to_ascii_lowercase().contains("不得")
            || publishing.contains("不用"),
        "publishing.md 不得把 all-features = true 写成推荐（FR7）"
    );
}
