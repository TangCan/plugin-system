# Local one-shot: format, then the CI regression script
# (`ci-test.sh` already includes `cargo fmt --all -- --check`).
test:
    cargo fmt --all
    ./scripts/ci-test.sh
