# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Changed

- Repository is now Rust-only; removed the Node.js implementation and related docs
- Rust project moved from `rust/` into the repository root (`Cargo.toml`, `src/`, `target/`)
- Documentation updated to reference a single architecture document: `docs/architecture.md`
- Added open-source governance and contributor workflows:
  - `CODE_OF_CONDUCT.md`, `SUPPORT.md`
  - `.github/CODEOWNERS`
  - PR/issue templates under `.github/`
  - CI workflow at `.github/workflows/ci.yml` (fmt, clippy, test, release build)
- Added maintainer release documentation: `RELEASING.md`
- Added repository policy/config files: `.editorconfig`, `.gitattributes`

## [1.0.0] — 2025-02-24

### Added

**Core analysis engine**
- 7 independent analyzers: churn, bug correlation, revert tracking, burst detection,
  co-change coupling, author concentration (silo), and commit quality
- Weighted hotspot scoring formula (0–100) with configurable `--weight-*` flags that
  auto-normalize to sum to 1.0 regardless of input values
- Four risk tiers: 🔴 CRITICAL (≥75), 🟠 HIGH (≥50), 🟡 MEDIUM (≥25), 🟢 LOW (<25)
- Security scanner: detects `.env`, key/cert files, and credential files present
  anywhere in git history (including deleted files)
- Recommendation engine surfacing actionable insights about author silos, burst
  patterns, revert-prone files, WIP commits, and oversized commits
- Co-change coupling detection (files that always change together)

**Dual implementation**
- Rust implementation (`rust/`) — primary, single binary, zero runtime dependencies
- Node.js implementation (`node/`) — identical feature set and CLI interface

**Output formats**
- Terminal: UTF8_FULL grid table with ANSI colors, churn bar charts, emoji tier indicators
- JSON: structured `Report` object for programmatic consumption
- HTML: self-contained interactive report with Chart.js bar chart and dark theme

**Developer experience**
- Interactive setup mode (runs when no arguments given): guided prompts for path, date
  range, format, output path, top-N, bugs-only mode, and custom weights
- "Analyze another repo?" loop in interactive mode
- ZORP the surfing space invader: animated ASCII art mascot while analysis runs;
  freezes as a header and reprints as a footer around the report
- Multi-repo support: pass a parent folder to discover and analyze all nested git repos
- `--bugs-only` flag to filter results to files with at least one bug-fix commit
- `--path` flag to scope analysis to a subdirectory
- `--since` flag accepting any git date format (`"6 months ago"`, `"2024-01-01"`, etc.)
- `--top N` to control result count (all files are always analyzed; only display is capped)

**Performance (Rust)**
- Single `git log --numstat` invocation produces both commit list and diff stats
  (eliminates second git subprocess, saves 0.5–2 s on large repos)
- All 7 analyzers run concurrently via `rayon::join` tree (3–5× speedup on
  multi-core hardware for large repositories)

**Testing**
- 44 unit tests across Rust analyzers, scorer, parser, and main module
- Integration tests (Rust + Node) that auto-skip when `TEST_REPO_PATH` is not configured
- `.env.example` with setup instructions for integration test configuration

**Documentation**
- Architecture docs with Mermaid diagrams for both implementations (`docs/`)
- ZORP mascot documentation (`ZORP.md`)

[1.0.0]: https://github.com/benbahrenburg/git-scanline/releases/tag/v1.0.0
