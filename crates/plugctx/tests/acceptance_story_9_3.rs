//! Acceptance tests for story 9.3 — dry-run CI 与 release 工作流文档（ATDD / FR53 / NFR13）。

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

/// AC#1: CI 脚本含 dry-run 且 `set -e`，失败会阻断；脚本实际执行成功。
#[test]
fn ci_publish_dry_run_gate_blocks_on_failure() {
    let script = read_required("scripts/ci-publish-dry-run.sh");
    assert!(
        script.contains("set -euo pipefail") || script.contains("set -e"),
        "ci-publish-dry-run.sh 须 set -e，使 dry-run 失败阻断流水线（FR53）"
    );
    assert!(
        script.contains("cargo publish") && script.contains("--dry-run"),
        "ci-publish-dry-run.sh 须执行 cargo publish --dry-run（FR53）"
    );
    assert!(
        script.contains("--workspace")
            || (script.contains("plugctx") && script.contains("plugctx-derive")),
        "dry-run 须覆盖 workspace 或双公开包（FR53）"
    );

    let ci_test = read_required("scripts/ci-test.sh");
    assert!(
        ci_test.contains("ci-publish-dry-run")
            || ci_test.contains("acceptance_story_9_3")
            || (ci_test.contains("cargo publish") && ci_test.contains("--dry-run")),
        "ci-test.sh 须接入 dry-run 门禁或 Story 9.3 验收（FR53）"
    );

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
        "ci-publish-dry-run.sh 须成功（失败应非零退出以阻断 CI；FR53）\n{combined}"
    );
    assert!(
        combined.contains("aborting upload due to dry run")
            || combined.contains("Packaged")
            || combined.contains("OK: ci-publish-dry-run"),
        "dry-run 输出应表明已演练打包（FR53）\n{combined}"
    );
}

/// AC#2: 发布文档含 token/trusted publishing、首次手工发布、速率限制（NFR13）。
#[test]
fn release_workflow_docs_cover_fr53_nfr13() {
    let publishing = read_required("docs/publishing.md");
    for needle in [
        "release-plz",
        "dry-run",
        "token",
        "trusted publishing",
        "手工",
        "速率",
        "yank",
    ] {
        assert!(
            publishing
                .to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
                || publishing.contains(needle),
            "docs/publishing.md 须记载发布工作流要点「{needle}」（FR53 / NFR13）"
        );
    }
    // 中文「手工发布」与 trusted publishing 约束
    assert!(
        publishing.contains("首次") && (publishing.contains("手工") || publishing.contains("手动")),
        "须写明新 crate 名首次须手工发布（FR53）"
    );

    let workflow = read_required(".github/workflows/ci.yml");
    assert!(
        workflow.contains("ci-test.sh") && workflow.contains("ubuntu"),
        "GitHub Actions 须在 ubuntu 上调用 ci-test.sh"
    );
}
