# Contributing to git-scanline

Thank you for your interest in contributing! These are the rules of the road.

---

## Code of Conduct

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

### Rust version

```bash
cd rust
cargo build            # dev build
cargo test             # run tests
cargo build --release  # release binary
```

### Node version

```bash
cd node
npm install
npm run build          # compile TypeScript
npm test               # run tests (requires a built output)
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
- For bugs, include the OS, Node/Rust version, and a minimal reproduction.
- For feature requests, describe the problem you are solving, not just the solution.

### Pull Requests

- **One concern per PR.** Bug fixes and new features should be separate.
- **Write or update tests** for any behavioral change.
- **Keep the diff small.** Large PRs are harder to review and slower to merge.
- **Both implementations.** If a change is applicable to both Node and Rust, implement
  it in both. If it only makes sense in one, that's fine — explain why in the PR.
- **No breaking changes to the CLI** without a major-version discussion in an issue first.

### Code Style

- **Rust**: `cargo fmt` and `cargo clippy` must pass with no warnings.
- **Node/TypeScript**: `tsc` must pass with `"strict": true`. No `any` types without
  a justifying comment.
- Match the existing style of the file you are editing.
- Comments should explain *why*, not *what*.

### Commit Messages

- Use the imperative mood: *"Add burst-detection threshold option"* not *"Added..."*
- Keep the subject line ≤ 72 characters.
- Reference related issues when appropriate: *"Fix #42 — …"*

---

## Architecture Overview

```
workspace/
├── node/               TypeScript implementation
│   ├── bin/            CLI entry point
│   ├── src/
│   │   ├── git/        Git data extraction (log, diff, blame)
│   │   ├── analyzers/  Per-signal analysis modules
│   │   ├── scoring/    Weighted score aggregation
│   │   ├── filters/    File noise filtering
│   │   └── reporters/  terminal / json / html output
│   └── tests/          Integration test suite
│
└── rust/               Rust implementation (mirrors node/)
    └── src/
        ├── git/
        ├── analyzers/
        ├── scoring.rs
        ├── filters.rs
        ├── reporters/
        └── main.rs     CLI entry point + tests
```

The two implementations are intentionally kept **feature-parallel** — any new signal,
filter, or output format should be added to both.

---

## Licensing

By contributing you agree that your contributions will be licensed under the
[MIT License](LICENSE) that covers this project.
