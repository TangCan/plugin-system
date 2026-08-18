//! Acceptance tests for story 9.1 — 发布元数据与 `publish = false` 边界（ATDD / FR51）。

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

fn package_section(manifest: &str) -> &str {
    let rest = manifest
        .strip_prefix("[package]")
        .or_else(|| {
            manifest
                .find("[package]\n")
                .map(|i| &manifest[i + "[package]\n".len()..])
        })
        .unwrap_or(manifest);
    let end = rest.find("\n[").unwrap_or(rest.len());
    &rest[..end]
}

/// AC#1: plugctx / plugctx-derive 含 license、description；repository 或文档必填等价项；path 依赖带 version。
#[test]
fn public_crates_have_crates_io_metadata() {
    for rel in [
        "crates/plugctx/Cargo.toml",
        "crates/plugctx-derive/Cargo.toml",
    ] {
        let text = read_required(rel);
        let pkg = package_section(&text);
        assert!(
            pkg.contains("license") || text.contains("license.workspace"),
            "{rel} 须含 license（FR51）"
        );
        assert!(
            pkg.contains("description"),
            "{rel} 须含 description（FR51）"
        );
        assert!(
            pkg.contains("documentation") || pkg.contains("repository"),
            "{rel} 须含 documentation 或 repository（FR51）"
        );
        assert!(
            text.contains("repository.workspace")
                || pkg.contains("github.com/TangCan/plugin-system"),
            "{rel} 须继承或声明 repository = https://github.com/TangCan/plugin-system"
        );
    }

    let publishing = read_required("docs/publishing.md");
    assert!(
        !publishing.contains("暂无 origin") && !publishing.contains("暂无公开远端"),
        "docs/publishing.md 不得再写本地暂无 origin"
    );
    assert!(
        publishing.contains("https://github.com/TangCan/plugin-system"),
        "docs/publishing.md 须写出真实 GitHub repository URL"
    );
    assert!(
        publishing.contains("先发") && publishing.contains("plugctx-derive"),
        "清单须写明先发 plugctx 再发 plugctx-derive"
    );
    for needle in [
        "license",
        "description",
        "repository",
        "publish = false",
        "dry-run",
    ] {
        assert!(
            publishing.contains(needle),
            "docs/publishing.md 须列出必填等价项/边界「{needle}」（FR51）"
        );
    }

    let ws = read_required("Cargo.toml");
    assert!(
        ws.contains("plugctx = { path = \"crates/plugctx\", version")
            || ws.contains("version = \"0.1.0\"") && ws.contains("path = \"crates/plugctx\""),
        "workspace.dependencies.plugctx 须 path+version（FR51）"
    );
    assert!(
        ws.contains("plugin-api = { path = \"crates/plugin-api\", version")
            || (ws.contains("path = \"crates/plugin-api\"") && ws.contains("version = \"0.1.0\"")),
        "workspace.dependencies.plugin-api 须 path+version（FR51）"
    );

    // 默认可发布图不 path-依赖未上架 plugin-api（注释提及名称除外）
    let plugctx = read_required("crates/plugctx/Cargo.toml");
    let dep_lines: Vec<&str> = plugctx
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect();
    assert!(
        !dep_lines.iter().any(|l| l.contains("plugin-api")),
        "plugctx 不得再 path-依赖 plugin-api（以便 dry-run；ABI 在 c_abi）"
    );
}

/// AC#2: fixture / 内部成员 publish = false。
#[test]
fn fixtures_and_internal_crates_are_publish_false() {
    for rel in [
        "crates/plugin-api/Cargo.toml",
        "crates/plugin-host/Cargo.toml",
        "crates/plugins/hello/Cargo.toml",
        "crates/plugins/echo/Cargo.toml",
        "crates/plugins/wasm_echo/Cargo.toml",
        "guests/wit-sample/Cargo.toml",
        "examples/Cargo.toml",
    ] {
        let text = read_required(rel);
        assert!(
            text.contains("publish = false") || text.contains("publish=false"),
            "{rel} 须设置 publish = false（FR51）"
        );
    }
}

/// AC#3: cargo publish --dry-run 对公开包通过（workspace；主包可单独）。
#[test]
fn cargo_publish_dry_run_succeeds_for_public_crates() {
    let root = plugin_system_root();

    // 主包可单独 dry-run（不依赖其他本工作区可发布包）。
    let out = Command::new("cargo")
        .args(["publish", "-p", "plugctx", "--dry-run", "--allow-dirty"])
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cargo publish -p plugctx: {e}"));
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "cargo publish -p plugctx --dry-run 失败（FR51）\n{combined}"
    );
    assert!(
        combined.contains("aborting upload due to dry run") || combined.contains("Packaged"),
        "dry-run 输出应含 Packaging/aborting upload（plugctx）\n{combined}"
    );

    // derive 在 plugctx 未上架前不能单独 dry-run；用 --workspace 交叉验证（Cargo ≥1.90）。
    let out = Command::new("cargo")
        .args(["publish", "--workspace", "--dry-run", "--allow-dirty"])
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cargo publish --workspace: {e}"));
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "cargo publish --workspace --dry-run 失败（FR51）\n{combined}"
    );
    assert!(
        combined.contains("plugctx") && combined.contains("plugctx-derive"),
        "workspace dry-run 应打包 plugctx 与 plugctx-derive\n{combined}"
    );
    assert!(
        combined.contains("aborting upload due to dry run") || combined.contains("Packaged"),
        "workspace dry-run 输出应含 Packaging/aborting upload\n{combined}"
    );
}
