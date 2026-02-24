```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   /\  /\  /\  /\  /\  /\  /\  /\  /\  /\  /\  /\  /\  /\  ║
║  (oo)(oo)(oo)(oo)(oo)(oo)(oo)(oo)(oo)(oo)(oo)(oo)(oo)(oo)   ║
║   \/  \/  \/  \/  \/  \/  \/  \/  \/  \/  \/  \/  \/  \/   ║
║                                                              ║
║              G I T - S C A N L I N E                        ║
║                                                              ║
║  ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~    ║
║  Surface the riskiest files in your git repositories.       ║
║  Signals: Churn · Bugs · Reverts · Coupling · Security      ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

**git-scanline** analyzes your local git history to surface **code hotspots** — files that
are frequently changed, correlated with bug-fix commits, reverted, and owned by a single
author. No instrumentation, no network calls. Just point it at any existing git repository
and run.

---

## Meet ZORP

```
      |\  /|
     (o \/ o)      ZORP — git-scanline's surf champion
      |====|
     /| || |\      "Catch the code wave before it crashes!"
    / |_||_| \
   /___________\
    ~~~~~~~~~~~
```

ZORP is the surfing space invader mascot of git-scanline. Like all good space invaders,
ZORP rides the git waves scanning for hotspots — files that churn, crash, and revert
while everyone else is catching clean breaks.

---

## Two implementations

| | [node/](node/) | [rust/](rust/) |
|---|---|---|
| Runtime | Node.js ≥ 18 | Native binary (no runtime) |
| Build | `npm run build` | `cargo build --release` |
| Binary | `node dist/bin/git-scanline.js` | `./git-scanline` |
| Drag-and-drop | No | Yes (Finder / macOS) |
| Desktop HTML default | No | Yes (`~/Desktop/`) |

---

## Rust version (recommended)

### Build

```bash
cd rust
cargo build --release
# Binary: rust/target/release/git-scanline
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

## Node version

### Build

```bash
cd node
npm install
npm run build
# Output: node/dist/bin/git-scanline.js
```

### Run

```bash
# Analyze a specific repo
node /path/to/git-scanline/node/dist/bin/git-scanline.js /path/to/repo

# JSON output
node /path/to/git-scanline/node/dist/bin/git-scanline.js /path/to/repo --format json

# HTML report
node /path/to/git-scanline/node/dist/bin/git-scanline.js /path/to/repo --format html --output report.html

# Multiple repos (pass parent folder)
node /path/to/git-scanline/node/dist/bin/git-scanline.js /path/to/projects-folder
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

Both implementations include a test suite. Tests that run against a real git repository
read the path from a `.env` file in the workspace root (never committed to git).

### Setup

1. Copy the example and set your repo path:

```
# .env  (workspace root — git-ignored)
TEST_REPO_PATH=/Users/yourname/Documents/Projects/my-project
```

2. Run the tests:

```bash
# Rust
cd rust && cargo test

# Node (build first, then test)
cd node && npm run build && npm test
```

Tests that require `TEST_REPO_PATH` are automatically skipped when it is not set.

---

## License

MIT — see [LICENSE](LICENSE).
