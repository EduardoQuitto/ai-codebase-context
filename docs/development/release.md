# Release process (skeleton — full automation in Fase 27–28)

1. Update `CHANGELOG.md` (`Unreleased` → version + date).
2. Bump workspace version (`[workspace.package]`) and crate versions together.
3. `cargo fmt && cargo clippy -- -D warnings && cargo test --workspace`.
4. Tag `vX.Y.Z`; `release.yml` builds `windows-x64 / linux-x64 / macos-arm64 / macos-x64 / linux-arm64`, runs tests, packages archives + `SHA256SUMS`, and publishes a GitHub Release.
5. Publish to crates.io (primary: GitHub Releases; package managers follow demand: Homebrew, winget, Scoop).

Rules: never publish a binary that skipped its platform tests; checksums mandatory; ARM64 Windows after x64 proves stable.
