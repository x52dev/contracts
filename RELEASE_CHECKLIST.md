# Things to check/change before pushing a release

1. Run all the tests with `cargo test --all`
2. Run `cargo run --example library` to make sure the examples still run properly
3. Run `cargo fmt`
4. Change version numbers
    - inside `README.md` in the "Set-up" section
    - inside `Cargo.toml`
5. Write or finish entry in `CHANGELOG.md`
6. Run `cargo doc --open` and check if the documentation looks fine and is up to date
7. Run `cargo package` and check the output for any unwanted or missing files
8. Run `cargo publish` to upload to crates.io