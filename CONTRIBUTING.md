# Contributing to git-scanline

Thank you for your interest in contributing! These are the rules of the road.

---

## Code of Conduct

This project follows [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

Be respectful, constructive, and inclusive. Harassment of any kind is not tolerated.
All interactions — issues, pull requests, discussions — must remain professional and welcoming.

---

## Getting Started

1. **Fork** the repository and **clone** your fork locally.
2. Create a **feature branch** from `main`:
   ```bash
   git checkout -b feat/my-improvement
   ```
3. Make your changes, add tests, and verify everything passes.
4. Open a **pull request** against `main` with a clear description of what changed and why.

---

## Development Setup

```bash
cargo build            # dev build
cargo test             # run tests
cargo build --release  # release binary
```

### Recommended validation before opening a PR

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### Running tests with a real repo

Both test suites can run integration tests against an actual git repository.
Set `TEST_REPO_PATH` in the workspace-root `.env` file (never committed to git):

```
# .env  (workspace root — git-ignored)
TEST_REPO_PATH=/Users/yourname/Documents/Projects/my-project
```

Tests that require a real repository are automatically skipped when the path is not set.

---

## Contribution Guidelines

### Issues

- Search existing issues before opening a new one.
- For bugs, include the OS, Rust version, and a minimal reproduction.
- For feature requests, describe the problem you are solving, not just the solution.
- For sensitive issues, avoid posting exploit details publicly; open a support request first.

### Pull Requests

- **One concern per PR.** Bug fixes and new features should be separate.
- **Write or update tests** for any behavioral change.
- **Keep the diff small.** Large PRs are harder to review and slower to merge.
- **No breaking changes to the CLI** without a major-version discussion in an issue first.

### Code Style

- **Rust**: `cargo fmt` and `cargo clippy` must pass with no warnings.
- Match the existing style of the file you are editing.
- Comments should explain *why*, not *what*.

### Commit Messages

- Use the imperative mood: *"Add burst-detection threshold option"* not *"Added..."*
- Keep the subject line ≤ 72 characters.
- Reference related issues when appropriate: *"Fix #42 — …"*

### Releases (maintainers)

For versioning and release steps, follow [RELEASING.md](RELEASING.md).

---

## Architecture Overview

```
workspace/
├── src/
│   ├── git/            Git data extraction
│   ├── analyzers/      Per-signal analysis modules
│   ├── scoring.rs      Weighted score aggregation
│   ├── filters.rs      File noise filtering
│   ├── reporters/      terminal / json / html output
│   └── main.rs         CLI entry point + tests
├── Cargo.toml
└── docs/
```

---

## Licensing

By contributing you agree that your contributions will be licensed under the
[MIT License](LICENSE) that covers this project.
