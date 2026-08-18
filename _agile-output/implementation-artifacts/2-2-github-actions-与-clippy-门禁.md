---
id: "2.2"
status: done
---

# Story 2.2: GitHub Actions 与 clippy

Status: done

`.github/workflows/ci.yml` 在 ubuntu-latest 跑 `./scripts/ci-test.sh`。脚本增加 `cargo clippy -p plugctx -p plugctx-derive -- -D warnings`。禁止 `--all-features` 作为测试门槛。

### File List

- `.github/workflows/ci.yml`
- `scripts/ci-test.sh`
- `crates/plugctx/tests/acceptance_story_5_8.rs`
- `crates/plugctx/tests/acceptance_story_9_3.rs`
