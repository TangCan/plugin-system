//! Acceptance tests for story 6.2 — Error 枚举与核心 API 冻结对照表（ATDD / FR35）。
//!
//! Red phase: fails until `Error::ServiceNotFound` exists, core seven variants are
//! `thiserror`-backed (`Display` + `std::error::Error`), and the Chinese API freeze
//!对照表 document is present.

use plugctx::Error;
use std::error::Error as StdError;
use std::path::Path;

/// AC#1: 设计 §6.2.5 核心七变体均可构造且彼此可区分。
#[test]
fn core_error_variants_from_design_6_2_5_exist() {
    let variants: &[Error] = &[
        Error::MissingDependency,
        Error::CircularDependency,
        Error::AlreadyStarted,
        Error::AlreadyDisposed,
        Error::ServiceNotFound,
        Error::PluginAlreadyDisposed,
        Error::BuildFailed,
    ];
    assert_eq!(variants.len(), 7);
    // 两两不相等，防止别名合并。
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b, "variant {i} must differ from {j}");
            }
        }
    }
}

/// AC#1: thiserror（或等价）提供 Display + std::error::Error；文案非空中文可读。
#[test]
fn core_errors_implement_display_and_std_error() {
    let samples = [
        Error::MissingDependency,
        Error::CircularDependency,
        Error::AlreadyStarted,
        Error::AlreadyDisposed,
        Error::ServiceNotFound,
        Error::PluginAlreadyDisposed,
        Error::BuildFailed,
    ];
    for err in samples {
        let msg = err.to_string();
        assert!(!msg.is_empty(), "{err:?} Display must be non-empty");
        let _dyn: &dyn StdError = &err;
        assert!(err.source().is_none());
    }
    assert!(
        Error::ServiceNotFound.to_string().contains("服务"),
        "ServiceNotFound Display 须为中文可读"
    );
}

/// 护栏：plugctx 通过 workspace `thiserror` 实现 Error（FR35）。
#[test]
fn plugctx_depends_on_thiserror() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).expect("read Cargo.toml");
    assert!(
        text.contains("thiserror"),
        "plugctx Cargo.toml 须声明 thiserror（FR35）"
    );
}

/// AC#1: `ServiceNotFound` 可 matches!（集成方可依赖稳定变体名）。
#[test]
fn service_not_found_is_matchable() {
    let err = Error::ServiceNotFound;
    assert!(matches!(err, Error::ServiceNotFound));
    assert_eq!(err, Error::ServiceNotFound);
}

/// AC#2: 中文核心 API 冻结对照表文件存在且提及 §6.1 / Error。
#[test]
fn api_freeze_doc_exists_and_covers_core_table() {
    let doc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/api-freeze.md");
    let text = std::fs::read_to_string(&doc)
        .unwrap_or_else(|e| panic!("缺少 API 冻结对照表 {}: {e}", doc.display()));
    assert!(
        text.contains("6.1") || text.contains("§6.1"),
        "对照表须引用设计 §6.1"
    );
    assert!(
        text.contains("ServiceNotFound"),
        "对照表须覆盖 ServiceNotFound"
    );
    assert!(
        text.contains("已对齐") || text.contains("偏差"),
        "对照表须含对齐/偏差标注"
    );
}
