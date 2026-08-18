//! trybuild：钉死常见宏误用（Story 5.1 / NFR4）。

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/enum_not_struct.rs");
    t.compile_fail("tests/ui/missing_on_build.rs");
    t.compile_fail("tests/ui/unknown_plugin_key.rs");
}
