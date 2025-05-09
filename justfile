import '.toolchain/rust.just'

toolchain := ""

_list:
    @just --list

# Downgrade dependencies necessary to run MSRV checks/tests.
[private]
downgrade-for-msrv:
    # No downgrades currently needed.

# Check project
check:
    just --unstable --fmt --check
    fd --hidden --extension=md --extension=yml --exec-batch prettier --check
    fd --hidden --extension=toml --exec-batch taplo format --check
    fd --hidden --extension=toml --exec-batch taplo lint
    cargo +nightly fmt -- --check
    cargo shear

# Format project
fmt:
    just --unstable --fmt
    nixpkgs-fmt .
    fd --hidden --extension=md --extension=yml --exec-batch prettier --write
    fd --hidden --extension=toml --exec-batch taplo format
    cargo +nightly fmt

# Lint workspace with Clippy
clippy:
    cargo clippy --workspace --no-default-features
    cargo clippy --workspace
    # cargo clippy --workspace --all-features

# Test workspace without generating coverage files
[private]
test-no-coverage:
    cargo {{ toolchain }} nextest run --workspace --no-default-features
    # cargo {{ toolchain }} nextest run --workspace --all-features
    cargo {{ toolchain }} test --doc --workspace
    # cargo {{ toolchain }} test --doc --workspace --all-features
    RUSTDOCFLAGS="-D warnings" cargo {{ toolchain }} doc --workspace --no-deps --all-features

# Test workspace and generate coverage files
test: test-no-coverage
    # @just test-coverage-codecov
    # @just test-coverage-lcov

# Test workspace using MSRV
test-msrv: downgrade-for-msrv test

# Test workspace and generate Codecov coverage file
test-coverage-codecov:
    cargo {{ toolchain }} llvm-cov --workspace --all-features --codecov --output-path codecov.json

# Test workspace and generate LCOV coverage file
test-coverage-lcov:
    cargo {{ toolchain }} llvm-cov --workspace --all-features --lcov --output-path lcov.info

# Document workspace
doc *args:
    RUSTDOCFLAGS="--cfg=docsrs" cargo +nightly doc --no-deps --workspace --all-features {{ args }}

# Document workspace and watch for changes
doc-watch: (doc "--open")
    cargo watch -- just doc
