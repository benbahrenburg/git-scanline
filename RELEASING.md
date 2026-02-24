# Releasing git-scanline

This document describes the maintainer workflow for cutting a new release.

## Release cadence

Releases are cut as needed when meaningful changes are ready. Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Pre-release checklist

1. Ensure `main` is green in CI.
2. Pull latest changes:
   ```bash
   git checkout main
   git pull --ff-only origin main
   ```
3. Run local quality gates:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   cargo build --release
   ```
4. Confirm docs are current (`README.md`, `CONTRIBUTING.md`, `docs/`).

## Version bump

1. Update version in `Cargo.toml`:
   - `version = "x.y.z"`
2. Move relevant entries from `## [Unreleased]` in `CHANGELOG.md` into a new version section:
   - `## [x.y.z] — YYYY-MM-DD`
3. Add or update the release link reference at the bottom of `CHANGELOG.md`:
   - `[x.y.z]: https://github.com/benbahrenburg/git-scanline/releases/tag/vx.y.z`

## Create tag and GitHub release

1. Commit release metadata updates:
   ```bash
   git add Cargo.toml CHANGELOG.md
   git commit -m "release: vX.Y.Z"
   ```
2. Create and push tag:
   ```bash
   git tag vX.Y.Z
   git push origin main
   git push origin vX.Y.Z
   ```
3. Create a GitHub Release from tag `vX.Y.Z`:
   - Title: `vX.Y.Z`
   - Notes: summarize key items from `CHANGELOG.md`

## Post-release

1. Verify the release page is published correctly.
2. Verify users can build from source:
   ```bash
   git clone https://github.com/benbahrenburg/git-scanline.git
   cd git-scanline
   cargo build --release
   ```
3. Start the next cycle by ensuring `CHANGELOG.md` has an `## [Unreleased]` section ready.
