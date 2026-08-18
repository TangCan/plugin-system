//! Acceptance tests for story 9.2 — 空 default 与 docs.rs 构建子集（ATDD / FR52 / NFR14）。

use std::path::PathBuf;
use std::process::Command;

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

fn features_section(manifest: &str) -> &str {
    section_after_header(manifest, "[features]")
}

fn docs_rs_section(manifest: &str) -> &str {
    section_after_header(manifest, "[package.metadata.docs.rs]")
}

fn normalize_toml_key_eq(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Parse `features = ["a", "b"]` from a docs.rs metadata section body.
fn parse_docs_rs_features(docs_rs: &str) -> Vec<String> {
    let line = docs_rs
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.starts_with('#') && (l.starts_with("features ") || l.starts_with("features=")))
        .unwrap_or_else(|| {
            panic!("[package.metadata.docs.rs] 须含 features = [...]（FR52）\n{docs_rs}")
        });
    let bracket = line
        .find('[')
        .and_then(|i| line[i + 1..].find(']').map(|j| (i + 1, i + 1 + j)))
        .unwrap_or_else(|| panic!("features 须为数组（FR52）: {line}"));
    let inner = &line[bracket.0..bracket.1];
    inner
        .split(',')
        .map(|p| p.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn strip_tree_prefix(line: &str) -> &str {
    line.trim_start_matches(|c: char| {
        c.is_whitespace()
            || c == '├'
            || c == '│'
            || c == '└'
            || c == '─'
            || c == '`'
            || c == '|'
            || c == '+'
            || c == '-'
    })
}

/// AC#1: 默认依赖图不含 extism / libloading / wasmtime。
#[test]
fn default_cargo_tree_excludes_heavy_runtimes() {
    let root = plugin_system_root();
    let out = Command::new("cargo")
        .args(["tree", "-p", "plugctx", "-e", "normal"])
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cargo tree: {e}"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "cargo tree -p plugctx -e normal 失败\n{stdout}\n{stderr}"
    );
    let lower = stdout.to_ascii_lowercase();
    for banned in ["extism", "libloading", "wasmtime"] {
        assert!(
            !lower.lines().any(|line| {
                // crate 名作为树节点：`extism v…` / `libloading v…` / `wasmtime v…`
                let trimmed = strip_tree_prefix(line);
                trimmed.starts_with(banned) || trimmed.starts_with(&format!("{banned} "))
            }),
            "默认 cargo tree 不得含 {banned}（FR52 / NFR14）\n{stdout}"
        );
    }
}

/// AC#3: 重能力仅经 `dep:` / 具名 feature。
#[test]
fn heavy_capabilities_use_dep_features() {
    let manifest = read_required("crates/plugctx/Cargo.toml");
    let features = features_section(&manifest);
    assert!(
        features
            .lines()
            .any(|l| l.trim_start().starts_with("default = []")
                || l.trim_start().starts_with("default=[]")),
        "default 须为空数组（FR52）\n{features}"
    );
    for (feat, dep) in [
        ("dynamic-native", "dep:libloading"),
        ("dynamic-wasm", "dep:extism"),
        ("dynamic-wasm-component", "dep:wasmtime"),
    ] {
        let line = features
            .lines()
            .find(|l| {
                l.trim_start().starts_with(&format!("{feat} "))
                    || l.trim_start().starts_with(&format!("{feat}="))
            })
            .unwrap_or_else(|| panic!("缺少 feature `{feat}`"));
        assert!(
            line.contains(dep),
            "`{feat}` 须经 `{dep}` 启用（FR52）: {line}"
        );
        assert!(
            !features.contains(&format!("default = [\"{feat}\"]"))
                && !features.lines().any(|l| {
                    let t = l.trim_start();
                    t.starts_with("default = [") && t.contains(feat)
                }),
            "`{feat}` 不得进入 default（FR52）"
        );
    }
}

/// AC#2: docs.rs metadata 指向可构建轻量子集，且按该子集 rustdoc 成功。
#[test]
fn docs_rs_metadata_subset_builds_rustdoc() {
    let manifest = read_required("crates/plugctx/Cargo.toml");
    let docs_rs = docs_rs_section(&manifest);
    assert!(
        docs_rs.contains("features"),
        "[package.metadata.docs.rs] 须声明 features（FR52）\n{docs_rs}"
    );
    let normalized = normalize_toml_key_eq(docs_rs);
    assert!(
        !normalized.contains("all-features=true"),
        "docs.rs 不得 all-features（重运行时系统依赖风险；FR52）\n{docs_rs}"
    );
    for heavy in ["dynamic-native", "dynamic-wasm", "dynamic-wasm-component"] {
        assert!(
            !docs_rs.contains(heavy),
            "docs.rs features 不得含 {heavy}（FR52）\n{docs_rs}"
        );
    }
    let parsed = parse_docs_rs_features(docs_rs);
    for light in ["async", "parallel", "thread-safe", "tracing", "stages"] {
        assert!(
            parsed.iter().any(|f| f == light),
            "docs.rs 轻量子集应含 {light}（FR52）\nparsed={parsed:?}\n{docs_rs}"
        );
    }

    let publishing = read_required("docs/publishing.md");
    for needle in [
        "docs.rs",
        "async",
        "dynamic-wasm",
        "dynamic-native",
        "dynamic-wasm-component",
        "extism",
        "wasmtime",
        "libloading",
    ] {
        assert!(
            publishing.contains(needle),
            "docs/publishing.md 须记载 docs.rs 子集/排除原因「{needle}」（FR52）"
        );
    }

    let root = plugin_system_root();
    let features = parsed.join(",");
    let out = Command::new("cargo")
        .args(["doc", "-p", "plugctx", "--no-deps", "--features", &features])
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cargo doc: {e}"));
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "按 docs.rs metadata 解析的 features（{features}）cargo doc 失败（FR52）\n{combined}"
    );
}
