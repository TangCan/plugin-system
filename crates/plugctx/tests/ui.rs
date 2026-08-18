//! trybuild UI harness — API 误用编译失败套件（Story 5.6 / FR39）。
//!
//! 至少 3 例 `compile_fail`；用例见 `tests/ui/*.rs`。

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
