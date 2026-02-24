```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║  ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░  ║
║                                                              ║
║             G I T - S C A N L I N E                         ║
║                                                              ║
║  ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░  ║
║                                                              ║
║  Surface the riskiest files in your git repositories.       ║
║  Signals: Churn · Bugs · Reverts · Coupling · Security      ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

**git-scanline** (Node.js / TypeScript) analyzes your local git history to surface
**code hotspots** — files that change often, break often, and are owned by a single
author. No setup, no instrumentation, no network calls.

---

## Quick Start in VS Code

### 1. Open the terminal

| Method | Action |
|---|---|
| Keyboard | `` Ctrl+` `` on Windows/Linux, `` ⌃` `` on macOS |
| Menu | **Terminal → New Terminal** |
| Command Palette | `⌘⇧P` → **"Toggle Terminal"** |

### 2. Build (first time only)

```bash
cd /path/to/git-scanline/node
npm install
npm run build
```

### 3. Run against any git repo

```bash
# Analyze the current directory
node /path/to/git-scanline/node/dist/bin/git-scanline.js

# Analyze a specific repo or parent folder
node /path/to/git-scanline/node/dist/bin/git-scanline.js /path/to/repo

# Install globally so you can type `git-scanline` anywhere
npm install -g /path/to/git-scanline/node
git-scanline /path/to/repo
```

---

## CLI Options

```bash
# All history (default)
git-scanline /path/to/repo

# Limit history to the last 6 months
git-scanline /path/to/repo --since="6 months ago"

# Focus on a subdirectory
git-scanline /path/to/repo --path src/

# Show top 30 files in report (all files are always scanned)
git-scanline /path/to/repo --top 30

# Only show files correlated with bug-fix commits
git-scanline /path/to/repo --bugs-only

# JSON output (pipe-friendly)
git-scanline /path/to/repo --format json

# HTML report with charts
git-scanline /path/to/repo --format html --output report.html

# Multiple repos — pass a parent folder
git-scanline /path/to/projects-folder

# Tune scoring weights (auto-normalized, don't need to sum to 1)
git-scanline /path/to/repo --weight-churn 0.4 --weight-bugs 0.4
```

---

## How Scoring Works

Each file receives a **Hotspot Score (0–100)** computed from seven signals:

| Signal | Default Weight | What it measures |
|---|---|---|
| Churn (recency-weighted) | 27% | How often the file changes, recent changes weighted higher |
| Bug-fix correlation | 27% | Appearances in commits mentioning fix/bug/hotfix/regression |
| Revert frequency | 14% | Times the file appeared in a `Revert …` commit |
| Commit quality | 9% | WIP/low-quality commits and oversized commits |
| Burst patterns | 9% | Rapid successive commits (≥ 3 within 24 h) |
| Co-change coupling | 9% | Files that always change together (hidden dependencies) |
| Author silo | 5% | Concentration of commits from a single author |

Risk tiers:

| Tier | Score |
|---|---|
| 🔴 CRITICAL | 75–100 |
| 🟠 HIGH | 50–74 |
| 🟡 MEDIUM | 25–49 |
| 🟢 LOW | 0–24 |

---

## Testing

Tests are in `tests/integration.test.ts`. Tests that require a real git repository
read the path from a `.env` file in the workspace root (never committed to git).

### Setup

Create or edit the `.env` file in the **workspace root** (parent of `node/`):

```
# .env  (workspace root — git-ignored)
TEST_REPO_PATH=/Users/yourname/Documents/Projects/my-project
```

### Run

```bash
npm run build   # compile TypeScript first
npm test        # run tests (skips repo-dependent tests if path not set)
```

---

## Development

```bash
# Install dependencies
npm install

# Compile TypeScript → dist/
npm run build

# Run directly from compiled output
node dist/bin/git-scanline.js --help
```

### Project Structure

```
node/
├── bin/
│   └── git-scanline.ts     # CLI entry point
├── src/
│   ├── types.ts            # Shared TypeScript interfaces
│   ├── git/                # Git data extraction (log, numstat, blame)
│   ├── analyzers/          # Individual signal analyzers
│   ├── scoring/            # Weighted score aggregation
│   ├── filters/            # File noise filtering
│   └── reporters/          # terminal / json / html output
├── tests/
│   └── integration.test.ts # Test suite
├── dist/                   # Compiled JavaScript (git-ignored)
├── tsconfig.json
└── package.json
```

---

## Requirements

- Node.js ≥ 18
- Git installed and in your `PATH`

## License

MIT — see [LICENSE](../LICENSE).
