//! Acceptance tests for post-0.1.1 story 1.2 — cargo-hack 互斥 feature（ATDD / FR3）。

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

/// AC#1: 回归入口用 cargo-hack 表达 thread-safe 互斥，并排除隐式 --all-features。
#[test]
fn cargo_hack_gate_expresses_thread_safe_exclusion() {
    let hack = read_required("scripts/ci-cargo-hack.sh");
    assert!(
        hack.contains("cargo hack") || hack.contains("cargo-hack"),
        "ci-cargo-hack.sh 须调用 cargo hack（FR3）\n{hack}"
    );
    assert!(
        hack.contains("--each-feature") || hack.contains("--feature-powerset"),
        "须使用 --each-feature 或 --feature-powerset（FR3）\n{hack}"
    );
    assert!(
        hack.contains("--mutually-exclusive-features") && hack.contains("thread-safe"),
        "须声明 --mutually-exclusive-features 覆盖 thread-safe（FR3）\n{hack}"
    );
    assert!(
        hack.contains("--exclude-all-features"),
        "须加 --exclude-all-features，避免 cargo-hack 隐式 --all-features（FR3）\n{hack}"
    );

    let ci_test = read_required("scripts/ci-test.sh");
    assert!(
        ci_test.contains("ci-cargo-hack.sh"),
        "ci-test.sh 须接入 ci-cargo-hack.sh（FR3）\n{ci_test}"
    );
}

/// AC#2: 不得削弱既有门；testing.md 写明本地命令；不得把 --all-features 当唯一 job。
#[test]
fn existing_gates_remain_and_docs_explain_hack() {
    let ci_test = read_required("scripts/ci-test.sh");
    assert!(
        ci_test.contains("cargo fmt") && ci_test.contains("clippy"),
        "须保留 fmt 与 clippy 门（FR3）\n{ci_test}"
    );
    assert!(
        ci_test.contains("-D warnings") || ci_test.contains("-Dwarnings"),
        "clippy 须保持 -D warnings（FR3）\n{ci_test}"
    );
    assert!(
        ci_test.contains("ci-extension-matrix.sh"),
        "须保留扩展矩阵（FR3）\n{ci_test}"
    );

    let testing = read_required("docs/testing.md");
    assert!(
        testing.contains("cargo hack")
            || testing.contains("cargo-hack")
            || testing.contains("ci-cargo-hack"),
        "docs/testing.md 须写明如何本地跑 cargo-hack（FR3）"
    );
    assert!(
        testing.contains("--exclude-all-features") && testing.contains("thread-safe"),
        "testing.md 须写明 --exclude-all-features 与 thread-safe 互斥（FR3）"
    );

    let workflow = read_required(".github/workflows/ci.yml");
    let has_all_features_test = workflow
        .lines()
        .any(|l| l.contains("cargo test") && l.contains("--all-features"));
    assert!(
        !has_all_features_test,
        "ci.yml 不得把 cargo test --all-features 当作测试命令（FR3）\n{workflow}"
    );
}

/// AC#3: 不增加 Windows GHA runner。
#[test]
fn ci_stays_ubuntu_hosted() {
    let ci = read_required(".github/workflows/ci.yml");
    let release = read_required(".github/workflows/release.yml");
    for (name, wf) in [("ci.yml", ci.as_str()), ("release.yml", release.as_str())] {
        let lower = wf.to_ascii_lowercase();
        assert!(
            !lower.contains("windows-latest") && !lower.contains("windows-2022"),
            "{name} 不得增加 Windows runner（AD-6）\n{wf}"
        );
        assert!(
            lower.contains("ubuntu-latest") || lower.contains("ubuntu-"),
            "{name} 须保持 ubuntu 托管（AD-6）\n{wf}"
        );
    }
}
