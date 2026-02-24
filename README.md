```
   ·   ★   ·   ✦   ·   ·   ★   ·   ✦   ·   ·   ★   ·   ✦   ·   ·   ★   ·
 ✦   ·   ★   ·   ✦   ·   ★   ·   ✦   ·   ★   ·   ✦   ·   ★   ·   ✦   ·   ★
   ·   ✦   ·   ★   ·   ·   ✦   ·   ★   ·   ·   ✦   ·   ★   ·   ·   ✦   ·

            |\  /\
           (o \/ o)          G I T - S C A N L I N E
            |====|           ─────────────────────────────────────────
           /| || |\          Surface the riskiest files
          / |_||_| \         in your git repositories.
         /___________\       Churn · Bugs · Reverts · Coupling · Security
  ══════════════════════════════════════════════════════════════════════════
  ~^~^~~^~^~~^~^~~~^~^~~^~^~~^~^~~~^~^~~^~^~~^~^~~~^~^~~^~^~~^~^~~~^~^~~
  ^^^~~^~^~~^~^~~~^~^~~^~^~~^~^~~~^~^~~^~^~~^~^~~~^~^~~^~^~~^~^~~~^~^~~^~
```

**git-scanline** analyzes your local git history to surface **code hotspots** — files that
are frequently changed, correlated with bug-fix commits, reverted, and owned by a single
author. No instrumentation, no network calls. Just point it at any existing git repository
and run.

---

## Project layout

This repository now contains a single Rust implementation at the workspace root.

---

## Build

```bash
cargo build --release
# Binary: target/release/git-scanline
```

### Build multiple targets

```bash
# Uses host-safe defaults and auto-installs missing Rust targets
./scripts/build-targets.sh

# Or provide explicit targets
./scripts/build-targets.sh x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

# Use broad cross-platform matrix defaults
./scripts/build-targets.sh --matrix
```

### Run

```bash
# Interactive mode (no arguments — works great as a double-click target)
./git-scanline

# Analyze a specific repo or parent folder
./git-scanline /path/to/repo
./git-scanline /path/to/projects-folder   # discovers all nested git repos

# Drag a folder onto the executable in Finder — it passes the path automatically

# Options
./git-scanline --help
./git-scanline /path/to/repo --since="6 months ago" --top 20
./git-scanline /path/to/repo --format json --output report.json
./git-scanline /path/to/repo --format html                  # saves to ~/Desktop/
./git-scanline /path/to/repo --format html --output /tmp/report.html
./git-scanline /path/to/repo --bugs-only --top 10
```

Interactive mode now follows this order:

1. Analyze and display the report in terminal.
2. Ask whether to export the report to a file (`Output this report to file? [no]:`).
3. If yes, ask for output path and export as HTML or JSON (based on file extension).
4. Ask whether to analyze another repo (`Analyze another repo? [no]:`).

### Flags

| Flag | Default | Description |
|---|---|---|
| `PATH` | current dir | Git repo or parent folder (positional) |
| `--since` | *(all history)* | Limit analysis, e.g. `"6 months ago"` or `"2024-01-01"` |
| `--top N` | `20` | Files to show in report (all files are always scanned) |
| `--format` | `terminal` | Output format: `terminal`, `json`, `html` |
| `--output PATH` | Desktop (html) | Output file path |
| `--path SUBDIR` | *(all)* | Restrict to a subdirectory |
| `--bugs-only` | off | Only show files with bug-fix correlation |
| `--no-interactive` | off | Skip interactive prompts |

---

## Example output

> The real terminal is colorized: scores are red/yellow/green by severity,
> churn bars are red, coupling warnings are yellow, and tier badges use their
> emoji colors. Shown here without ANSI codes.

