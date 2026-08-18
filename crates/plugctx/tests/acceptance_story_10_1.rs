//! Acceptance tests for post-0.1.1 story 1.1 — Trusted Publishing（ATDD / FR1 / FR2 / NFR8）。

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

/// AC#1: tag 触发的 release.yml 用 OIDC，先 plugctx 再 derive，不用长期 secret token。
#[test]
fn release_workflow_uses_oidc_trusted_publishing() {
    let wf = read_required(".github/workflows/release.yml");
    assert!(
        wf.contains("v*") || wf.contains("'v*'") || wf.contains("\"v*\""),
        "release.yml 须由 v* tag 触发（FR1）\n{wf}"
    );
    assert!(
        wf.contains("id-token: write") || wf.contains("id-token:write"),
        "须声明 permissions id-token: write（FR1）\n{wf}"
    );
    assert!(
        wf.contains("rust-lang/crates-io-auth-action"),
        "须使用 rust-lang/crates-io-auth-action 交换短时 token（FR1）\n{wf}"
    );
    assert!(
        !wf.contains("secrets.CARGO_REGISTRY_TOKEN"),
        "默认发版不得使用 secrets.CARGO_REGISTRY_TOKEN（FR1 / NFR8）\n{wf}"
    );

    let plugctx_at = wf
        .find("cargo publish -p plugctx")
        .or_else(|| wf.find("cargo publish --package plugctx"))
        .expect("须 cargo publish -p plugctx（FR1）");
    let derive_at = wf
        .find("cargo publish -p plugctx-derive")
        .or_else(|| wf.find("cargo publish --package plugctx-derive"))
        .expect("须 cargo publish -p plugctx-derive（FR1）");
    assert!(
        plugctx_at < derive_at,
        "须先发 plugctx 再发 plugctx-derive（FR1）\n{wf}"
    );
}

/// AC#2: publishing.md 是可执行 Trusted Publishing 步骤。
#[test]
fn publishing_docs_are_executable_trusted_publishing_steps() {
    let publishing = read_required("docs/publishing.md");
    for needle in ["release.yml", "Trusted Publisher", "30", "OIDC", "id-token"] {
        assert!(
            publishing.contains(needle),
            "docs/publishing.md 须含可执行要点「{needle}」（FR2）"
        );
    }
    assert!(
        publishing.contains("TangCan/plugin-system") || publishing.contains("plugin-system"),
        "须写明 GitHub 仓库名以便配置 Trusted Publisher（FR2）"
    );
    assert!(
        publishing.contains("并存") || publishing.contains("同时"),
        "须写明迁移期可与 API token 并存（FR2）"
    );
    assert!(
        publishing.contains("首次") && (publishing.contains("手工") || publishing.contains("手动")),
        "须写明新 crate 名首次仍须手工 token 发布（FR2）"
    );
}

/// AC#3: 不实际上架；dry-run 门禁仍绿。
#[test]
fn publish_dry_run_still_green_without_live_upload() {
    let root = plugin_system_root();
    let out = Command::new("bash")
        .arg("scripts/ci-publish-dry-run.sh")
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("failed to run ci-publish-dry-run.sh: {e}"));
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "ci-publish-dry-run.sh 须仍成功（本故事不实际上架；FR1 AC#3）\n{combined}"
    );
}