```
  ✓ [1/5] Parsing commit log + diff stats       318ms
  ✓ [2/5] Scanning for security risks           2ms
  ✓ [3/5] Filtering files                       9ms
  ✓ [4/5] All 7 analyzers (parallel)            1.4s
  ✓ [5/5] Scoring hotspots                      4ms
✔ [my-app] 4,821 commits, 67 files — ⏱ 2.1s

🔐 Security Risks — sensitive files found in git history:
   Even deleted files remain accessible via git history!

   ⚠  config/database.yml  [credentials]  3 commits (first: 2021-03-12, last: 2022-08-05)

🔥 git-scanline — since "6 months ago" (4,821 commits, 67 files)

╔══════╦════════════════════════════════════════════════╦═══════╦═══════╦══════╦═════════╦═════╦═════════════╗
║ RANK ║ FILE                                           ║ SCORE ║ CHURN ║ BUGS ║ REVERTS ║ WIP ║ RISK        ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║    1 ║ src/api/payments/processor.ts                  ║    94 ║ ████▌ ║   31 ║       4 ║   8 ║ 🔴 CRITICAL ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║    2 ║ src/auth/session-manager.ts                    ║    87 ║ ███▊  ║   22 ║       3 ║   5 ║ 🔴 CRITICAL ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║    3 ║ src/core/event-bus.ts                          ║    71 ║ ███▎  ║   14 ║       2 ║   3 ║ 🟠 HIGH     ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║    4 ║ src/db/migrations/runner.ts                    ║    58 ║ ██▍   ║    8 ║       1 ║   2 ║ 🟠 HIGH     ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║    5 ║ src/api/orders/cart.ts                         ║    52 ║ ██▊   ║    9 ║       0 ║   4 ║ 🟠 HIGH     ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║    6 ║ src/middleware/rate-limiter.ts                 ║    41 ║ █▌    ║    4 ║       0 ║   1 ║ 🟡 MEDIUM   ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║    7 ║ src/utils/date-helpers.ts                      ║    33 ║ ██    ║    5 ║       0 ║   0 ║ 🟡 MEDIUM   ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║    8 ║ src/config/feature-flags.ts                    ║    28 ║ █▎    ║    2 ║       0 ║   2 ║ 🟡 MEDIUM   ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║    9 ║ src/api/webhooks/stripe.ts                     ║    19 ║ ▊     ║    1 ║       0 ║   0 ║ 🟢 LOW      ║
╠══════╬════════════════════════════════════════════════╬═══════╬═══════╬══════╬═════════╬═════╬═════════════╣
║   10 ║ src/ui/components/checkout.tsx                 ║    14 ║ ▌     ║    2 ║       0 ║   0 ║ 🟢 LOW      ║
╚══════╩════════════════════════════════════════════════╩═══════╩═══════╩══════╩═════════╩═════╩═════════════╝

⚠️  Co-change coupling detected:
    src/auth/session-manager.ts ↔ src/core/event-bus.ts (changed together 28x, strength 84%)
    src/api/payments/processor.ts ↔ src/db/migrations/runner.ts (changed together 19x, strength 72%)
    src/api/orders/cart.ts ↔ src/ui/components/checkout.tsx (changed together 14x, strength 61%)

💡 Recommendations:
    • processor.ts has been reverted 4 times — consider adding tests or stricter review
    • session-manager.ts has 91% single-author commits — consider a knowledge-transfer session
    • processor.ts appears in 8 WIP/low-quality commits — this area needs careful review
    • event-bus.ts shows burst patterns: 5 rapid-commit windows detected
```

---

## Scoring signals

| Signal | Default Weight | What it measures |
|---|---|---|
| Churn | 27% | Commit frequency, recency-weighted (recent changes score higher) |
| Bug-fix correlation | 27% | Appearances in commits mentioning fix/bug/hotfix/regression |
| Revert frequency | 14% | How often changes to the file were reverted |
| Commit quality | 9% | WIP commits, very short messages, and oversized commits |
| Burst patterns | 9% | Rapid successive commits — crisis / patch-on-patch behavior |
| Co-change coupling | 9% | Files that always change together (hidden dependencies) |
| Author silo | 5% | Single-author concentration (bus factor risk) |

## Risk tiers

| Tier | Score |
|---|---|
| 🔴 CRITICAL | ≥ 75 |
| 🟠 HIGH | ≥ 50 |
| 🟡 MEDIUM | ≥ 25 |
| 🟢 LOW | < 25 |

---

## Testing

The Rust suite includes unit tests that run without any configuration,
plus integration tests that run against a real git repository on your machine.

### `.env` setup (required for integration tests)

Integration tests read `TEST_REPO_PATH` from a `.env` file at the **workspace root**
(the same folder as this README). The file is git-ignored — it never leaves your machine.

**1. Create the file:**

```bash
# From the workspace root
cp .env.example .env
```

**2. Edit `.env` and set an absolute path to any local git repository:**

```bash
# .env  (workspace root — git-ignored, never committed)
TEST_REPO_PATH=/Users/yourname/path/to/any-git-repo
```

`TEST_REPO_PATH` must be an absolute path to a directory that contains a `.git` folder.
Any repository works — the tests only read history and never write anything.

> **No `.env`?** All 44 Rust unit tests still run and pass. Only the two real-repo
> integration tests are skipped with a logged notice.

### Run the tests

```bash
# Rust — all tests (integration tests run if TEST_REPO_PATH is set)
cargo test

# Rust — unit tests only
cargo test --lib

# Rust — verbose output showing skipped tests
cargo test -- --nocapture
```

### What each test covers

| Test | Requires `.env` | What it verifies |
|---|---|---|
| `test_parse_log_real_repo` | Yes | `parse_log` returns commits with a valid hash and author |
| `test_full_pipeline_scores_in_range` | Yes | End-to-end hotspot scores are in the 0–100 range |
| All other tests (42) | No | Individual analyzers, scoring, path helpers, file filters |

---

## Community & Security

- Code of Conduct: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
- Contributing guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Support channels: [SUPPORT.md](SUPPORT.md)
- Release process (maintainers): [RELEASING.md](RELEASING.md)

## Repository Policies

- Editor and newline rules: [.editorconfig](.editorconfig)
- Git text/binary attributes: [.gitattributes](.gitattributes)

---

## CI

GitHub Actions run the following checks on pushes and pull requests to `main`:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo build --release`

Workflow file: `.github/workflows/ci.yml`

---

## License

MIT — see [LICENSE](LICENSE).
